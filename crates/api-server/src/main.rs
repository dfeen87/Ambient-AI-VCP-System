use anyhow::Result;
use api_server::{create_router, state::AppState};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Get port from environment or use default
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = format!("0.0.0.0:{}", port);

    info!("Starting Ambient AI VCP API Server");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Create application state
    let state = Arc::new(AppState::new());

    // Create router
    let app = create_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("API Server listening on http://{}", addr);
    info!("Swagger UI available at http://{}/swagger-ui", addr);
    info!("API Documentation at http://{}/api-docs/openapi.json", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
