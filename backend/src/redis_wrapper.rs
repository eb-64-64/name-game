use std::{
    error::Error,
    marker::{Send, Sync},
    str::FromStr,
    sync::LazyLock,
};

use futures::{Stream, StreamExt};
use miette::{Context, IntoDiagnostic, bail};
use rand::{Rng, rng};
use redis::{
    AsyncConnectionConfig, AsyncTypedCommands, Client, PushKind, Script, Value,
    aio::MultiplexedConnection,
};
use secrecy::{ExposeSecret, SecretString};
use tokio::sync::{broadcast::Receiver as BroadcastReceiver, watch::Receiver as WatchReceiver};
use tokio_stream::wrappers::{BroadcastStream, WatchStream};
use tracing::warn;
use uuid::Uuid;

use crate::{Epoch, GameState};

const NAMES_KEY: &str = "names";
const GUESSES_KEY: &str = "guesses";
const STATE_KEY: &str = "gameState";
const EPOCH_KEY: &str = "epoch";

const NUM_NAMES_CHANNEL: &str = "numNames";
const GUESS_CHANNEL: &str = "guess";
const UNGUESS_CHANNEL: &str = "unguess";
const STATE_SUBMITTING_CHANNEL: &str = "stateSubmitting";
const STATE_PLAYING_CHANNEL: &str = "statePlaying";

static ADD_NAME_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        &r#"
server.call("HSET", KEYS[1], ARGV[2], ARGV[1])
local num_names = server.call("HLEN", KEYS[1])
server.call("PUBLISH", "NUM_NAMES_CHANNEL", num_names)
return ARGV[2]
"#
        .trim()
        .replace("NUM_NAMES_CHANNEL", NUM_NAMES_CHANNEL),
    )
});

static REMOVE_NAME_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        &r#"
server.call("HDEL", KEYS[1], ARGV[1])
local num_names = server.call("HLEN", KEYS[1])
server.call("PUBLISH", "NUM_NAMES_CHANNEL", num_names)
    "#
        .trim()
        .replace("NUM_NAMES_CHANNEL", NUM_NAMES_CHANNEL),
    )
});

static GUESS_NAME_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        &r#"
server.call("SETBIT", KEYS[1], ARGV[1], 1)
server.call("PUBLISH", "GUESS_CHANNEL", ARGV[1])
"#
        .trim()
        .replace("GUESS_CHANNEL", GUESS_CHANNEL),
    )
});

static UNGUESS_NAME_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        &r#"
server.call("SETBIT", KEYS[1], ARGV[1], 0)
server.call("PUBLISH", "UNGUESS_CHANNEL", ARGV[1])
"#
        .trim()
        .replace("UNGUESS_CHANNEL", UNGUESS_CHANNEL),
    )
});

static CHANGE_STATE_TO_SUBMITTING: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        &r#"
-- clear names and guesses
server.call("DEL", KEYS[3])
server.call("DEL", KEYS[4])

-- set state
server.call("SET", KEYS[1], "SUBMITTING_STATE")
local epoch = server.call("INCR", KEYS[2])

-- publish state change
server.call("PUBLISH", "STATE_SUBMITTING_CHANNEL", epoch)
"#
        .trim()
        .replace("STATE_SUBMITTING_CHANNEL", STATE_SUBMITTING_CHANNEL)
        .replace("SUBMITTING_STATE", GameState::SUBMITTING),
    )
});

static CHANGE_STATE_TO_PLAYING: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        &r#"
-- shuffle names
math.randomseed(ARGV[1])
local names = server.call("HVALS", KEYS[2])
for i = 1, #names - 1 do
    local j = math.random(i, #names)
    names[i], names[j] = names[j], names[i]
end
server.call("DEL", KEYS[2])
server.call("RPUSH", KEYS[2], unpack(names))

-- set state
server.call("SET", KEYS[1], "PLAYING_STATE")

-- publish state change
server.call("PUBLISH", "STATE_PLAYING_CHANNEL", "")
"#
        .trim()
        .replace("STATE_PLAYING_CHANNEL", STATE_PLAYING_CHANNEL)
        .replace("PLAYING_STATE", GameState::PLAYING),
    )
});

