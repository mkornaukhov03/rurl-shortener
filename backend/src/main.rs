mod app;
mod config;
mod handlers;
mod link_generator;
mod metrics;
mod storage;
mod validation;

use app::App;
use config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = Config::from_env();

    let app = App::from_config(&config).await;
    app.run().await;
}
