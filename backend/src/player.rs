use std::{pin::pin, sync::Arc};

use futures::stream::unfold;
use tokio_stream::StreamExt;
use tracing::error;

use crate::{
    GameState,
    messages::NGMessage,
    redis_wrapper::RedisWrapper,
    socket::{Sender, Socket},
};

enum Event {
    Message(miette::Result<Option<NGMessage>>),
    StateChange(GameState),
}

async fn send_state(state: GameState, socket: &mut Sender, redis_wrapper: &RedisWrapper) {
    match state {
        GameState::Submitting(epoch) => socket
            .send(NGMessage::StateSubmitting(epoch))
            .await
            .unwrap(),
        GameState::Playing => {
            let (names, guesses) = redis_wrapper.names_and_guesses().await.unwrap();
            socket.send(NGMessage::Names(names, guesses)).await.unwrap()
        }
    }
}

pub async fn handle_player(socket: Socket, redis_wrapper: Arc<RedisWrapper>) {
    let (mut socket_sender, socket_receiver) = socket.split();
    send_state(redis_wrapper.state(), &mut socket_sender, &redis_wrapper).await;

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
                match msg {
                    NGMessage::SubmitName(name)
                        if matches!(redis_wrapper.state(), GameState::Submitting(_)) =>
                    {
                        let id = redis_wrapper.add_name(&name).await.unwrap();
                        socket_sender
                            .send(NGMessage::NameSubmitted(name, id))
                            .await
                            .unwrap()
                    }
                    NGMessage::UnsubmitName(id)
                        if matches!(redis_wrapper.state(), GameState::Submitting(_)) =>
                    {
                        redis_wrapper.remove_name(&id).await.unwrap();
                        socket_sender
                            .send(NGMessage::NameUnsubmitted(id))
                            .await
                            .unwrap()
                    }
                    _ => {
                        error!("unexpected message from player: {msg:?}");
                        break;
                    }
                }
            }
            Event::StateChange(new_state) => {
                send_state(new_state, &mut socket_sender, &redis_wrapper).await;
            }
        }
    }
}
