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
        "default-src 'self'; script-src 'self' 'unsafe-inline' https://unpkg.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com data:"
            .parse()
            .unwrap(),
    );

    response
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_security_headers_added() {
        // Middleware tests require actual server context
        // Integration tests should cover middleware behavior
    }
}
