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

use crate::AppState;

#[debug_handler]
pub(crate) async fn status(State(_state): State<Arc<AppState>>) -> StatusCode {
    log::info!("GET /status");
    http::StatusCode::OK
}

#[debug_handler]
pub(crate) async fn post(
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
pub(crate) async fn get(State(state): State<Arc<AppState>>, Path(path): Path<String>) -> Response {
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
