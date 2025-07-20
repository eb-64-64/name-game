use std::pin::pin;

use futures::stream::unfold;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio_stream::StreamExt;
use tracing::{error, warn};

use crate::{GameState, messages::NGMessage, socket::Socket};

enum Event {
    Message(miette::Result<Option<NGMessage>>),
    StateChange(GameState),
}

pub async fn handle_player(
    mut socket: Socket,
    mut state: GameState,
    name_sender: Sender<String>,
    state_change_receiver: Receiver<GameState>,
) {
    match state {
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
    let b = unfold(state_change_receiver, async |mut state_change_receiver| {
        Some((
            Event::StateChange(state_change_receiver.recv().await.unwrap()),
            state_change_receiver,
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
                        error!("error while receiving message from player: {err:?}");
                        break;
                    }
                };
                let NGMessage::Name(name) = msg else {
                    error!("got non-name message from player: {msg:?}");
                    break;
                };
                if let GameState::Submitting = state {
                    match name_sender.send(name) {
                        Err(_) => {
                            // swallow the error, because there's not
                            // really anything we can do
                            warn!("there is no display to take the name");
                        }
                        _ => {}
                    }
                }
            }
            Event::StateChange(new_state) => {
                state = new_state;
                match state {
                    GameState::Submitting => {
                        socket_sender.send(NGMessage::Submitting).await.unwrap()
                    }
                    GameState::NotSubmitting => {
                        socket_sender.send(NGMessage::NotSubmitting).await.unwrap()
                    }
                }
            }
        }
    }
}
