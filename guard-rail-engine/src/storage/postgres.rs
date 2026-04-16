use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn connect_pool(
    config: &crate::config::DatabaseConfig,
) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&config.url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn assert_schema_ready(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "select count(*) from information_schema.tables where table_name = 'execution_audit'",
    )
    .fetch_one(pool)
    .await
    .and_then(|count| {
        if count == 1 {
            Ok(())
        } else {
            Err(sqlx::Error::Protocol("execution_audit table missing".into()))
        }
    })
}