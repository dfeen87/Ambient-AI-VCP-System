/// Authentication module with JWT and API key support
///
/// This module provides secure authentication mechanisms including:
/// - JWT token generation and validation
/// - API key authentication
/// - Password hashing with bcrypt
use crate::error::{ApiError, ApiResult};
use axum::{extract::FromRequestParts, http::request::Parts};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Once;
use utoipa::ToSchema;

type HmacSha256 = Hmac<Sha256>;

const DEV_REFRESH_TOKEN_PEPPER: &str = "dev-refresh-pepper-change-me";
const DEV_API_KEY_PEPPER: &str = "dev-api-key-pepper-change-me";
const DEV_CONNECT_SESSION_TOKEN_PEPPER: &str = "dev-connect-session-pepper-change-me";
const MIN_PEPPER_LENGTH: usize = 32;
static REFRESH_TOKEN_PEPPER_WARNING: Once = Once::new();
static API_KEY_PEPPER_WARNING: Once = Once::new();
static CONNECT_SESSION_TOKEN_PEPPER_WARNING: Once = Once::new();

fn warn_missing_pepper_once(label: &str, primary_var: &str) {
    let warning = || {
        tracing::warn!(
            "{} is not configured; using development-only fallback pepper. Set {} (or AUTH_HASH_PEPPER) to silence this warning.",
            label,
            primary_var
        );
    };

    match label {
        "REFRESH_TOKEN_PEPPER" => REFRESH_TOKEN_PEPPER_WARNING.call_once(warning),
        "API_KEY_PEPPER" => API_KEY_PEPPER_WARNING.call_once(warning),
        "CONNECT_SESSION_TOKEN_PEPPER" => CONNECT_SESSION_TOKEN_PEPPER_WARNING.call_once(warning),
        _ => warning(),
    }
}

fn is_production_environment() -> bool {
    std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .to_lowercase()
        == "production"
}

fn pepper_is_weak(value: &str) -> bool {
    let normalized = value.to_lowercase();
    value.len() < MIN_PEPPER_LENGTH
        || normalized.contains("change-me")
        || normalized.contains("dev")
        || normalized.contains("test")
        || normalized.contains("local")
}

fn read_pepper(primary_var: &str) -> Option<String> {
    std::env::var(primary_var)
        .ok()
        .or_else(|| std::env::var("AUTH_HASH_PEPPER").ok())
}

fn resolve_hash_pepper(primary_var: &str, dev_default: &str, label: &str) -> String {
    if let Some(pepper) = read_pepper(primary_var) {
        if is_production_environment() && pepper_is_weak(&pepper) {
            panic!(
                "PRODUCTION ERROR: {label} is weak. Configure {primary_var} (or AUTH_HASH_PEPPER) with a strong random value (min {MIN_PEPPER_LENGTH} chars)."
            );
        }

        return pepper;
    }

    if is_production_environment() {
        panic!(
            "PRODUCTION ERROR: {label} is not configured. Set {primary_var} (or AUTH_HASH_PEPPER)."
        );
    }

    warn_missing_pepper_once(label, primary_var);
    dev_default.to_string()
}

