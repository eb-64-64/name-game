use std::{
    error::Error,
    marker::{Send, Sync},
    str::FromStr,
    sync::LazyLock,
};

use futures::Stream;
use miette::{Context, IntoDiagnostic, bail};
use rand::{Rng, rng};
use redis::{
    AsyncConnectionConfig, AsyncTypedCommands, Client, PushKind, Script, Value,
    aio::MultiplexedConnection,
};
use secrecy::{ExposeSecret, SecretString};
use tokio::sync::watch::Receiver;
use tokio_stream::wrappers::WatchStream;
use tracing::warn;

use crate::GameState;

const NAMES_KEY: &'static str = "names";
const STATE_KEY: &'static str = "gameState";

const NUM_NAMES_CHANNEL: &'static str = "numNames";
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
static CHANGE_STATE_TO_SUBMITTING: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        &r#"
-- clear names
server.call("DEL", KEYS[2])

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
static CHANGE_STATE_TO_NOT_SUBMITTING: LazyLock<Script> = LazyLock::new(|| {
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
server.call("SET", KEYS[1], "NOT_SUBMITTING_STATE")

-- publish state change
server.call("PUBLISH", "STATE_CHANGE_CHANNEL", "NOT_SUBMITTING_STATE")
"#
        .trim()
        .replace("STATE_CHANGE_CHANNEL", STATE_CHANGE_CHANNEL)
        .replace("NOT_SUBMITTING_STATE", GameState::NotSubmitting.to_str()),
    )
});

#[derive(Debug)]
pub struct RedisWrapper {
    _client: Client,
    conn: MultiplexedConnection,
    num_names_receiver: Receiver<usize>,
    state_change_receiver: Receiver<GameState>,
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
        conn.subscribe(&[NUM_NAMES_CHANNEL, STATE_CHANGE_CHANNEL])
            .await
            .into_diagnostic()?;

        let num_names = conn
            .llen(NAMES_KEY)
            .await
            .into_diagnostic()
            .wrap_err("get initial name count")?;
        let (num_names_sender, num_names_receiver) = tokio::sync::watch::channel(num_names);

        let game_state = match conn.get(STATE_KEY).await {
            Ok(Some(s)) => s.parse().wrap_err("get initial game state")?,
            Ok(None) => GameState::NotSubmitting,
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
            state_change_receiver,
        })
    }

    pub fn name_count(&self) -> usize {
        *self.num_names_receiver.borrow()
    }

    pub fn name_count_stream(&self) -> impl Stream<Item = usize> {
        WatchStream::from_changes(self.num_names_receiver.clone())
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

    pub async fn names(&self) -> miette::Result<Vec<String>> {
        self.conn
            .clone()
            .lrange(NAMES_KEY, 0, -1)
            .await
            .into_diagnostic()
            .wrap_err("get names")
    }

    pub fn state(&self) -> GameState {
        *self.state_change_receiver.borrow()
    }

    pub fn state_change_stream(&self) -> impl Stream<Item = GameState> {
        WatchStream::from_changes(self.state_change_receiver.clone())
    }

    pub async fn change_state_to_submitting(&self) -> miette::Result<()> {
        CHANGE_STATE_TO_SUBMITTING
            .key(STATE_KEY)
            .key(NAMES_KEY)
            .invoke_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("set state to submitting")
    }

    pub async fn change_state_to_not_submitting(&self) -> miette::Result<()> {
        let seed = rng().random::<u32>();
        CHANGE_STATE_TO_NOT_SUBMITTING
            .key(STATE_KEY)
            .key(NAMES_KEY)
            .arg(seed)
            .invoke_async(&mut self.conn.clone())
            .await
            .into_diagnostic()
            .wrap_err("set state to not submitting")
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
