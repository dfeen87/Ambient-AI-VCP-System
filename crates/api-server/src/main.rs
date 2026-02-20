use anyhow::Result;
use api_server::{create_router, db, rate_limit, state::AppState};
use std::sync::Arc;
use std::time::Duration;
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
    let auth_config = api_server::auth::AuthConfig::from_env()?;
    let state = Arc::new(AppState::new(pool).with_auth_config(auth_config));

    let monitor_interval_seconds = AppState::connect_session_monitor_interval_seconds();
    let monitor_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(monitor_interval_seconds));
        loop {
            ticker.tick().await;
            match monitor_state.sweep_connect_sessions().await {
                Ok(swept) if swept > 0 => {
                    info!(swept, "Connect session monitor swept stale sessions");
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::error!("Connect session monitor sweep failed: {err}");
                }
            }
        }
    });
    info!(
        monitor_interval_seconds,
        "Connect session monitor task started"
    );

    // Start node offline sweep — marks nodes as offline when they have not
    // sent a heartbeat within NODE_HEARTBEAT_TIMEOUT_MINUTES (default: 5).
    let node_sweep_interval_seconds: u64 = std::env::var("NODE_OFFLINE_SWEEP_INTERVAL_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|v: &u64| *v > 0)
        .unwrap_or(60);
    let node_sweep_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(node_sweep_interval_seconds));
        loop {
            ticker.tick().await;
            match node_sweep_state.sweep_offline_nodes().await {
                Ok(swept) if swept > 0 => {
                    info!(swept, "Node offline sweep marked nodes offline");
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::error!("Node offline sweep failed: {err}");
                }
            }
        }
    });
    info!(
        node_sweep_interval_seconds,
        "Node offline sweep task started"
    );

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
