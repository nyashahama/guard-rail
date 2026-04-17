#[tokio::test]
async fn test_create_tenant_and_api_key_persists_hash_only() {
    let database_url =
        std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let repo = guard_rail_engine::tenant::repository::TenantRepository::new(pool.clone());
    let unique_name = format!("acme_{}", uuid::Uuid::new_v4());
    let tenant = repo.create_tenant(&unique_name).await.unwrap();
    let issued = repo.create_api_key(tenant.id, "primary").await.unwrap();

    assert!(issued.raw_key.starts_with("grk_"));
    assert_ne!(issued.raw_key, issued.key_prefix);

    let row = sqlx::query("select key_hash, key_prefix from api_keys where id = $1")
        .bind(issued.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    let key_hash: String = sqlx::Row::get(&row, "key_hash");
    let key_prefix: String = sqlx::Row::get(&row, "key_prefix");
    assert_ne!(key_hash, issued.raw_key);
    assert_eq!(key_prefix, issued.key_prefix);
}