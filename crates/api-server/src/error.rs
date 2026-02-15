//! Structured error handling with secure defaults
//!
//! This module provides comprehensive error types for the API with
//! security-focused error handling that prevents information leakage.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;

/// API Error type with structured error information
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiError {
    /// Error type identifier
    pub error: String,
    /// User-friendly error message
    pub message: String,
    /// HTTP status code
    #[serde(skip)]
    pub status_code: StatusCode,
}

impl ApiError {
    /// Create a new API error
    pub fn new(
        error: impl Into<String>,
        message: impl Into<String>,
        status_code: StatusCode,
    ) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            status_code,
        }
    }

    /// 400 Bad Request - Invalid request data
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("bad_request", message, StatusCode::BAD_REQUEST)
    }

    /// 401 Unauthorized - Authentication required
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new("unauthorized", message, StatusCode::UNAUTHORIZED)
    }

    /// 403 Forbidden - Insufficient permissions
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new("forbidden", message, StatusCode::FORBIDDEN)
    }

    /// 404 Not Found - Resource doesn't exist
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message, StatusCode::NOT_FOUND)
    }

    /// 409 Conflict - Resource already exists
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new("conflict", message, StatusCode::CONFLICT)
    }

    /// 422 Unprocessable Entity - Validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new(
            "validation_error",
            message,
            StatusCode::UNPROCESSABLE_ENTITY,
        )
    }

    /// 429 Too Many Requests - Rate limit exceeded
    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(
            "rate_limited",
            message,
            StatusCode::TOO_MANY_REQUESTS,
        )
    }

    /// 500 Internal Server Error - Generic server error
    /// NOTE: Use sparingly and avoid exposing internal details
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(
            "internal_error",
            message,
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    }

    /// 503 Service Unavailable - Service temporarily unavailable
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(
            "service_unavailable",
            message,
            StatusCode::SERVICE_UNAVAILABLE,
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code;
        (status, Json(self)).into_response()
    }
}

/// Convert anyhow errors to API errors with safe error handling
/// This prevents internal error details from leaking to users
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        // Log the full error internally for debugging
        tracing::error!("Internal error: {:?}", err);
        
        // Return a sanitized error to the user
        ApiError::internal_error("An internal error occurred. Please try again later.")
    }
}

/// Convert sqlx errors to API errors with safe error handling
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        // Log the full error for debugging
        tracing::error!("Database error: {:?}", err);
        
        match err {
            sqlx::Error::RowNotFound => {
                ApiError::not_found("The requested resource was not found")
            }
            sqlx::Error::Database(db_err) => {
                // Check for constraint violations (e.g., unique constraint)
                if let Some(constraint) = db_err.constraint() {
                    if constraint.contains("unique") || constraint.contains("pkey") {
                        return ApiError::conflict("A resource with this identifier already exists");
                    }
                }
                
                // For other database errors, return a generic message
                ApiError::internal_error("A database error occurred. Please try again later.")
            }
            sqlx::Error::PoolTimedOut => {
                ApiError::service_unavailable("The service is temporarily unavailable. Please try again later.")
            }
            _ => {
                // For all other database errors, return a generic message
                ApiError::internal_error("A database error occurred. Please try again later.")
            }
        }
    }
}

/// Convert JWT errors to API errors
impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        // Log the error for debugging
        tracing::warn!("JWT error: {:?}", err);
        
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                ApiError::unauthorized("Your session has expired. Please log in again.")
            }
            jsonwebtoken::errors::ErrorKind::InvalidToken => {
                ApiError::unauthorized("Invalid authentication token")
            }
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                ApiError::unauthorized("Invalid authentication token")
            }
            _ => {
                ApiError::unauthorized("Authentication failed")
            }
        }
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = Result<T, ApiError>;
