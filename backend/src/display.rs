use futures::stream::unfold;
use std::{pin::pin, sync::Arc};
use tokio_stream::StreamExt;
use tracing::{error, warn};

use crate::{GameState, messages::NGMessage, redis_wrapper::RedisWrapper, socket::Socket};

enum Event {
    Message(miette::Result<Option<NGMessage>>),
    NewNameCount(usize),
    NameGuessed(usize),
    StateChange(GameState),
}

pub async fn handle_display(mut socket: Socket, redis_wrapper: Arc<RedisWrapper>) {
    match redis_wrapper.state() {
        GameState::Submitting => socket
            .send(NGMessage::NumNames(redis_wrapper.name_count()))
            .await
            .unwrap(),
        GameState::Playing => {
            let (names, guesses) = redis_wrapper.names_and_guesses().await.unwrap();
            socket.send(NGMessage::Names(names, guesses)).await.unwrap()
        }
    }

    let (mut socket_sender, socket_receiver) = socket.split();

    let a = unfold(socket_receiver, async |mut socket_receiver| {
        Some((
            Event::Message(socket_receiver.recv().await),
            socket_receiver,
        ))
    });
    let b = redis_wrapper.name_count_stream().map(Event::NewNameCount);
    let c = redis_wrapper.guess_stream().map(Event::NameGuessed);
    let d = redis_wrapper.state_change_stream().map(Event::StateChange);
    let mut stream = pin!(a.merge(b).merge(c).merge(d));

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
                    NGMessage::StatePlaying => {
                        redis_wrapper.change_state_to_playing().await.unwrap();
                    }
                    NGMessage::MakeGuess(index) => {
                        redis_wrapper.make_guess(index).await.unwrap();
                    }
                    NGMessage::StateSubmitting => {
                        redis_wrapper.change_state_to_submitting().await.unwrap();
                    }
                    _ => {
                        warn!("got unexpected message from display: {msg:?}");
                        continue;
                    }
                }
            }
            Event::NewNameCount(num_names) => {
                socket_sender
                    .send(NGMessage::NumNames(num_names))
                    .await
                    .unwrap();
            }
            Event::NameGuessed(index) => {
                socket_sender
                    .send(NGMessage::NameGuessed(index))
                    .await
                    .unwrap();
            }
            Event::StateChange(state) => match state {
                GameState::Submitting => socket_sender.send(NGMessage::NumNames(0)).await.unwrap(),
                GameState::Playing => {
                    let (names, guesses) = redis_wrapper.names_and_guesses().await.unwrap();
                    socket_sender
                        .send(NGMessage::Names(names, guesses))
                        .await
                        .unwrap()
                }
            },
        }
    }
}