#[derive(Debug)]
pub struct RedisWrapper {
    _client: Client,
    conn: MultiplexedConnection,
    num_names_receiver: WatchReceiver<usize>,
    guess_receiver: BroadcastReceiver<usize>,
    unguess_receiver: BroadcastReceiver<usize>,
    state_change_receiver: WatchReceiver<GameState>,
}

impl RedisWrapper {
    pub async fn new(url: SecretString) -> miette::Result<Self> {
        let client = Client::open(url.expose_secret())
            .into_diagnostic()
            .wrap_err("create redis client")?;

        let (sender, mut receiver) = tokio::sync::broadcast::channel::<redis::PushInfo>(512);
        let config = AsyncConnectionConfig::new().set_push_sender(sender);
        let mut conn = client
            .get_multiplexed_async_connection_with_config(&config)
            .await
            .into_diagnostic()
            .wrap_err("establish connection with redis")?;
        conn.subscribe(&[
            NUM_NAMES_CHANNEL,
            GUESS_CHANNEL,
            UNGUESS_CHANNEL,
            STATE_SUBMITTING_CHANNEL,
            STATE_PLAYING_CHANNEL,
        ])
        .await
        .into_diagnostic()?;

        let num_names = conn
            .llen(NAMES_KEY)
            .await
            .into_diagnostic()
            .wrap_err("get initial name count")?;
        let (num_names_sender, num_names_receiver) = tokio::sync::watch::channel(num_names);

        let (guess_sender, guess_receiver) = tokio::sync::broadcast::channel(128);
        let (unguess_sender, unguess_receiver) = tokio::sync::broadcast::channel(128);

        let game_state = match redis::pipe()
            .get(STATE_KEY)
            .get(EPOCH_KEY)
            .query_async::<(Option<String>, Option<u32>)>(&mut conn)
            .await
        {
            Ok((Some(state), epoch)) if state == GameState::SUBMITTING => {
                GameState::Submitting(Epoch(epoch.unwrap_or(0)))
            }
            Ok((Some(state), _)) if state == GameState::PLAYING => GameState::Playing,
            Ok((Some(state), _)) => {
                bail!("unknown state while getting initial game state: {state}")
            }
            Ok((None, epoch)) => GameState::Submitting(Epoch(epoch.unwrap_or(0))),
            Err(err) => {
                return Err(err)
                    .into_diagnostic()
                    .wrap_err("get initial game state");
            }
        };
        let (state_change_sender, state_change_receiver) = tokio::sync::watch::channel(game_state);

        tokio::spawn(async move {
            loop {
                let push = receiver.recv().await.unwrap();
                let PushKind::Message = push.kind else {
                    continue;
                };
                let Ok(channel) = push.data[0].try_as_str() else {
                    continue;
                };
                match channel {
                    NUM_NAMES_CHANNEL => {
                        let Ok(num_names) = push.data[1].try_from_str::<usize>() else {
                            warn!(
                                "got non-numeric number of names on channel: {:?}",
                                push.data[1]
                            );
                            continue;
                        };
                        num_names_sender.send_replace(num_names);
                    }
                    GUESS_CHANNEL => {
                        let Ok(index) = push.data[1].try_from_str::<usize>() else {
                            warn!("got non-numeric guess index on channel: {:?}", push.data[1]);
                            continue;
                        };
                        guess_sender.send(index).expect(
                            "there should be at least one receiver listening to the guess channel",
                        );
                    }
                    UNGUESS_CHANNEL => {
                        let Ok(index) = push.data[1].try_from_str::<usize>() else {
                            warn!(
                                "got non-numeric unguess index on channel: {:?}",
                                push.data[1]
                            );
                            continue;
                        };
                        unguess_sender.send(index).expect(
                            "there should be at least one receiver listening to the unguess channel",
                        );
                    }
                    STATE_SUBMITTING_CHANNEL => {
                        let Ok(epoch) = push.data[1].try_from_str::<u32>() else {
                            warn!(
                                "got non-integer on submitting state change channel: {:?}",
                                push.data[1]
                            );
                            continue;
                        };
                        // update number of names without sending a
                        // notification (no notification is needed, as any
                        // currently connected displays will get a 0 num names
                        // packet when the state change is observed)
                        num_names_sender.send_if_modified(|num| {
                            *num = 0;
                            false
                        });
                        state_change_sender.send_replace(GameState::Submitting(Epoch(epoch)));
                    }
                    STATE_PLAYING_CHANNEL => {
                        state_change_sender.send_replace(GameState::Playing);
                    }
                    _ => {}
                }
            }
        });

        Ok(Self {
            _client: client,
            conn,
            num_names_receiver,
            guess_receiver,
            unguess_receiver,
            state_change_receiver,
        })
    }

