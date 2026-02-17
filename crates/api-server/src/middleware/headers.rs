/// Security headers middleware
///
/// Adds security-related HTTP headers to all responses
use axum::{body::Body, extract::Request, http::header, middleware::Next, response::Response};

/// Add security headers to response
pub async fn security_headers_middleware(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Strict-Transport-Security (HSTS)
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );

    // X-Content-Type-Options
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());

    // X-Frame-Options
    headers.insert(header::X_FRAME_OPTIONS, "DENY".parse().unwrap());

    // Referrer-Policy
    headers.insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    // X-XSS-Protection (legacy but still useful)
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());

    // Expect-CT
    headers.insert("Expect-CT", "max-age=86400, enforce".parse().unwrap());

    // Content-Security-Policy
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com data:"
            .parse()
            .unwrap(),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::security_headers_middleware;
    use axum::{
        body::Body,
        http::{header, Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_security_headers_added() {
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn(security_headers_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let csp = response
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .expect("CSP header missing")
            .to_str()
            .expect("CSP should be valid UTF-8");

        assert!(csp.contains("script-src 'self'"));
        assert!(!csp.contains("unsafe-inline"));
        assert!(!csp.contains("https://unpkg.com"));
    }
}
