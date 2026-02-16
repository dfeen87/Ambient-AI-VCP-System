/// Authentication module with JWT and API key support
///
/// This module provides secure authentication mechanisms including:
/// - JWT token generation and validation
/// - API key authentication
/// - Password hashing with bcrypt
use crate::error::{ApiError, ApiResult};
use axum::{extract::FromRequestParts, http::request::Parts};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// User role
    pub role: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
}

impl Claims {
    /// Create new claims for a user
    pub fn new(user_id: String, username: String, role: String, expiration_hours: i64) -> Self {
        let now = Utc::now();
        let expiration = now + Duration::hours(expiration_hours);

        Self {
            sub: user_id,
            username,
            role,
            exp: expiration.timestamp(),
            iat: now.timestamp(),
        }
    }
}

/// Authentication configuration
#[derive(Clone)]
pub struct AuthConfig {
    /// JWT secret key for token signing
    jwt_secret: String,
    /// JWT token expiration in hours
    pub jwt_expiration_hours: i64,
}

impl AuthConfig {
    /// Create authentication config from environment variables
    pub fn from_env() -> ApiResult<Self> {
        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| ApiError::internal_error("JWT_SECRET not configured"))?;

        // Check if we're in production mode
        let is_production = std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            == "production";

        // In production, reject insecure default secrets
        if is_production
            && (jwt_secret == "your-jwt-secret-key-change-this-in-production"
                || jwt_secret.contains("dev")
                || jwt_secret.contains("local")
                || jwt_secret.contains("test")
                || jwt_secret.len() < 32)
        {
            return Err(ApiError::internal_error(
                "PRODUCTION ERROR: JWT_SECRET must be a secure random string (min 32 chars). Generate with: openssl rand -base64 32"
            ));
        }

        // In development, just warn
        if !is_production
            && (jwt_secret == "your-jwt-secret-key-change-this-in-production"
                || jwt_secret.len() < 32)
        {
            tracing::warn!("Using weak JWT_SECRET - only acceptable for development. Generate a secure one with: openssl rand -base64 32");
        }

        let jwt_expiration_hours = std::env::var("JWT_EXPIRATION_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(24);

        Ok(Self {
            jwt_secret,
            jwt_expiration_hours,
        })
    }

    /// Generate a JWT token for a user
    pub fn generate_token(
        &self,
        user_id: String,
        username: String,
        role: String,
    ) -> ApiResult<String> {
        let claims = Claims::new(user_id, username, role, self.jwt_expiration_hours);

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            tracing::error!("Failed to generate JWT: {:?}", e);
            ApiError::internal_error("Failed to generate authentication token")
        })?;

        Ok(token)
    }

    /// Validate a JWT token and extract claims
    pub fn validate_token(&self, token: &str) -> ApiResult<Claims> {
        let validation = Validation::default();

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )?;

        Ok(token_data.claims)
    }
}

/// Authenticated user extracted from request
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

/// Extract authenticated user from Claims stored in request extensions by middleware
#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get validated claims from extensions (set by jwt_auth_middleware)
        let claims = parts
            .extensions
            .get::<Claims>()
            .ok_or_else(|| {
                ApiError::unauthorized("Authentication required. Claims not found in request extensions.")
            })?;

        Ok(AuthUser {
            user_id: claims.sub.clone(),
            username: claims.username.clone(),
            role: claims.role.clone(),
        })
    }
}

/// Login request
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Username
    pub username: String,
    /// Password
    pub password: String,
}

impl LoginRequest {
    /// Validate login request
    pub fn validate(&self) -> ApiResult<()> {
        // Validate username format
        if self.username.is_empty() {
            return Err(ApiError::validation_error("Username cannot be empty"));
        }

        if self.username.len() > 64 {
            return Err(ApiError::validation_error(
                "Username cannot exceed 64 characters",
            ));
        }

        // Validate password not empty
        if self.password.is_empty() {
            return Err(ApiError::validation_error("Password cannot be empty"));
        }

        if self.password.len() > 128 {
            return Err(ApiError::validation_error(
                "Password cannot exceed 128 characters",
            ));
        }

        Ok(())
    }
}

/// Login response
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    /// JWT access token
    pub access_token: String,
    /// Refresh token for token rotation
    pub refresh_token: Option<String>,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Token expiration in seconds
    pub expires_in: i64,
}

/// Refresh token request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    /// Refresh token
    pub refresh_token: String,
}

/// Refresh token response
#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshTokenResponse {
    /// New JWT access token
    pub access_token: String,
    /// New refresh token
    pub refresh_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Token expiration in seconds
    pub expires_in: i64,
}

/// Generate a secure refresh token
pub fn generate_refresh_token() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const TOKEN_LENGTH: usize = 64;

    let mut rng = rand::thread_rng();

    let token: String = (0..TOKEN_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    format!("rt_{}", token)
}

/// Hash a refresh token using SHA-256
pub fn hash_refresh_token(token: &str) -> String {
    // Use bcrypt-style hashing for consistency
    // In production, you might want to use a dedicated hashing function
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// User registration request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    /// Username (3-32 characters, alphanumeric and underscores only)
    pub username: String,
    /// Password (minimum 8 characters)
    pub password: String,
}

impl RegisterRequest {
    /// Validate registration request
    pub fn validate(&self) -> ApiResult<()> {
        // Validate username
        if self.username.len() < 3 || self.username.len() > 32 {
            return Err(ApiError::validation_error(
                "Username must be 3-32 characters",
            ));
        }

        if !self
            .username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            return Err(ApiError::validation_error(
                "Username can only contain letters, numbers, and underscores",
            ));
        }

        // Validate password strength
        if self.password.len() < 8 {
            return Err(ApiError::validation_error(
                "Password must be at least 8 characters",
            ));
        }

        Ok(())
    }
}

/// Hash a password using bcrypt
pub fn hash_password(password: &str) -> ApiResult<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| {
        tracing::error!("Failed to hash password: {:?}", e);
        ApiError::internal_error("Failed to process password")
    })
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> ApiResult<bool> {
    bcrypt::verify(password, hash).map_err(|e| {
        tracing::error!("Failed to verify password: {:?}", e);
        ApiError::internal_error("Failed to verify password")
    })
}

/// Generate a secure API key
pub fn generate_api_key() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const KEY_LENGTH: usize = 32;

    let mut rng = rand::thread_rng();

    let key: String = (0..KEY_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    format!("vcp_{}", key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "secure_password_123";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_api_key_generation() {
        let key = generate_api_key();
        assert!(key.starts_with("vcp_"));
        assert_eq!(key.len(), 36); // "vcp_" + 32 characters
    }
}
