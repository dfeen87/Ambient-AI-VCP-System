/// Test utilities for integration tests
/// 
/// This module provides helper functions for setting up test databases
/// and creating test fixtures.

use sqlx::PgPool;

/// Create a test database pool for integration tests
/// 
/// This requires TEST_DATABASE_URL to be set to a PostgreSQL test database.
/// For CI/CD, you can skip these tests by running: cargo test --lib
pub async fn create_test_pool() -> PgPool {
    // Check if TEST_DATABASE_URL is set for real integration tests
    if let Ok(db_url) = std::env::var("TEST_DATABASE_URL") {
        create_real_test_pool(&db_url).await
    } else {
        // Skip database tests if no test database is configured
        panic!(
            "TEST_DATABASE_URL not set. Database tests require a PostgreSQL test database.\n\
             Either:\n\
             1. Set TEST_DATABASE_URL to a test database (e.g., postgres://test:test@localhost/test_db)\n\
             2. Run only unit tests with: cargo test --lib"
        );
    }
}

/// Create a real PostgreSQL test pool
async fn create_real_test_pool(database_url: &str) -> PgPool {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations on test database");

    pool
}

/// Clean up test database after tests
pub async fn cleanup_test_db(pool: &PgPool) {
    // Clean up all test data
    let _ = sqlx::query("TRUNCATE TABLE task_assignments, tasks, nodes, users CASCADE")
        .execute(pool)
        .await;
}
