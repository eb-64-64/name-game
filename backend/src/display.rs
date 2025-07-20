use futures::stream::unfold;
use rand::seq::SliceRandom;
use std::{
    pin::pin,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio_stream::StreamExt;
use tracing::{error, warn};

use crate::{GameState, messages::NGMessage, socket::Socket};

enum Event {
    Message(miette::Result<Option<NGMessage>>),
    NewName(String),
}

pub async fn handle_display(
    socket: Socket,
    state: Arc<Mutex<GameState>>,
    name_receiver: Receiver<String>,
    state_change_sender: Sender<GameState>,
) {
    let mut names = Vec::new();

    let (mut socket_sender, socket_receiver) = socket.split();

    let a = unfold(socket_receiver, async |mut socket_receiver| {
        Some((
            Event::Message(socket_receiver.recv().await),
            socket_receiver,
        ))
    });
    let b = unfold(name_receiver, async |mut name_receiver| {
        Some((
            Event::NewName(name_receiver.recv().await.unwrap()),
            name_receiver,
        ))
    });
    let mut stream = pin!(a.merge(b));

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
                        *state.lock().unwrap() = GameState::Submitting;
                        if let Err(_) = state_change_sender.send(GameState::Submitting) {
                            warn!("no players watching for state change");
                        }
                        socket_sender
                            .send(NGMessage::NumNames(names.len() as u8))
                            .await
                            .unwrap();
                    }
                    NGMessage::NotSubmitting => {
                        *state.lock().unwrap() = GameState::NotSubmitting;
                        if let Err(_) = state_change_sender.send(GameState::NotSubmitting) {
                            warn!("no players watching for state change");
                        }
                        let mut names = std::mem::take(&mut names);
                        names.shuffle(&mut rand::rng());
                        socket_sender.send(NGMessage::Names(names)).await.unwrap();
                    }
                    _ => {
                        warn!("got unexpected message from display: {msg:?}");
                        continue;
                    }
                }
            }
            Event::NewName(new_name) => {
                names.push(new_name);
                socket_sender
                    .send(NGMessage::NumNames(names.len() as u8))
                    .await
                    .unwrap();
            }
        }
    }
}
