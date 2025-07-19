use axum::{
    Router,
    extract::{
        WebSocketUpgrade,
        ws::Message::{self, Binary},
    },
    response::IntoResponse,
    routing::any,
};
use futures::{SinkExt, StreamExt};
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use messages::NGMessage;

mod messages;

async fn player_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| {
        let (mut sender, mut receiver) = socket.split();
        tokio::spawn(async move {
            sender
                .send(Binary(NGMessage::SubmissionTime.encode()))
                .await
                .unwrap();
        });
        tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(msg) => {
                        if let Message::Binary(msg) = msg {
                            let msg = NGMessage::parse(msg).unwrap();
                            info!("received message: {msg:?}");
                        }
                    }
                    Err(err) => {
                        error!("got error: {err:?}");
                        break;
                    }
                }
            }
        });
    })
}

async fn display_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(async move |_socket| todo!())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_default_env()
                .add_directive("tokio_http=debug".parse().unwrap())
                .add_directive(concat!(env!("CARGO_CRATE_NAME"), "=debug").parse().unwrap()),
        )
        .init();

    let app = Router::new()
        .route("/ws/player", any(player_handler))
        .route("/ws/display", any(display_handler))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap()
}
