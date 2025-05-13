use axum::response::IntoResponse;
use axum_macros::debug_handler;
use lazy_static::lazy_static;
use prometheus::{
    HistogramVec, IntCounterVec, TextEncoder, register_histogram_vec, register_int_counter_vec,
};
use std::time::Instant;

lazy_static! {
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Total number of HTTP requests",
        &["method", "status"]
    )
    .unwrap();
    pub static ref HTTP_RESPONSE_TIME_MS: HistogramVec = register_histogram_vec!(
        "http_response_time_ms",
        "HTTP response times",
        &["method"],
        vec![1.0, 10.0, 50.0, 200.0, 1000.0, 3000.0, 10000.0]
    )
    .unwrap();
}

#[debug_handler]
pub async fn metrics_handler() -> axum::response::Response {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder
        .encode_to_string(&metric_families)
        .expect("Cannot encode prometheus data")
        .into_response()
}

pub struct MetricsMiddleware;

impl MetricsMiddleware {
    pub async fn record(
        request: axum::http::Request<axum::body::Body>,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        let start_time = Instant::now();
        let path = request.uri().path().to_string();
        let method = request.method().to_string();

        let response = next.run(request).await;

        if path == "/status" || path == "/metrics" {
            return response;
        }

        let status = response.status().as_u16().to_string();
        let duration = start_time.elapsed().as_millis() as f64;

        HTTP_REQUESTS_TOTAL
            .with_label_values(&[&method, &status])
            .inc();

        HTTP_RESPONSE_TIME_MS
            .with_label_values(&[&method])
            .observe(duration);

        response
    }
}
