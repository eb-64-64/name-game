use futures::stream::unfold;
use std::{pin::pin, sync::Arc};
use tokio_stream::StreamExt;
use tracing::{error, warn};

use crate::{GameState, messages::NGMessage, redis_wrapper::RedisWrapper, socket::Socket};

enum Event {
    Message(miette::Result<Option<NGMessage>>),
    StateChange(GameState),
    NewNameCount(usize),
}

pub async fn handle_display(socket: Socket, redis_wrapper: Arc<RedisWrapper>) {
    let (mut socket_sender, socket_receiver) = socket.split();

    let a = unfold(socket_receiver, async |mut socket_receiver| {
        Some((
            Event::Message(socket_receiver.recv().await),
            socket_receiver,
        ))
    });
    let b = redis_wrapper.name_count_stream().map(Event::NewNameCount);
    let c = redis_wrapper.state_change_stream().map(Event::StateChange);
    let mut stream = pin!(a.merge(b).merge(c));

    while let Some(event) = stream.next().await {
        match event {
            Event::Message(msg) => {
                let msg = match msg {
                    Ok(Some(msg)) => msg,
                    Ok(None) => break,
                    Err(err) => {
                        error!("error while receiving message from display: {err:?}");
                        break;
                    }
                };
                match msg {
                    NGMessage::Submitting => {
                        redis_wrapper.change_state_to_submitting().await.unwrap();
                    }
                    NGMessage::NotSubmitting => {
                        redis_wrapper
                            .change_state_to_not_submitting()
                            .await
                            .unwrap();
                    }
                    _ => {
                        warn!("got unexpected message from display: {msg:?}");
                        continue;
                    }
                }
            }
            Event::StateChange(state) => match state {
                GameState::Submitting => socket_sender.send(NGMessage::NumNames(0)).await.unwrap(),
                GameState::NotSubmitting => socket_sender
                    .send(NGMessage::Names(redis_wrapper.names().await.unwrap()))
                    .await
                    .unwrap(),
            },
            Event::NewNameCount(num_names) => {
                socket_sender
                    .send(NGMessage::NumNames(num_names))
                    .await
                    .unwrap();
            }
        }
    }
}
