use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn test_stage4_migration_creates_replay_tables() {
    let database_url =
        std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let tables: Vec<String> = sqlx::query_scalar(
        r#"
        select table_name
        from information_schema.tables
        where table_schema = 'public'
          and table_name in ('policy_snapshots', 'execution_artifacts', 'replay_runs')
        order by table_name
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        tables,
        vec!["execution_artifacts", "policy_snapshots", "replay_runs"]
    );
}