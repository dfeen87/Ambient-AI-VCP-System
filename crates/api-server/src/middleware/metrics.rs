/// Prometheus metrics middleware and exporter
///
/// Exposes /metrics endpoint and tracks per-route metrics

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec, register_int_counter_vec, Encoder, HistogramVec, IntCounterVec,
    TextEncoder,
};
use std::time::Instant;
use tracing::error;

lazy_static! {
    /// HTTP request duration histogram (in seconds)
    static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request latencies in seconds",
        &["method", "endpoint", "status"]
    )
    .unwrap();

    /// HTTP request counter
    static ref HTTP_REQUEST_COUNTER: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Total HTTP requests",
        &["method", "endpoint", "status"]
    )
    .unwrap();

    /// HTTP error counter
    static ref HTTP_ERROR_COUNTER: IntCounterVec = register_int_counter_vec!(
        "http_errors_total",
        "Total HTTP errors",
        &["method", "endpoint", "status"]
    )
    .unwrap();
}

/// Metrics collection middleware
pub async fn metrics_middleware(
    request: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    // Normalize path to avoid high cardinality
    let endpoint = normalize_endpoint(&path);

    let response = next.run(request).await;
    let status = response.status().as_u16().to_string();
    let duration = start.elapsed().as_secs_f64();

    // Record metrics
    HTTP_REQUEST_DURATION
        .with_label_values(&[&method, &endpoint, &status])
        .observe(duration);

    HTTP_REQUEST_COUNTER
        .with_label_values(&[&method, &endpoint, &status])
        .inc();

    // Track errors (4xx and 5xx)
    if response.status().is_client_error() || response.status().is_server_error() {
        HTTP_ERROR_COUNTER
            .with_label_values(&[&method, &endpoint, &status])
            .inc();
    }

    response
}

/// Normalize endpoint path to reduce cardinality
fn normalize_endpoint(path: &str) -> String {
    // Replace UUIDs and IDs with placeholders
    let parts: Vec<&str> = path.split('/').collect();
    let normalized: Vec<String> = parts
        .iter()
        .map(|&part| {
            // Check if part looks like a UUID or ID
            if part.len() == 36 && part.contains('-') {
                "{id}".to_string()
            } else if part.chars().all(|c| c.is_ascii_hexdigit()) && part.len() >= 8 {
                "{id}".to_string()
            } else {
                part.to_string()
            }
        })
        .collect();
    normalized.join("/")
}

/// Metrics endpoint handler
async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        error!("Failed to encode metrics: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to encode metrics".to_string(),
        )
            .into_response();
    }

    let output = String::from_utf8(buffer).unwrap_or_else(|e| {
        error!("Failed to convert metrics to UTF-8: {}", e);
        "Failed to convert metrics".to_string()
    });

    (StatusCode::OK, output).into_response()
}

/// Create metrics router
pub fn create_metrics_router<S>() -> Router<S> 
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/metrics", get(metrics_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_endpoint() {
        assert_eq!(
            normalize_endpoint("/api/v1/nodes/550e8400-e29b-41d4-a716-446655440000"),
            "/api/v1/nodes/{id}"
        );
        assert_eq!(
            normalize_endpoint("/api/v1/tasks/abc123"),
            "/api/v1/tasks/abc123"
        );
        assert_eq!(
            normalize_endpoint("/api/v1/tasks/deadbeef12345678"),
            "/api/v1/tasks/{id}"
        );
    }
}
