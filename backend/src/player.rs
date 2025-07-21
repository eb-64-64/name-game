use std::{pin::pin, sync::Arc};

use futures::stream::unfold;
use tokio_stream::StreamExt;
use tracing::error;

use crate::{GameState, messages::NGMessage, redis_wrapper::RedisWrapper, socket::Socket};

enum Event {
    Message(miette::Result<Option<NGMessage>>),
    StateChange(GameState),
}

pub async fn handle_player(mut socket: Socket, redis_wrapper: Arc<RedisWrapper>) {
    match redis_wrapper.state() {
        GameState::Submitting => socket.send(NGMessage::Submitting).await.unwrap(),
        GameState::NotSubmitting => socket.send(NGMessage::NotSubmitting).await.unwrap(),
    }

    let (mut socket_sender, socket_receiver) = socket.split();

    let a = unfold(socket_receiver, async |mut socket_receiver| {
        Some((
            Event::Message(socket_receiver.recv().await),
            socket_receiver,
        ))
    });
    let b = redis_wrapper.state_change_stream().map(Event::StateChange);
    let mut stream = pin!(a.merge(b));

    while let Some(event) = stream.next().await {
        match event {
            Event::Message(msg) => {
                let msg = match msg {
                    Ok(Some(msg)) => msg,
                    Ok(None) => break,
                    Err(err) => {
                        error!("error while receiving message from player: {err:?}");
                        break;
                    }
                };
                let NGMessage::Name(name) = msg else {
                    error!("got non-name message from player: {msg:?}");
                    break;
                };
                if let GameState::Submitting = redis_wrapper.state() {
                    redis_wrapper.add_name(name).await.unwrap()
                }
            }
            Event::StateChange(new_state) => match new_state {
                GameState::Submitting => socket_sender.send(NGMessage::Submitting).await.unwrap(),
                GameState::NotSubmitting => {
                    socket_sender.send(NGMessage::NotSubmitting).await.unwrap()
                }
            },
        }
    }
}
