use crate::metrics::MetricsMiddleware;
use crate::{handlers, metrics};
use axum::{Router, middleware, routing::get};
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::config::Config;
use crate::link_generator::LinkGenerator;
use crate::storage::Storage;

pub struct AppState {
    pub storage: Storage,
    pub link_generator: LinkGenerator,
}

pub struct App {
    router: Router,
    listener: TcpListener,
}

impl App {
    pub async fn from_config(config: &Config) -> Self {
        let state = Arc::new(AppState {
            link_generator: LinkGenerator::from_config(config),
            storage: Storage::from_config(config).await,
        });

        let router = handlers::api::v1::router()
            .route("/status", get(handlers::status))
            .route("/metrics", get(metrics::metrics_handler))
            .layer(middleware::from_fn(MetricsMiddleware::record))
            .with_state(state.clone());

        let addr = format!("{}:{}", config.host, config.port);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        App { router, listener }
    }

    #[allow(dead_code)]
    pub fn get_addr(&self) -> Result<std::net::SocketAddr, std::io::Error> {
        self.listener.local_addr()
    }

    pub async fn run(self) {
        let addr = self.listener.local_addr().expect("Cannot get local addr");
        log::info!("Starting to accept clients on {addr}");
        axum::serve(self.listener, self.router).await.unwrap();
    }
}