    pub fn name_count(&self) -> usize {
        *self.num_names_receiver.borrow()
    }

    pub fn name_count_stream(&self) -> impl Stream<Item = usize> {
        let mut receiver = self.num_names_receiver.clone();
        receiver.mark_unchanged();
        WatchStream::from_changes(receiver)
    }

    pub async fn add_name(&self, name: &str) -> miette::Result<Uuid> {
        ADD_NAME_SCRIPT
            .key(NAMES_KEY)
            .arg(name)
            .arg(Uuid::new_v4())
            .invoke_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("add name")
    }

    pub async fn remove_name(&self, id: &Uuid) -> miette::Result<()> {
        REMOVE_NAME_SCRIPT
            .key(NAMES_KEY)
            .arg(id)
            .invoke_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("remove name")
    }

    pub async fn names_and_guesses(&self) -> miette::Result<(Vec<String>, Vec<u8>)> {
        let mut pipe = redis::pipe();
        match self.state() {
            GameState::Submitting(_) => pipe.hvals(NAMES_KEY),
            GameState::Playing => pipe.lrange(NAMES_KEY, 0, -1),
        };
        let (names, mut guesses): (Vec<String>, Vec<u8>) = pipe
            .get(GUESSES_KEY)
            .query_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("get names and guesses")?;

        // if there are more guesses than names, just get rid of the extra ones
        // at the back
        guesses.truncate(names.len().div_ceil(8));

        Ok((names, guesses))
    }

    pub async fn guess_name(&self, index: usize) -> miette::Result<()> {
        GUESS_NAME_SCRIPT
            .key(GUESSES_KEY)
            .arg(index)
            .invoke_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("guess name")
    }

    pub async fn unguess_name(&self, index: usize) -> miette::Result<()> {
        UNGUESS_NAME_SCRIPT
            .key(GUESSES_KEY)
            .arg(index)
            .invoke_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("unguess name")
    }

    pub fn guess_stream(&self) -> impl Stream<Item = usize> {
        BroadcastStream::new(self.guess_receiver.resubscribe())
            .map(|res| res.expect("the guess channel's sender should not have been dropped"))
    }

    pub fn unguess_stream(&self) -> impl Stream<Item = usize> {
        BroadcastStream::new(self.unguess_receiver.resubscribe())
            .map(|res| res.expect("the unguess channel's sender should not have been dropped"))
    }

    pub fn state(&self) -> GameState {
        *self.state_change_receiver.borrow()
    }

    pub fn state_change_stream(&self) -> impl Stream<Item = GameState> {
        let mut receiver = self.state_change_receiver.clone();
        receiver.mark_unchanged();
        WatchStream::from_changes(receiver)
    }

    pub async fn change_state_to_submitting(&self) -> miette::Result<()> {
        CHANGE_STATE_TO_SUBMITTING
            .key(STATE_KEY)
            .key(EPOCH_KEY)
            .key(NAMES_KEY)
            .key(GUESSES_KEY)
            .invoke_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("set state to submitting")
    }

    pub async fn change_state_to_playing(&self) -> miette::Result<()> {
        let seed = rng().random::<u32>();
        CHANGE_STATE_TO_PLAYING
            .key(STATE_KEY)
            .key(NAMES_KEY)
            .arg(seed)
            .invoke_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("set state to playing")
    }
}

trait ValueExt {
    fn try_as_str(&self) -> miette::Result<&str>;
    fn try_from_str<T>(&self) -> miette::Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: Error + Send + Sync + 'static;
}

impl ValueExt for Value {
    fn try_as_str(&self) -> miette::Result<&str> {
        let Value::BulkString(string) = self else {
            bail!("value is not a bulk string: {self:?}");
        };
        let string = std::str::from_utf8(string)
            .into_diagnostic()
            .wrap_err("parse bulk string into UTF-8")?;
        Ok(string)
    }

    fn try_from_str<T>(&self) -> miette::Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: Error + Send + Sync + 'static,
    {
        self.try_as_str()?
            .parse()
            .into_diagnostic()
            .wrap_err("parse value from string")
    }
}
