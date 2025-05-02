use std::{collections::HashMap, env, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{self, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_macros::debug_handler;
use tokio::sync::{Mutex, RwLock};

// TODO use pool of connections instead of one
struct RedisSingleConnection {
    conn: Mutex<redis::aio::MultiplexedConnection>,
}

impl RedisSingleConnection {
    async fn new(endpoint: String) -> Self {
        let client =
            redis::Client::open(format!("redis://{}/", endpoint)).expect("Cannot connect to redis");

        RedisSingleConnection {
            conn: Mutex::new(
                client
                    .get_multiplexed_async_connection()
                    .await
                    .expect("Cannot get async connection"),
            ),
        }
    }

    async fn store(&self, short: String, url: String) -> bool {
        // Set key=short with value=url if not set yet atomically
        // TODO think about for how much time to store?
        match self
            .conn
            .lock()
            .await
            .send_packed_command(
                redis::cmd("SET")
                    .arg(short)
                    .arg(url)
                    .arg("EX")
                    .arg(3600)
                    .arg("NX"),
            )
            .await
        {
            Ok(resp) => match resp {
                redis::Value::Okay => return true,
                redis::Value::Nil => return false,
                _ => {
                    log::warn!("Response from redis store is not OK, nor nil");
                    return false;
                }
            },
            Err(e) => {
                log::error!("Error to store in redis: {}", e);
                return false;
            }
        }
    }

    async fn fetch(&self, short: &str) -> Option<String> {
        match self
            .conn
            .lock()
            .await
            .send_packed_command(redis::cmd("GET").arg(short))
            .await
        {
            Ok(resp) => match resp {
                redis::Value::SimpleString(s) => {
                    return Some(s);
                }
                redis::Value::Nil => {
                    return None;
                }
                redis::Value::BulkString(s) => match String::from_utf8(s) {
                    Ok(s) => {
                        return Some(s);
                    }
                    Err(_) => {
                        log::warn!("Non utf-8 url is stored");
                        return None;
                    }
                },
                _ => {
                    log::warn!("Stored value is not string");
                    return None;
                }
            },
            Err(e) => {
                log::error!("Error to fetch from redis: {}", e);
                return None;
            }
        }
    }
}

// TODO validation of url
#[allow(unused)]
enum Storage {
    NonPersistent(RwLock<HashMap<String, String>>),
    Redis(RedisSingleConnection),
}

impl Storage {
    async fn store(&self, short: String, url: String) -> bool {
        match self {
            Storage::NonPersistent(rw_lock) => {
                let mut guard = rw_lock.write().await;
                if guard.contains_key(&short) {
                    false
                } else {
                    guard.insert(short, url);
                    true
                }
            }
            Storage::Redis(redis_single_connection) => {
                redis_single_connection.store(short, url).await
            }
        }
    }

    async fn fetch(&self, short: &str) -> Option<String> {
        match self {
            Storage::NonPersistent(rw_lock) => rw_lock.read().await.get(short).cloned(),
            Storage::Redis(redis_single_connection) => redis_single_connection.fetch(short).await,
        }
    }
}

struct AppState {
    storage: Storage,
    config: Config,
}

#[derive(Default)]
struct Config {
    port: u16,
    host: String,
    redis_endpoint: String,
}

fn config_from_env() -> Config {
    let port: u16 = env::var("RURL_PORT")
        .expect("Provide port via RURL_PORT")
        .parse()
        .expect("RURL_PORT is not a valid port");

    let host = env::var("RURL_HOST").expect("Provide host via RURL_HOST");

    // TODO fallback into in memory or fail?
    let redis_endpoint = env::var("RURL_REDIS_ENDPOINT").unwrap_or("redis:6379".into());

    Config {
        port,
        host,
        redis_endpoint,
    }
}

#[tokio::main]
async fn main() {
    // Env logger is for development purposes
    env_logger::init();

    let config = config_from_env();

    let storage = Storage::Redis(RedisSingleConnection::new(config.redis_endpoint).await);
    let state = Arc::new(AppState {
        config: config_from_env(),
        storage: storage,
    });

    let app = Router::new()
        .route("/{*link}", get(handle_get))
        .route("/status", get(handle_status))
        .route("/", post(handle_post))
        .with_state(state.clone());

    let addr = format!("{}:{}", (*state).config.host, state.config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    log::info!("Starting to accept clients on {addr}");
    axum::serve(listener, app).await.unwrap();
}

#[debug_handler]
async fn handle_status(State(_state): State<Arc<AppState>>) -> StatusCode {
    log::info!("GET /status");
    http::StatusCode::OK
}

#[debug_handler]
async fn handle_post(
    State(state): State<Arc<AppState>>,
    Json(mut params): Json<HashMap<String, String>>,
) -> StatusCode {
    log::info!("POST / ({:?})", params);
    match (params.remove("old"), params.remove("short")) {
        (Some(old), Some(short)) => {
            if short == "status" {
                return http::StatusCode::CONFLICT;
            }
            if state.storage.store(short, old).await {
                http::StatusCode::OK
            } else {
                http::StatusCode::CONFLICT
            }
        }
        _ => http::StatusCode::BAD_REQUEST,
    }
}

#[debug_handler]
async fn handle_get(State(state): State<Arc<AppState>>, Path(path): Path<String>) -> Response {
    log::info!("GET /{}", path);
    match state.storage.fetch(&path).await {
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
