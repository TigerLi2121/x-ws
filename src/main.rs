use axum::{routing::get, Router};
use log::init_log;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tower_http::services::ServeDir;
use uuid::Uuid;
use ws::ws_handler;

mod log;
mod ws;

#[derive(Clone)]
struct AppState {
    clients: Arc<Mutex<HashMap<Uuid, broadcast::Sender<String>>>>,
}
#[tokio::main]
async fn main() {
    init_log().await;

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

    let app_state = AppState { clients: Arc::new(Mutex::new(HashMap::new())) };

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}


