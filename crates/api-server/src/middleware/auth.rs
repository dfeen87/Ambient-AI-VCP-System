/// JWT Authentication Middleware
///
/// This middleware enforces JWT validation globally for all protected routes.
/// Unlike extractor-based auth (AuthUser), this runs in the middleware layer
/// and rejects unauthorized requests before they reach handlers.

use crate::auth::AuthConfig;
use crate::error::ApiError;
use axum::{
    body::Body,
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use tracing::{debug, warn};

/// Extract and validate JWT token from Authorization header
pub async fn jwt_auth_middleware(
    headers: HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract Authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing authorization header");
            ApiError::unauthorized("Missing authorization header")
        })?;

    // Parse Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| {
            warn!("Invalid authorization header format");
            ApiError::unauthorized("Invalid authorization header format. Expected 'Bearer <token>'")
        })?;

    // Load auth config
    let auth_config = AuthConfig::from_env()?;

    // Validate token
    let claims = auth_config.validate_token(token).map_err(|e| {
        warn!("Token validation failed: {:?}", e);
        ApiError::unauthorized("Invalid or expired token")
    })?;

    debug!(
        "JWT validated for user: {} (role: {})",
        claims.username, claims.role
    );

    // Store validated claims in request extensions for handlers
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Extract TypedHeader-based auth (alternative implementation)
pub async fn jwt_auth_typed_middleware(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let token = auth.token();

    // Load auth config
    let auth_config = AuthConfig::from_env()?;

    // Validate token
    let claims = auth_config.validate_token(token).map_err(|e| {
        warn!("Token validation failed: {:?}", e);
        ApiError::unauthorized("Invalid or expired token")
    })?;

    debug!(
        "JWT validated for user: {} (role: {})",
        claims.username, claims.role
    );

    // Store validated claims in request extensions
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Inject AuthConfig into request extensions (for public routes)
pub async fn auth_config_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let auth_config = AuthConfig::from_env()?;
    request.extensions_mut().insert(auth_config);
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Claims;
    use axum::{body::Body, http::Request};
    
    #[tokio::test]
    async fn test_missing_auth_header() {
        let headers = HeaderMap::new();
        let request = Request::builder().body(Body::empty()).unwrap();
        
        // Create a dummy next handler
        let next = |_req: Request<Body>| async {
            Response::builder()
                .status(StatusCode::OK)
                .body(Body::empty())
                .unwrap()
        };
        
        let result = jwt_auth_middleware(
            headers,
            request,
            Next::new(Box::pin(next)),
        )
        .await;
        
        assert!(result.is_err());
    }
}
