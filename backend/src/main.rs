use axum::{Router, extract::WebSocketUpgrade, response::IntoResponse, routing::any};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

async fn player_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(async move |_socket| todo!())
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
