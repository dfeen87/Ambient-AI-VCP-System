/// Database module for PostgreSQL persistence
/// 
/// This module provides a connection pool and database operations
/// for the Ambient AI VCP system. It uses sqlx for async database access.

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use anyhow::{Context, Result};

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections in the pool
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
}

impl DatabaseConfig {
    /// Create a new database configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL must be set")?;
        
        let max_connections = std::env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        
        let min_connections = std::env::var("DB_MIN_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);
        
        let connection_timeout = std::env::var("DB_CONNECTION_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        
        Ok(Self {
            url,
            max_connections,
            min_connections,
            connection_timeout,
        })
    }
}

/// Create a PostgreSQL connection pool
/// 
/// This establishes an async connection pool to PostgreSQL with the
/// specified configuration. The pool is designed for production use
/// with proper timeout and connection management.
pub async fn create_pool(config: &DatabaseConfig) -> Result<PgPool> {
    tracing::info!("Initializing database connection pool");
    tracing::debug!(
        "Database config - max_connections: {}, min_connections: {}, timeout: {}s",
        config.max_connections,
        config.min_connections,
        config.connection_timeout
    );
    
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.connection_timeout))
        .connect(&config.url)
        .await
        .context("Failed to create database connection pool")?;
    
    tracing::info!("Database connection pool established successfully");
    
    Ok(pool)
}

/// Run database migrations
/// 
/// This applies any pending database migrations to ensure the schema
/// is up to date. Should be run on application startup.
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    tracing::info!("Running database migrations");
    
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("Failed to run database migrations")?;
    
    tracing::info!("Database migrations completed successfully");
    
    Ok(())
}

/// Health check for database connection
/// 
/// Verifies that the database is accessible and responding
pub async fn health_check(pool: &PgPool) -> Result<()> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .context("Database health check failed")?;
    
    Ok(())
}
