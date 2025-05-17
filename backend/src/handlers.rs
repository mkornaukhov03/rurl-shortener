use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Path, State},
    http,
    response::{IntoResponse, Response},
};
use axum_macros::debug_handler;
use maplit::hashmap;
use reqwest::{StatusCode, header};

use crate::{AppState, validation};

#[debug_handler]
pub(crate) async fn status(State(_state): State<Arc<AppState>>) -> StatusCode {
    log::info!("GET /status");
    http::StatusCode::OK
}

pub mod api {
    use super::*;
    pub mod v1 {
        pub(crate) fn router() -> Router<Arc<AppState>> {
            Router::new()
                .route("/api/v1/{*link}", axum::routing::get(get))
                .route("/api/v1/", axum::routing::post(post))
        }

        use axum::Router;

        use super::*;
        #[debug_handler]
        async fn post(
            State(state): State<Arc<AppState>>,
            Json(mut params): Json<HashMap<String, String>>,
        ) -> Response {
            log::info!("POST / ({:?})", params);

            match params.remove("url") {
                Some(url) => {
                    if !validation::is_valid_url(&url) {
                        return (http::StatusCode::BAD_REQUEST, "Invalid url").into_response();
                    }
                    const MAX_ATTEMPTS: usize = 3;

                    for attempt in 1..=MAX_ATTEMPTS {
                        let short = state.link_generator.generate(&url).await;

                        let short = if let Some(s) = short {
                            s
                        } else {
                            log::warn!(
                                "Attempt {}/{} failed to generate short link:",
                                attempt,
                                MAX_ATTEMPTS
                            );
                            continue;
                        };

                        if !validation::is_valid_short_link(&short) {
                            log::warn!(
                                "Attempt {}/{} failed (short link is not valid: {short})",
                                attempt,
                                MAX_ATTEMPTS
                            );
                            continue;
                        }

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
                            "Attempt {}/{} failed (not a unique short link: {short})",
                            attempt,
                            MAX_ATTEMPTS
                        );
                    }

                    (
                        http::StatusCode::SERVICE_UNAVAILABLE,
                        "Cannot generate unique short link",
                    )
                        .into_response()
                }
                _ => http::StatusCode::BAD_REQUEST.into_response(),
            }
        }

        #[debug_handler]
        async fn get(State(state): State<Arc<AppState>>, Path(path): Path<String>) -> Response {
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
    }
}