pub fn validate_hash_pepper_configuration() -> ApiResult<()> {
    if !is_production_environment() {
        return Ok(());
    }

    let refresh_pepper = read_pepper("REFRESH_TOKEN_PEPPER").ok_or_else(|| {
        ApiError::internal_error(
            "PRODUCTION ERROR: REFRESH_TOKEN_PEPPER (or AUTH_HASH_PEPPER) must be configured.",
        )
    })?;

    if pepper_is_weak(&refresh_pepper) {
        return Err(ApiError::internal_error(
            "PRODUCTION ERROR: REFRESH_TOKEN_PEPPER (or AUTH_HASH_PEPPER) is weak. Use a random secret with at least 32 characters.",
        ));
    }

    let api_key_pepper = read_pepper("API_KEY_PEPPER").ok_or_else(|| {
        ApiError::internal_error(
            "PRODUCTION ERROR: API_KEY_PEPPER (or AUTH_HASH_PEPPER) must be configured.",
        )
    })?;

    if pepper_is_weak(&api_key_pepper) {
        return Err(ApiError::internal_error(
            "PRODUCTION ERROR: API_KEY_PEPPER (or AUTH_HASH_PEPPER) is weak. Use a random secret with at least 32 characters.",
        ));
    }

    let connect_session_pepper = read_pepper("CONNECT_SESSION_TOKEN_PEPPER").ok_or_else(|| {
        ApiError::internal_error(
            "PRODUCTION ERROR: CONNECT_SESSION_TOKEN_PEPPER (or AUTH_HASH_PEPPER) must be configured.",
        )
    })?;

    if pepper_is_weak(&connect_session_pepper) {
        return Err(ApiError::internal_error(
            "PRODUCTION ERROR: CONNECT_SESSION_TOKEN_PEPPER (or AUTH_HASH_PEPPER) is weak. Use a random secret with at least 32 characters.",
        ));
    }

    Ok(())
}

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
        let is_production = is_production_environment();

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
        let claims = parts.extensions.get::<Claims>().ok_or_else(|| {
            ApiError::unauthorized(
                "Authentication required. Claims not found in request extensions.",
            )
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
    let pepper = resolve_hash_pepper(
        "REFRESH_TOKEN_PEPPER",
        DEV_REFRESH_TOKEN_PEPPER,
        "REFRESH_TOKEN_PEPPER",
    );

    hash_with_hmac_sha256(token, &pepper)
}

/// Hash API keys for storage and lookup.
pub fn hash_api_key(key: &str) -> String {
    let pepper = resolve_hash_pepper("API_KEY_PEPPER", DEV_API_KEY_PEPPER, "API_KEY_PEPPER");
    hash_with_hmac_sha256(key, &pepper)
}

/// Generate a secure connect session token for relay/tunnel setup.
pub fn generate_connect_session_token() -> String {
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

    format!("cs_{}", token)
}

/// Hash connect session token for storage and lookup.
pub fn hash_connect_session_token(token: &str) -> String {
    let pepper = resolve_hash_pepper(
        "CONNECT_SESSION_TOKEN_PEPPER",
        DEV_CONNECT_SESSION_TOKEN_PEPPER,
        "CONNECT_SESSION_TOKEN_PEPPER",
    );

    hash_with_hmac_sha256(token, &pepper)
}

fn hash_with_hmac_sha256(input: &str, pepper: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(pepper.as_bytes())
        .expect("HMAC accepts keys of any size for SHA256");
    mac.update(input.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// User registration request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    /// Username (3-32 characters, alphanumeric and underscores only)
    pub username: String,
    /// Password (minimum 8 characters)
    pub password: String,
    /// Optional email address for task completion notifications
    pub email: Option<String>,
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

        if let Some(email) = &self.email {
            let trimmed = email.trim();
            if trimmed.is_empty() {
                return Err(ApiError::validation_error(
                    "Email cannot be empty when provided",
                ));
            }

            if trimmed.len() > 255 {
                return Err(ApiError::validation_error(
                    "Email cannot exceed 255 characters",
                ));
            }

            if trimmed.contains('\r') || trimmed.contains('\n') {
                return Err(ApiError::validation_error(
                    "Email cannot contain carriage return or newline characters",
                ));
            }

            if !trimmed.contains('@') {
                return Err(ApiError::validation_error("Email must be a valid address"));
            }
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

/// Verify a password using a blocking thread pool.
pub async fn verify_password_async(password: String, hash: String) -> ApiResult<bool> {
    tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|_| ApiError::internal_error("Password verification task failed"))?
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
    use std::sync::{Mutex, OnceLock};

    fn env_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, original }
        }

        fn remove(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                std::env::set_var(self.key, value);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

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

    #[test]
    fn test_connect_session_token_generation_and_hashing() {
        let _guard = env_test_lock().lock().unwrap();
        let _pepper = EnvVarGuard::set(
            "CONNECT_SESSION_TOKEN_PEPPER",
            "StrongConnectSessionPepperValue1234567890AB!",
        );
        let _environment = EnvVarGuard::set("ENVIRONMENT", "development");

        let token = generate_connect_session_token();
        assert!(token.starts_with("cs_"));

        let h1 = hash_connect_session_token(&token);
        let h2 = hash_connect_session_token(&token);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert_ne!(h1, token);
    }

    #[test]
    fn test_refresh_hash_stable_and_non_plaintext() {
        let _guard = env_test_lock().lock().unwrap();
        let _refresh = EnvVarGuard::set(
            "REFRESH_TOKEN_PEPPER",
            "StrongRefreshPepperValue1234567890ABCD!",
        );
        let _environment = EnvVarGuard::set("ENVIRONMENT", "development");

        let h1 = hash_refresh_token("token-value");
        let h2 = hash_refresh_token("token-value");
        assert_eq!(h1, h2);
        assert_ne!(h1, "token-value");
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_production_startup_guard_rejects_missing_peppers() {
        let _guard = env_test_lock().lock().unwrap();
        let _environment = EnvVarGuard::set("ENVIRONMENT", "production");
        let _refresh = EnvVarGuard::remove("REFRESH_TOKEN_PEPPER");
        let _api = EnvVarGuard::remove("API_KEY_PEPPER");
        let _shared = EnvVarGuard::remove("AUTH_HASH_PEPPER");

        let result = validate_hash_pepper_configuration();
        assert!(result.is_err());
    }

    #[test]
    fn test_production_startup_guard_accepts_strong_shared_pepper() {
        let _guard = env_test_lock().lock().unwrap();
        let _environment = EnvVarGuard::set("ENVIRONMENT", "production");
        let _refresh = EnvVarGuard::remove("REFRESH_TOKEN_PEPPER");
        let _api = EnvVarGuard::remove("API_KEY_PEPPER");
        let _shared = EnvVarGuard::set(
            "AUTH_HASH_PEPPER",
            "ThisIsAStrongSharedPepperValueForProd123!",
        );

        let result = validate_hash_pepper_configuration();
        assert!(result.is_ok());
    }
}
