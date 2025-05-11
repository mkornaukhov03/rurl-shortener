mod config;
mod handlers;
mod link_generator;
mod storage;

use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

use config::Config;
use link_generator::LinkGenerator;
use storage::Storage;

struct AppState {
    storage: Storage,
    link_generator: LinkGenerator,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = Config::from_env();

    let state = Arc::new(AppState {
        link_generator: LinkGenerator::from_config(&config),
        storage: Storage::from_config(&config).await,
    });

    let app = Router::new()
        .route("/{*link}", get(handlers::get))
        .route("/status", get(handlers::status))
        .route("/", post(handlers::post))
        .with_state(state.clone());

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    log::info!("Starting to accept clients on {addr}");
    axum::serve(listener, app).await.unwrap();
}
