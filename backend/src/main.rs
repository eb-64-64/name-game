use std::sync::{Arc, Mutex};

use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::any,
};
use miette::IntoDiagnostic;
use tokio::sync::broadcast::Sender;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{settings::get_settings, socket::Socket};

mod display;
mod messages;
mod player;
mod settings;
mod socket;

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

async fn player_upgrader(
    ws: WebSocketUpgrade,
    State(channels): State<Channels>,
) -> impl IntoResponse {
    let name_sender = channels.name_submission.clone();
    let state_change_receiver = channels.state_change.subscribe();
    let cur_state = *channels.cur_state.lock().unwrap();
    ws.on_upgrade(async move |socket| {
        player::handle_player(
            Socket::new(socket),
            cur_state,
            name_sender,
            state_change_receiver,
        )
        .await;
    })
}

async fn display_upgrader(
    ws: WebSocketUpgrade,
    State(channels): State<Channels>,
) -> impl IntoResponse {
    let name_receiver = channels.name_submission.subscribe();
    let state_change_sender = channels.state_change.clone();
    let cur_state = channels.cur_state.clone();
    ws.on_upgrade(async move |socket| {
        display::handle_display(
            Socket::new(socket),
            cur_state,
            name_receiver,
            state_change_sender,
        )
        .await;
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
        .route("/ws/player", any(player_upgrader))
        .route("/ws/display", any(display_upgrader))
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
