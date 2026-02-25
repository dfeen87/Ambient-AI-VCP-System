use anyhow::{Context, Result};
/// Database module for PostgreSQL persistence
///
/// This module provides a connection pool and database operations
/// for the Ambient AI VCP system. It uses sqlx for async database access.
use sqlx::{postgres::PgPoolOptions, Executor, PgPool};
use std::str::FromStr;
use std::time::Duration;

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
        let url = resolve_database_url().context(
            "No valid database connection URL found. Set DATABASE_URL or a Render-compatible fallback (DATABASE_INTERNAL_URL / POSTGRES_INTERNAL_URL / POSTGRES_URL), or provide PGHOST/PGPORT/PGUSER/PGPASSWORD/PGDATABASE",
        )?;

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

fn resolve_database_url() -> Option<String> {
    const CANDIDATE_KEYS: &[&str] = &[
        "DATABASE_URL",
        "DATABASE_INTERNAL_URL",
        "POSTGRES_INTERNAL_URL",
        "POSTGRES_URL",
    ];

    for key in CANDIDATE_KEYS {
        let raw = match std::env::var(key) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let candidate = raw.trim().trim_matches('"').trim_matches('\'').to_string();
        if candidate.is_empty() {
            continue;
        }

        if let Some(accepted) = validate_postgres_url(&candidate, key) {
            return Some(accepted);
        }
    }

    if let Some(from_pg_parts) = build_url_from_pg_parts() {
        if let Some(accepted) = validate_postgres_url(&from_pg_parts, "PG*") {
            return Some(accepted);
        }
    }

    None
}

fn build_url_from_pg_parts() -> Option<String> {
    let host = clean_env("PGHOST")?;
    let user = clean_env("PGUSER")?;
    let password = clean_env("PGPASSWORD")?;
    let database = clean_env("PGDATABASE")?;
    let port = clean_env("PGPORT").unwrap_or_else(|| "5432".to_string());

    let parts = [
        ("PGHOST", host.as_str()),
        ("PGUSER", user.as_str()),
        ("PGPASSWORD", password.as_str()),
        ("PGDATABASE", database.as_str()),
        ("PGPORT", port.as_str()),
    ];

    for (key, value) in parts {
        if has_unresolved_template(value) {
            tracing::warn!(
                env_key = %key,
                value = %value,
                "Ignoring PG* database env because it appears to contain an unresolved template placeholder"
            );
            return None;
        }
    }

    tracing::info!(
        host = %host,
        db = %database,
        "Constructed database URL from PG* environment variables"
    );
    Some(format!(
        "postgres://{}:{}@{}:{}/{}",
        user, password, host, port, database
    ))
}

fn clean_env(key: &str) -> Option<String> {
    let raw = std::env::var(key).ok()?;
    let cleaned = raw.trim().trim_matches('"').trim_matches('\'').to_string();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn has_unresolved_template(value: &str) -> bool {
    value.contains('$') || value.contains('{') || value.contains('}')
}

fn validate_postgres_url(candidate: &str, source: &str) -> Option<String> {
    let Ok(connect_options) = sqlx::postgres::PgConnectOptions::from_str(candidate) else {
        tracing::warn!(
            env_key = %source,
            "Ignoring configured database URL because it is not a valid PostgreSQL connection string"
        );
        return None;
    };

    let host = connect_options.get_host();
    if has_unresolved_template(host) {
        tracing::warn!(
            env_key = %source,
            host = %host,
            "Ignoring configured database URL because hostname appears to contain an unresolved template placeholder"
        );
        return None;
    }

    tracing::info!(env_key = %source, host = %host, "Using database connection URL");
    Some(candidate.to_string())
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

    pool.execute("CREATE EXTENSION IF NOT EXISTS pgcrypto")
        .await
        .context("Failed to ensure pgcrypto extension exists")?;

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
