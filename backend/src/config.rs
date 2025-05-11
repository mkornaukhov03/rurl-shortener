use std::env;

#[derive(Default)]
pub struct Config {
    pub port: u16,
    pub host: String,

    pub redis_endpoint: String,
    pub openrouter_token: Option<String>,
}

impl Config {
    pub fn from_env() -> Config {
        let port: u16 = env::var("RURL_PORT")
            .expect("Provide port via RURL_PORT")
            .parse()
            .expect("RURL_PORT is not a valid port");

        let host = env::var("RURL_HOST").expect("Provide host via RURL_HOST");

        // TODO maybe better to fallback into in-memory storage?
        let redis_endpoint = env::var("RURL_REDIS_ENDPOINT")
            .expect("Provide redis endpoint via RURL_REDIS_ENDPOINT");
        log::info!("redis_endpoint = {}", redis_endpoint);
        let openrouter_token = env::var("RURL_OPENROUTER_TOKEN").ok();

        Config {
            port,
            host,
            redis_endpoint,
            openrouter_token,
        }
    }
}
