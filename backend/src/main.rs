use std::{collections::HashMap, env, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{self, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_macros::debug_handler;
use maplit::hashmap;
use rand::Rng;
use serde::{Deserialize, Serialize};
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
                redis::Value::Okay => true,
                redis::Value::Nil => false,
                _ => {
                    log::warn!("Response from redis store is not OK, nor nil");
                    false
                }
            },
            Err(e) => {
                log::error!("Error to store in redis: {}", e);
                false
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
                redis::Value::SimpleString(s) => Some(s),
                redis::Value::Nil => None,
                redis::Value::BulkString(s) => match String::from_utf8(s) {
                    Ok(s) => Some(s),
                    Err(_) => {
                        log::warn!("Non utf-8 url is stored");
                        None
                    }
                },
                _ => {
                    log::warn!("Stored value is not string");
                    None
                }
            },
            Err(e) => {
                log::error!("Error to fetch from redis: {}", e);
                None
            }
        }
    }
}

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
                if let std::collections::hash_map::Entry::Vacant(e) = guard.entry(short) {
                    e.insert(url);
                    true
                } else {
                    false
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

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenrouterRequestBody {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
struct ModelResponse {
    short_link: String,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct OpenrouterResponse {
    #[serde(rename = "id")]
    _id: String,
    choices: Vec<Choice>,
}

enum LinkGenerator {
    Random,
    OpenrouterLlama(String),
}

impl LinkGenerator {
    async fn generate(&self, full_link: &str) -> Option<String> {
        match self {
            LinkGenerator::Random => {
                const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                         abcdefghijklmnopqrstuvwxyz\
                         0123456789";
                let mut rng = rand::rng();
                Some(
                    (0..8)
                        .map(|_| {
                            let idx = rng.random_range(0..CHARSET.len());
                            CHARSET[idx] as char
                        })
                        .collect(),
                )
            }
            LinkGenerator::OpenrouterLlama(token) => {
                let prompt = format!(
                    r#"
Can you suggest a short path for a URL shortener for this URL: '{}'? 
Give only one suggestion. It should be one word, possibly with underscores.
Output have to be in json format, don't write anything except the json.
Example output:
{}
"#,
                    full_link, "{\"short_link\": \"url\"}"
                );
                let body = OpenrouterRequestBody {
                    model: "meta-llama/llama-4-maverick:free".to_string(),
                    messages: vec![Message {
                        role: "assistant".to_string(),
                        content: prompt,
                    }],
                };
                let client = reqwest::Client::new();

                let response = match client
                    .post("https://openrouter.ai/api/v1/chat/completions")
                    .header(header::AUTHORIZATION, format!("Bearer {}", token))
                    .json(&body)
                    .send()
                    .await
                {
                    Ok(resp) => resp,
                    Err(e) => {
                        log::error!("Error in openrouter api: {e}");
                        return None;
                    }
                };

                let openrouter_resp: OpenrouterResponse =
                    match response.json::<OpenrouterResponse>().await {
                        Ok(r) => r,
                        Err(e) => {
                            log::error!("Error in demarshalling api: {e}");
                            return None;
                        }
                    };
                let model_response: ModelResponse =
                    serde_json::from_str(&openrouter_resp.choices[0].message.content).ok()?;

                Some(model_response.short_link)
            }
        }
    }
}

struct AppState {
    storage: Storage,
    link_generator: LinkGenerator,
    config: Config,
}

#[derive(Default)]
struct Config {
    port: u16,
    host: String,

    redis_endpoint: String,
    openrouter_token: Option<String>,
}

fn config_from_env() -> Config {
    let port: u16 = env::var("RURL_PORT")
        .expect("Provide port via RURL_PORT")
        .parse()
        .expect("RURL_PORT is not a valid port");

    let host = env::var("RURL_HOST").expect("Provide host via RURL_HOST");

    // TODO fallback into in memory or fail?
    let redis_endpoint = env::var("RURL_REDIS_ENDPOINT").unwrap_or("redis:6379".into());
    log::info!("redis_endpoint = {}", redis_endpoint);
    let openrouter_token = env::var("RURL_OPENROUTER_TOKEN").ok();

    Config {
        port,
        host,
        redis_endpoint,
        openrouter_token,
    }
}

fn get_link_generator(config: &Config) -> LinkGenerator {
    match &config.openrouter_token {
        Some(token) => LinkGenerator::OpenrouterLlama(token.clone()),
        None => LinkGenerator::Random,
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = config_from_env();

    let link_generator = get_link_generator(&config);
    let storage = Storage::Redis(RedisSingleConnection::new(config.redis_endpoint.clone()).await);
    let state = Arc::new(AppState {
        config,
        link_generator,
        storage,
    });

    let app = Router::new()
        .route("/{*link}", get(handle_get))
        .route("/status", get(handle_status))
        .route("/", post(handle_post))
        .with_state(state.clone());

    let addr = format!("{}:{}", state.config.host, state.config.port);
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
) -> Response {
    log::info!("POST / ({:?})", params);

    // TODO validate uri
    match params.remove("url") {
        Some(url) => {
            const MAX_ATTEMPTS: usize = 3;

            for attempt in 1..=MAX_ATTEMPTS {
                let short = state.link_generator.generate(&url).await;

                let short = if let Some(s) = short {
                    s
                } else {
                    log::warn!(
                        "Attempt {}/{} failed to generate short link",
                        attempt,
                        MAX_ATTEMPTS
                    );
                    continue;
                };

                if state.storage.store(short.clone(), url.clone()).await {
                    return (
                        http::StatusCode::OK,
                        Json(hashmap! {
                            "short" => short
                        }),
                    )
                        .into_response();
                }

                log::warn!(
                    "Attempt {}/{} failed to generate unique short link",
                    attempt,
                    MAX_ATTEMPTS
                );
            }

            (
                http::StatusCode::SERVICE_UNAVAILABLE,
                "Cannot generate unique short link after multiple attempts",
            )
                .into_response()
        }
        _ => http::StatusCode::BAD_REQUEST.into_response(),
    }
}

#[debug_handler]
async fn handle_get(State(state): State<Arc<AppState>>, Path(path): Path<String>) -> Response {
    log::info!("GET /{}", path);
    match state.storage.fetch(&path).await {
        Some(full) => (
            StatusCode::MOVED_PERMANENTLY,
            [(header::LOCATION, full)],
            "Moved permanently",
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;

    #[tokio::test]
    async fn test_storage() {
        let storage = Storage::NonPersistent(Default::default());
        assert!(storage.store("key".into(), "val".into()).await);
        assert!(!storage.store("key".into(), "val2".into()).await);
        assert!(storage.fetch("key").await == Some("val".into()));
    }

    #[tokio::test]
    async fn test_random_link_generator() {
        let link_generator = LinkGenerator::Random;
        let keys = ["key1", "key2", "key3"];
        let mut short_links = HashSet::<String>::new();
        for key in keys.iter() {
            short_links.insert(
                link_generator
                    .generate(key)
                    .await
                    .expect("Cannot generate key"),
            );
        }
        assert!(short_links.len() != keys.len());
        // assert!(short_links.len() == keys.len());
    }
}
