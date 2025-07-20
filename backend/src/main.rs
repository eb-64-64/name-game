use std::sync::{Arc, Mutex};

use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::Message},
    response::IntoResponse,
    routing::any,
};
use miette::IntoDiagnostic;
use rand::seq::SliceRandom;
use tokio::{select, sync::broadcast::Sender};
use tower_http::trace::TraceLayer;
use tracing::{error, warn};
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use messages::NGMessage;

use crate::settings::get_settings;

mod messages;
mod settings;

#[derive(Clone, Copy, Debug)]
enum GameState {
    Submitting,
    NotSubmitting,
}

#[derive(Clone, Debug)]
struct Channels {
    state_change: Sender<GameState>,
    name_submission: Sender<String>,
    cur_state: Arc<Mutex<GameState>>,
}

async fn player_handler(
    ws: WebSocketUpgrade,
    State(channels): State<Channels>,
) -> impl IntoResponse {
    let name_sender = channels.name_submission.clone();
    let mut state_change_receiver = channels.state_change.subscribe();
    let cur_state = *channels.cur_state.lock().unwrap();
    ws.on_upgrade(async move |mut socket| {
        match cur_state {
            GameState::Submitting => socket
                .send(Message::Binary(NGMessage::Submitting.encode()))
                .await
                .unwrap(),
            GameState::NotSubmitting => socket
                .send(Message::Binary(NGMessage::NotSubmitting.encode()))
                .await
                .unwrap(),
        }

        let mut state = cur_state;
        loop {
            select! {
                msg = socket.recv() => {
                    let msg = match msg {
                        Some(Ok(Message::Binary(msg))) => msg,
                        Some(Ok(msg)) => {
                            warn!("got non-binary message from player: {msg:?}");
                            continue
                        }
                        Some(Err(err)) => {
                            error!("got error from player: {err:?}");
                            break
                        }
                        None => break,
                    };
                    let Some(msg) = NGMessage::parse(msg) else {
                        warn!("could not parse message from player");
                        continue
                    };
                    let NGMessage::Name(name) = msg else {
                        warn!("got non-name message from player: {msg:?}");
                        continue
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
                new_state = state_change_receiver.recv() => {
                    state = new_state.unwrap();
                    match state {
                        GameState::Submitting => socket
                            .send(Message::Binary(NGMessage::Submitting.encode()))
                            .await
                            .unwrap(),
                        GameState::NotSubmitting => socket
                            .send(Message::Binary(NGMessage::NotSubmitting.encode()))
                            .await
                            .unwrap(),
                    }
                }
            }
        }
    })
}

async fn display_handler(
    ws: WebSocketUpgrade,
    State(channels): State<Channels>,
) -> impl IntoResponse {
    let mut name_receiver = channels.name_submission.subscribe();
    let state_change_sender = channels.state_change.clone();
    let cur_state = channels.cur_state.clone();
    ws.on_upgrade(async move |mut socket| {
        let mut names = Vec::new();
        loop {
            select! {
                msg = socket.recv() => {
                    let msg = match msg {
                        Some(Ok(Message::Binary(msg))) => msg,
                        Some(Ok(msg)) => {
                            warn!("got non-binary message from display: {msg:?}");
                            continue
                        }
                        Some(Err(err)) => {
                            error!("got error from display: {err:?}");
                            break
                        }
                        None => break,
                    };
                    let Some(msg) = NGMessage::parse(msg) else {
                        warn!("could not parse message from display");
                        continue
                    };
                    match msg {
                        NGMessage::Submitting => {
                            *cur_state.lock().unwrap() = GameState::Submitting;
                            if let Err(_) = state_change_sender.send(GameState::Submitting) {
                                warn!("no players watching for state change");
                            }
                            socket.send(
                                Message::Binary(
                                    NGMessage::NumNames(names.len() as u8).encode()
                                )
                            ).await.unwrap();
                        }
                        NGMessage::NotSubmitting => {
                            *cur_state.lock().unwrap() = GameState::NotSubmitting;
                            if let Err(_) = state_change_sender.send(GameState::NotSubmitting) {
                                warn!("no players watching for state change");
                            }
                            let mut names = std::mem::take(&mut names);
                            names.shuffle(&mut rand::rng());
                            socket.send(
                                Message::Binary(
                                    NGMessage::Names(names).encode()
                                )
                            ).await.unwrap();
                        }
                        _ => {
                            warn!("got unexpected message from display: {msg:?}");
                            continue
                        }
                    }
                }
                new_name = name_receiver.recv() => {
                    names.push(new_name.unwrap());
                    socket.send(
                        Message::Binary(
                            NGMessage::NumNames(names.len() as u8).encode()
                        )
                    ).await.unwrap();
                }
            }
        }
    })
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_default_env()
                .add_directive("tokio_http=debug".parse().unwrap())
                .add_directive(concat!(env!("CARGO_CRATE_NAME"), "=debug").parse().unwrap()),
        )
        .init();

    let settings = tokio::task::spawn_blocking(|| get_settings())
        .await
        .into_diagnostic()??;

    let (state_change, _) = tokio::sync::broadcast::channel(256);
    let (name_submission, _) = tokio::sync::broadcast::channel(256);
    let app = Router::new()
        .route("/ws/player", any(player_handler))
        .route("/ws/display", any(display_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(Channels {
            state_change,
            name_submission,
            cur_state: Arc::new(Mutex::new(GameState::NotSubmitting)),
        });

    let listener = tokio::net::TcpListener::bind((settings.host, settings.port))
        .await
        .into_diagnostic()?;
    axum::serve(listener, app.into_make_service())
        .await
        .into_diagnostic()?;

    Ok(())
}
