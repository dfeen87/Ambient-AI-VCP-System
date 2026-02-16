/// Security headers middleware
///
/// Adds security-related HTTP headers to all responses

use axum::{
    body::Body,
    extract::Request,
    http::header,
    middleware::Next,
    response::Response,
};

/// Add security headers to response
pub async fn security_headers_middleware(
    request: Request<Body>,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Strict-Transport-Security (HSTS)
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );

    // X-Content-Type-Options
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        "nosniff".parse().unwrap(),
    );

    // X-Frame-Options
    headers.insert(
        header::X_FRAME_OPTIONS,
        "DENY".parse().unwrap(),
    );

    // Referrer-Policy
    headers.insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    // X-XSS-Protection (legacy but still useful)
    headers.insert(
        "X-XSS-Protection",
        "1; mode=block".parse().unwrap(),
    );

    // Content-Security-Policy
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'"
            .parse()
            .unwrap(),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::{Request, StatusCode}};

    #[tokio::test]
    async fn test_security_headers_added() {
        let request = Request::builder().body(Body::empty()).unwrap();
        
        let next = |_req: Request<Body>| async {
            Response::builder()
                .status(StatusCode::OK)
                .body(Body::empty())
                .unwrap()
        };
        
        let response = security_headers_middleware(
            request,
            Next::new(Box::pin(next)),
        )
        .await;
        
        let headers = response.headers();
        assert!(headers.contains_key("strict-transport-security"));
        assert!(headers.contains_key("x-content-type-options"));
        assert!(headers.contains_key("x-frame-options"));
        assert!(headers.contains_key("referrer-policy"));
    }
}
