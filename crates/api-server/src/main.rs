use anyhow::Result;
use api_server::{create_router, db, rate_limit, state::AppState};
use std::sync::Arc;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting Ambient AI VCP API Server");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Validate secret configuration before startup proceeds in production
    api_server::auth::validate_hash_pepper_configuration()?;

    // Initialize database connection
    let db_config = db::DatabaseConfig::from_env()?;
    let pool = db::create_pool(&db_config).await?;

    // Run database migrations
    db::run_migrations(&pool).await?;

    // Verify database connection
    db::health_check(&pool).await?;
    info!("Database connection established and verified");

    // Start rate limiter cleanup task
    rate_limit::start_cleanup_task().await;
    info!("Rate limiter cleanup task started");

    // Get port from environment or use default
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = format!("0.0.0.0:{}", port);

    // Create application state
    let state = Arc::new(AppState::new(pool));

    // Create router
    let app = create_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("API Server listening on http://{}", addr);
    info!("Swagger UI available at http://{}/swagger-ui", addr);
    info!("API Documentation at http://{}/api-docs/openapi.json", addr);
    info!("Prometheus metrics at http://{}/metrics", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
