#[tokio::test]
async fn migration_sql_files_are_postgres_compatible() {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost:5432/ambient_vcp_ci".to_string()
    });

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("connect to postgres for migration smoke test");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations should apply successfully on PostgreSQL");
}
