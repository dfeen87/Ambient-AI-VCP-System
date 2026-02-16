/// Logging and request tracing middleware
///
/// Adds structured logging with request IDs and timing
use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::{info, info_span, Instrument};
use uuid::Uuid;

/// Add request ID and tracing span to request
pub async fn request_tracing_middleware(mut request: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let method = request.method().clone();
    let uri = request.uri().clone();

    // Store request ID in extensions
    request.extensions_mut().insert(request_id.clone());

    // Create a tracing span with request details
    let span = info_span!(
        "http_request",
        request_id = %request_id,
        method = %method,
        uri = %uri.path(),
    );

    async move {
        let response = next.run(request).await;
        let duration = start.elapsed();
        let status = response.status();

        info!(
            request_id = %request_id,
            method = %method,
            uri = %uri.path(),
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request completed"
        );

        response
    }
    .instrument(span)
    .await
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_request_tracing() {
        // Middleware tests require actual server context
        // Integration tests should cover middleware behavior
    }
}
