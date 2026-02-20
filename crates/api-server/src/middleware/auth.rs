/// Authentication and authorization middleware

use crate::auth::{hash_api_key, Claims};
use crate::error::ApiError;
use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderMap},
    middleware::Next,
    response::Response,
};
use sqlx::Row;
use tracing::{debug, warn};

/// Extract and validate JWT token from Authorization header.
pub async fn jwt_auth_middleware(
    headers: HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("Missing authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::unauthorized("Invalid authorization header format"))?;

    let state = request
        .extensions()
        .get::<std::sync::Arc<crate::state::AppState>>()
        .cloned()
        .ok_or_else(|| ApiError::internal_error("Application state missing from request"))?;

    let claims = state
        .auth_config()?
        .validate_token(token)
        .map_err(|_| ApiError::unauthorized("Invalid or expired token"))?;

    debug!(
        "JWT validated for user: {} (role: {})",
        claims.username, claims.role
    );

    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}

/// API key auth middleware.
/// Accepts `X-API-Key: <key>` and resolves the associated user and scopes.
pub async fn api_key_auth_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("Missing X-API-Key header"))?;

    let key_hash = hash_api_key(key);

    let state = request
        .extensions()
        .get::<std::sync::Arc<crate::state::AppState>>()
        .cloned()
        .ok_or_else(|| ApiError::internal_error("Application state missing from request"))?;

    let row = sqlx::query(
        r#"
        SELECT u.user_id, u.username, u.role, ak.scopes, ak.revoked_at, ak.expires_at
        FROM api_keys ak
        JOIN users u ON ak.user_id = u.user_id
        WHERE ak.key_hash = $1
        "#,
    )
    .bind(key_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::unauthorized("Invalid API key"))?;

    let revoked_at: Option<chrono::DateTime<chrono::Utc>> = row.get("revoked_at");
    if revoked_at.is_some() {
        return Err(ApiError::unauthorized("API key has been revoked"));
    }

    let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.get("expires_at");
    if let Some(exp) = expires_at {
        if exp < chrono::Utc::now() {
            return Err(ApiError::unauthorized("API key has expired"));
        }
    }

    let claims = Claims {
        sub: row.get::<uuid::Uuid, _>("user_id").to_string(),
        username: row.get("username"),
        role: row.get("role"),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
        iat: chrono::Utc::now().timestamp(),
    };

    let scopes: Vec<String> = row.try_get("scopes").unwrap_or_default();

    request.extensions_mut().insert(claims);
    request.extensions_mut().insert(ApiScopes(scopes));

    Ok(next.run(request).await)
}

/// Scoped permission set extracted from JWT/API key.
#[derive(Debug, Clone)]
pub struct ApiScopes(pub Vec<String>);

/// Require one of the configured roles for a route.
pub async fn require_admin_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    authorize_roles(&request, &["admin"])?;
    Ok(next.run(request).await)
}

fn authorize_roles(request: &Request<Body>, allowed: &[&str]) -> Result<(), ApiError> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| ApiError::unauthorized("Authentication required"))?;

    if allowed.contains(&claims.role.as_str()) {
        return Ok(());
    }

    warn!("RBAC deny user={} role={}", claims.username, claims.role);
    Err(ApiError::forbidden("Insufficient role permissions"))
}
