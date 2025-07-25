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

use crate::GameState;

const NAMES_KEY: &'static str = "names";
const GUESSES_KEY: &'static str = "guesses";
const STATE_KEY: &'static str = "gameState";

const NUM_NAMES_CHANNEL: &'static str = "numNames";
const GUESS_CHANNEL: &'static str = "guess";
const STATE_CHANGE_CHANNEL: &'static str = "stateChange";

static ADD_NAME_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        &r#"
local num_names = server.call("RPUSH", KEYS[1], ARGV[1])
server.call("PUBLISH", "NUM_NAMES_CHANNEL", num_names)
"#
        .trim()
        .replace("NUM_NAMES_CHANNEL", NUM_NAMES_CHANNEL),
    )
});

static MAKE_GUESS_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        &r#"
server.call("SETBIT", KEYS[1], ARGV[1], 1)
server.call("PUBLISH", "GUESS_CHANNEL", ARGV[1])
"#
        .trim()
        .replace("GUESS_CHANNEL", GUESS_CHANNEL),
    )
});
static CHANGE_STATE_TO_SUBMITTING: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        &r#"
-- clear names and guesses
server.call("DEL", KEYS[2])
server.call("DEL", KEYS[3])

-- set state
server.call("SET", KEYS[1], "SUBMITTING_STATE")

-- publish state change
server.call("PUBLISH", "STATE_CHANGE_CHANNEL", "SUBMITTING_STATE")
"#
        .trim()
        .replace("STATE_CHANGE_CHANNEL", STATE_CHANGE_CHANNEL)
        .replace("SUBMITTING_STATE", GameState::Submitting.to_str()),
    )
});
static CHANGE_STATE_TO_PLAYING: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        &r#"
-- shuffle names
math.randomseed(ARGV[1])
local names = server.call("LRANGE", KEYS[2], 0, -1)
for i = 1, #names - 1 do
    local j = math.random(i, #names)
    names[i], names[j] = names[j], names[i]
end
server.call("DEL", KEYS[2])
server.call("RPUSH", KEYS[2], unpack(names))

-- set state
server.call("SET", KEYS[1], "PLAYING_STATE")

-- publish state change
server.call("PUBLISH", "STATE_CHANGE_CHANNEL", "PLAYING_STATE")
"#
        .trim()
        .replace("STATE_CHANGE_CHANNEL", STATE_CHANGE_CHANNEL)
        .replace("PLAYING_STATE", GameState::Playing.to_str()),
    )
});

#[derive(Debug)]
pub struct RedisWrapper {
    _client: Client,
    conn: MultiplexedConnection,
    num_names_receiver: WatchReceiver<usize>,
    guess_receiver: BroadcastReceiver<usize>,
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
        conn.subscribe(&[NUM_NAMES_CHANNEL, GUESS_CHANNEL, STATE_CHANGE_CHANNEL])
            .await
            .into_diagnostic()?;

        let num_names = conn
            .llen(NAMES_KEY)
            .await
            .into_diagnostic()
            .wrap_err("get initial name count")?;
        let (num_names_sender, num_names_receiver) = tokio::sync::watch::channel(num_names);

        let (guess_sender, guess_receiver) = tokio::sync::broadcast::channel(128);

        let game_state = match conn.get(STATE_KEY).await {
            Ok(Some(s)) => s.parse().wrap_err("get initial game state")?,
            Ok(None) => GameState::Submitting,
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
                    STATE_CHANGE_CHANNEL => {
                        let Ok(state) = push.data[1].try_as_str() else {
                            warn!("got non-string on state change channel: {:?}", push.data[1]);
                            continue;
                        };
                        let Ok(state) = state.parse::<GameState>() else {
                            warn!("got unknown state on state change channel: {state}");
                            continue;
                        };
                        if let GameState::Submitting = state {
                            // update number of names without sending a
                            // notification (no notification is needed, as any
                            // currently connected displays will get a 0 num
                            // names packet when the state change is observed)
                            num_names_sender.send_if_modified(|num| {
                                *num = 0;
                                false
                            });
                        }
                        state_change_sender.send_replace(state);
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

    pub async fn add_name(&self, name: String) -> miette::Result<()> {
        ADD_NAME_SCRIPT
            .key(NAMES_KEY)
            .arg(name)
            .invoke_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("add name")
    }

    pub async fn names_and_guesses(&self) -> miette::Result<(Vec<String>, Vec<u8>)> {
        let (names, mut guesses): (Vec<String>, Vec<u8>) = redis::pipe()
            .lrange(NAMES_KEY, 0, -1)
            .get(GUESSES_KEY)
            .query_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("get names and guesses")?;

        // if there are more guesses than names, just get rid of the extra ones
        // at the back
        guesses.truncate((names.len() + 7) / 8);

        Ok((names, guesses))
    }

    pub async fn make_guess(&self, index: usize) -> miette::Result<()> {
        MAKE_GUESS_SCRIPT
            .key(GUESSES_KEY)
            .arg(index)
            .invoke_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("make guess")
    }

    pub fn guess_stream(&self) -> impl Stream<Item = usize> {
        BroadcastStream::new(self.guess_receiver.resubscribe())
            .map(|res| res.expect("the guess channel's sender should not have been dropped"))
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
