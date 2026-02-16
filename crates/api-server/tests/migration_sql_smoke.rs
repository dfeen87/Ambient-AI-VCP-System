#[tokio::test]
async fn migration_sql_files_are_postgres_compatible() {
    let db_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("Skipping migration smoke test — no DATABASE_URL set");
            return;
        }
    };

    let pool = sqlx::PgPool::connect(&db_url)
        .await
        .expect("connect to postgres for migration smoke test");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations should apply successfully on PostgreSQL");
}
