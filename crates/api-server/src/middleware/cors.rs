/// CORS middleware with configurable origins
///
/// Replaces permissive CORS with production-ready configuration
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;
use tracing::info;

/// Create CORS layer from environment configuration
pub fn create_cors_layer() -> CorsLayer {
    let allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173".to_string());

    let is_production = std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .to_lowercase()
        == "production";

    if allowed_origins.contains("*") {
        panic!("CORS_ALLOWED_ORIGINS cannot contain wildcard (*)");
    }

    let origins: Vec<HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|origin| {
            let origin = origin.trim();
            if origin.is_empty() {
                return None;
            }
            origin.parse::<HeaderValue>().ok()
        })
        .collect();

    if origins.is_empty() {
        panic!("CORS_ALLOWED_ORIGINS must contain at least one explicit origin");
    }

    info!("Configuring CORS with {} allowed origins", origins.len());

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(3600))
}

/// Create permissive CORS layer (development only)
pub fn create_permissive_cors_layer() -> CorsLayer {
    let is_production = std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .to_lowercase()
        == "production";

    if is_production {
        panic!("Permissive CORS is not allowed in production mode");
    }

    info!("Using permissive CORS (development mode only)");
    CorsLayer::permissive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_configuration() {
        std::env::set_var("CORS_ALLOWED_ORIGINS", "https://example.com");
        let _layer = create_cors_layer();
    }

    #[test]
    #[should_panic(expected = "wildcard")]
    fn test_production_wildcard_rejected() {
        std::env::set_var("ENVIRONMENT", "production");
        std::env::set_var("CORS_ALLOWED_ORIGINS", "*");
        let _layer = create_cors_layer();
    }
}
