use std::{str::FromStr, sync::Arc};

use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::any,
};
use miette::{IntoDiagnostic, bail};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{redis_wrapper::RedisWrapper, settings::get_settings, socket::Socket};

mod display;
mod messages;
mod player;
mod redis_wrapper;
mod settings;
mod socket;

#[derive(Clone, Copy, Debug)]
enum GameState {
    Submitting,
    Playing,
}

impl GameState {
    fn to_str(&self) -> &'static str {
        match self {
            GameState::Submitting => "submitting",
            GameState::Playing => "playing",
        }
    }
}

impl FromStr for GameState {
    type Err = miette::Report;

    fn from_str(s: &str) -> miette::Result<Self> {
        match s {
            "submitting" => Ok(GameState::Submitting),
            "playing" => Ok(GameState::Playing),
            _ => bail!("unrecognized game state: {s}"),
        }
    }
}

async fn player_upgrader(
    ws: WebSocketUpgrade,
    State(redis_wrapper): State<Arc<RedisWrapper>>,
) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| {
        player::handle_player(Socket::new(socket), redis_wrapper.clone()).await;
    })
}

async fn display_upgrader(
    ws: WebSocketUpgrade,
    State(redis_wrapper): State<Arc<RedisWrapper>>,
) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| {
        display::handle_display(Socket::new(socket), redis_wrapper.clone()).await;
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

    let mut app = Router::new()
        .route("/ws/player", any(player_upgrader))
        .route("/ws/display", any(display_upgrader))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(RedisWrapper::new(settings.redis_url).await?));
    if let Some(serve_dir) = settings.serve_dir {
        app = app.fallback_service(
            ServeDir::new(&serve_dir).fallback(ServeFile::new(serve_dir.join("index.html"))),
        );
    }

    let listener = tokio::net::TcpListener::bind((settings.host, settings.port))
        .await
        .into_diagnostic()?;
    info!("Listening on {}", listener.local_addr().into_diagnostic()?);
    axum::serve(listener, app.into_make_service())
        .await
        .into_diagnostic()?;

    Ok(())
}
