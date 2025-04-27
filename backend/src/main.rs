use std::{collections::HashMap, env, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{self, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_macros::debug_handler;
use tokio::sync::RwLock;

#[derive(Default)]
struct AppState {
    link_map: RwLock<HashMap<String, String>>,
    config: Config,
}

#[derive(Default)]
struct Config {
    port: u16,
    host: String,
}

/*
    GET  /%SHORT%
    GET  /status
    POST /
*/

fn config_from_env() -> Config {
    let port: u16 = env::var("RURL_PORT")
        .expect("Provide port via RURL_PORT")
        .parse()
        .expect("RURL_PORT is not a valid port");

    let host = env::var("RURL_HOST").expect("Provide host via RURL_HOST");

    Config {
        port,
        host,
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        config: config_from_env(),
        ..Default::default()
    });

    let app = Router::new()
        .route("/{*link}", get(handle_get))
        .route("/status", get(handle_status))
        .route("/", post(handle_post))
        .with_state(state.clone());

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", (*state).config.host, state.config.port))
            .await
            .unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[debug_handler]
async fn handle_status(State(_state): State<Arc<AppState>>) -> StatusCode {
    http::StatusCode::OK
}

#[debug_handler]
async fn handle_post(
    State(state): State<Arc<AppState>>,
    Json(params): Json<HashMap<String, String>>,
) -> StatusCode {
    match (params.get("old"), params.get("short")) {
        (Some(old), Some(short)) => {
            let mut link_map = state.link_map.write().await;
            if link_map.contains_key(short) || short == "status" {
                return http::StatusCode::CONFLICT;
            }
            link_map.insert(short.clone(), old.clone());
            http::StatusCode::OK
        }
        _ => http::StatusCode::BAD_REQUEST,
    }
}

#[debug_handler]
async fn handle_get(State(state): State<Arc<AppState>>, Path(path): Path<String>) -> Response {
    match state.link_map.read().await.get(&path) {
        Some(full) => {
            return (
                StatusCode::MOVED_PERMANENTLY,
                [(header::LOCATION, full)],
                "Moved permanently",
            )
                .into_response();
        }
        None => return (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}
