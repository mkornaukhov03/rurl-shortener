use std::{collections::HashMap, str::Utf8Error, sync::Once, time::Duration};

use rand::Rng;
use reqwest::StatusCode;
use rurl_shortener::{app::App, config::Config};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{sync::RwLock, time::timeout};

use futures::{StreamExt, stream::FuturesUnordered};

#[derive(Serialize)]
struct ShortenRequest {
    url: String,
}

#[derive(Serialize, Deserialize)]
struct ShortenResponse {
    short: String,
}

struct Connection {
    port: u16,
    host: String,
    client: reqwest::Client,
}

impl Connection {
    async fn shorten_request(&self, req: ShortenRequest) -> Result<ShortenResponse, Error> {
        Ok(self
            .client
            .post(format!("http://{}:{}/api/v1/", self.host, self.port))
            .json(&req)
            .send()
            .await?
            .json::<ShortenResponse>()
            .await?)
    }

    async fn get_link_request(&self, short_link: String) -> Result<String, Error> {
        let response = self
            .client
            .get(format!(
                "http://{}:{}/api/v1/{short_link}",
                self.host, self.port
            ))
            .send()
            .await?;
        if response.status() == StatusCode::MOVED_PERMANENTLY {
            let location = response
                .headers()
                .get("Location")
                .ok_or_else(|| Error::MissingLocationHeader)?;
            let full_link = std::str::from_utf8(location.as_bytes())?.to_string();
            return Ok(full_link);
        }

        Err(Error::Not301Redirect(response.status().into()))
    }

    async fn is_alive(&self) -> bool {
        match self
            .client
            .get(format!("http://{}:{}/status", self.host, self.port))
            .send()
            .await
        {
            Ok(resp) => resp.status() == 200,
            Err(_) => false,
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP request failed")]
    Reqwest(#[from] reqwest::Error),

    #[error("Not a 301 redirect (got {0})")]
    Not301Redirect(u16),

    #[error("Missing Location header")]
    MissingLocationHeader,

    #[error("Non-utf8 link")]
    NonUtf8Link(#[from] Utf8Error),
}

fn inmemory_random_config() -> Config {
    Config {
        port: 0,
        host: "127.0.0.1".to_string(),
        redis_endpoint: None,
        openrouter_token: None,
    }
}

fn no_redirect_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Cannot build reqwest::client")
}

static INIT_LOGGER: Once = Once::new();
fn logger_init() {
    INIT_LOGGER.call_once(|| {
        env_logger::init();
    });
}

#[must_use]
async fn app_init() -> (Connection, App) {
    let config = inmemory_random_config();
    let app = App::from_config(&config).await;
    let addr = app.get_addr().expect("Cannot get local addr");

    let client = no_redirect_client();
    let conn = Connection {
        port: addr.port(),
        host: config.host,
        client,
    };

    (conn, app)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_alive() {
    logger_init();

    let (conn, app) = app_init().await;

    let script = async {
        assert!(conn.is_alive().await);
    };

    let (_, script_res) = tokio::join!(
        timeout(Duration::from_secs(1), app.run()),
        timeout(Duration::from_secs(1), script)
    );

    assert!(script_res.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shorten_one_link() {
    logger_init();

    let (conn, app) = app_init().await;
    let full_link = "https://vk.com";

    let script = async {
        let shorten_resp = conn
            .shorten_request(ShortenRequest {
                url: full_link.to_string(),
            })
            .await
            .expect("Cannot shorten link");
        let returned_full_link = conn
            .get_link_request(shorten_resp.short)
            .await
            .expect("Cannot get full link back");
        assert!(returned_full_link == full_link);
    };

    let (_, script_res) = tokio::join!(
        timeout(Duration::from_secs(1), app.run()),
        timeout(Duration::from_secs(1), script)
    );

    assert!(script_res.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shorten_multiple_links_concurrently() {
    logger_init();

    let (conn, app) = app_init().await;
    let full_links = vec![
        "https://vk.com".to_string(),
        "http://example.com".to_string(),
        "https://yandex.ru/search?text=rust".to_string(),
        "https://api.github.com/users/rust-lang/repos?sort=updated&page=2".to_string(),
        "https://en.wikipedia.org/wiki/Main_Page".to_string(),
        "https://docs.rs/".to_string(),
    ];
    let full_links_size = full_links.len();

    let short_to_full: RwLock<HashMap<String, String>> = Default::default();

    let script = async {
        let futures = FuturesUnordered::new();

        // Generating short links
        for full_link in full_links.into_iter() {
            futures.push(async {
                let shorten_resp = conn
                    .shorten_request(ShortenRequest {
                        url: full_link.clone(),
                    })
                    .await
                    .expect("Cannot shorten link");
                assert!(
                    short_to_full
                        .write()
                        .await
                        .insert(shorten_resp.short, full_link)
                        .is_none()
                );
            });
        }

        let _: Vec<_> = futures.collect().await;
        let short_links: Vec<(String, String)> = short_to_full
            .read()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        assert!(short_links.len() == full_links_size);

        // Getting full links
        let futures = FuturesUnordered::new();

        let get_tasks_cnt = 100;
        for _ in 0..get_tasks_cnt {
            futures.push(async {
                let requests_per_task = 100;
                for _ in 0..requests_per_task {
                    let (short, full) =
                        &short_links[rand::rng().random_range(0..short_links.len())];
                    let returned_full_link = conn
                        .get_link_request(short.to_string())
                        .await
                        .expect("Cannot get full link back");
                    assert!(returned_full_link == *full);
                }
            });
        }

        let _: Vec<_> = futures.collect().await;
    };

    let (_, script_res) = tokio::join!(
        timeout(Duration::from_secs(3), app.run()),
        timeout(Duration::from_secs(3), script)
    );

    assert!(script_res.is_ok());
}
