use std::sync::{Arc, OnceLock};

static TEST_DB_LOCK: OnceLock<Arc<tokio::sync::Mutex<()>>> = OnceLock::new();

struct TestDatabaseGuard {
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

impl TestDatabaseGuard {
    async fn acquire() -> Self {
        let lock = TEST_DB_LOCK
            .get_or_init(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone();
        Self {
            _guard: lock.lock_owned().await,
        }
    }
}

async fn reset_test_database(pool: &sqlx::PgPool) {
    sqlx::query(
        r#"
        truncate table
            replay_runs,
            execution_artifacts,
            policy_snapshots,
            execution_audit,
            tenant_routes,
            api_keys,
            tenants
        restart identity cascade
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_create_tenant_and_api_key_persists_hash_only() {
    let _db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
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

#[tokio::test]
async fn test_load_auth_cache_returns_only_active_keys_and_bindings() {
    let _db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
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
    let unique_route = format!("test-route-{}", uuid::Uuid::new_v4());
    repo.bind_route(&unique_route, tenant.id).await.unwrap();
    repo.revoke_api_key(issued.id, Some("rotated"))
        .await
        .unwrap();

    let snapshot = repo.load_auth_snapshot().await.unwrap();
    assert!(snapshot.route_bindings.contains_key(&unique_route));
    assert!(
        !snapshot.api_keys.values().any(|k| k.id == issued.id),
        "revoked key should not be in cache"
    );
}

fn write_file(dir: &std::path::Path, name: &str, contents: &str) {
    std::fs::write(dir.join(name), contents).unwrap();
}

async fn start_mock_upstream(status: u16, body: &'static str) -> String {
    let app = axum::Router::new().route(
        "/{*path}",
        axum::routing::any(move || async move {
            (
                axum::http::StatusCode::from_u16(status).unwrap(),
                body.to_string(),
            )
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

#[allow(dead_code)]
struct Stage3TestApp {
    base_url: String,
    store: guard_rail_engine::storage::postgres::PostgresAuditStore,
    tenant_a_id: uuid::Uuid,
    tenant_a_key_id: uuid::Uuid,
    tenant_a_key: String,
    #[allow(dead_code)]
    tenant_b_id: uuid::Uuid,
    tenant_b_key: String,
    _db_guard: TestDatabaseGuard,
}

impl Stage3TestApp {
    async fn admin_post(&self, path: &str, body: &str) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}{}", self.base_url, path))
            .header("authorization", "Bearer stage2-admin-token")
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .unwrap()
    }
}

async fn start_stage3_test_app(requests_per_minute: u32, burst: u32) -> Stage3TestApp {
    let db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    reset_test_database(&pool).await;

    let repo = guard_rail_engine::tenant::repository::TenantRepository::new(pool.clone());
    let tenant_a = repo.create_tenant("tenant-a").await.unwrap();
    let tenant_b = repo.create_tenant("tenant-b").await.unwrap();
    let key_a = repo.create_api_key(tenant_a.id, "primary-a").await.unwrap();
    let key_b = repo.create_api_key(tenant_b.id, "primary-b").await.unwrap();
    repo.bind_route("test-route", tenant_a.id).await.unwrap();
    repo.bind_route("tenant-b-route", tenant_b.id)
        .await
        .unwrap();

    let snapshot = repo.load_auth_snapshot().await.unwrap();
    let tenant_cache = guard_rail_engine::tenant::cache::TenantAuthCache::default();
    tenant_cache.replace(snapshot).await;

    let upstream = start_mock_upstream(200, "ok").await;
    let tmp = tempfile::TempDir::new().unwrap();
    write_file(
        tmp.path(),
        "routes.yaml",
        &format!(
            r#"
routes:
  - id: test-route
    auth_mode: tenant_bound
    upstream: {upstream}/tenant-a
    methods: [POST]
    policies: []
  - id: tenant-b-route
    auth_mode: tenant_bound
    upstream: {upstream}/tenant-b
    methods: [POST]
    policies: []
"#
        ),
    );
    let policies_dir = tmp.path().join("policies");
    std::fs::create_dir_all(&policies_dir).unwrap();
    write_file(&policies_dir, "policy.yaml", "policies: []\n");

    let routes =
        guard_rail_engine::routes::RouteTable::load(&tmp.path().join("routes.yaml")).unwrap();
    let policies = guard_rail_engine::policy::PolicySet::load_dir(&policies_dir).unwrap();
    let store = guard_rail_engine::storage::postgres::PostgresAuditStore::new(
        pool.clone(),
        std::time::Duration::from_millis(250),
    );

    let state = guard_rail_engine::proxy::AppState {
        routes: std::sync::Arc::new(tokio::sync::RwLock::new(routes)),
        policies: std::sync::Arc::new(tokio::sync::RwLock::new(policies)),
        http_client: reqwest::Client::new(),
        audit_store: Some(store.clone()),
        audit_persistence_mode: guard_rail_engine::config::AuditPersistenceMode::BestEffort,
        metrics: None,
        lifecycle: guard_rail_engine::shutdown::LifecycleState::new(),
        readiness_probe_timeout_ms: 250,
        trace_header_name: "traceparent".to_string(),
        route_config_hash: guard_rail_engine::audit::hash::hash_string("routes"),
        policy_set_hash: guard_rail_engine::audit::hash::hash_string("policies"),
        admin_token: "stage2-admin-token".to_string(),
        tenant_repo: repo.clone(),
        tenant_cache: tenant_cache.clone(),
        rate_limiter: guard_rail_engine::auth::rate_limit::TenantRateLimiter::new(
            requests_per_minute,
            burst,
        ),
        replay: guard_rail_engine::config::ReplayConfig::default(),
    };

    let app = guard_rail_engine::proxy::build_router(
        state,
        "stage2-admin-token".to_string(),
        1_048_576,
        &guard_rail_engine::config::ObservabilityConfig::default(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });

    Stage3TestApp {
        base_url: format!("http://{}", addr),
        store,
        tenant_a_id: tenant_a.id,
        tenant_a_key_id: key_a.id,
        tenant_a_key: key_a.raw_key,
        tenant_b_id: tenant_b.id,
        tenant_b_key: key_b.raw_key,
        _db_guard: db_guard,
    }
}

#[tokio::test]
async fn test_missing_api_key_returns_401_and_audits_event() {
    let harness = start_stage3_test_app(120, 30).await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/execute/test-route", harness.base_url))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let rows = harness
        .store
        .list_executions(
            guard_rail_engine::audit::api::AuditListQuery {
                tenant_id: None,
                route_id: None,
                verdict: None,
                from: None,
                to: None,
                limit: None,
                cursor: None,
                order: None,
            },
            guard_rail_engine::auth::context::AuditAccess::Admin,
        )
        .await
        .unwrap();
    assert_eq!(
        rows.items[0].auth_outcome.as_deref(),
        Some("missing_api_key")
    );
}

#[tokio::test]
async fn test_valid_key_for_other_tenant_route_returns_404() {
    let harness = start_stage3_test_app(120, 30).await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/execute/tenant-b-route", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_a_key))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_tenant_rate_limit_returns_429_without_blocking_other_tenant() {
    let harness = start_stage3_test_app(1, 1).await;

    let first = reqwest::Client::new()
        .post(format!("{}/v1/execute/test-route", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_a_key))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();
    let second = reqwest::Client::new()
        .post(format!("{}/v1/execute/test-route", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_a_key))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();
    let other = reqwest::Client::new()
        .post(format!("{}/v1/execute/tenant-b-route", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_b_key))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(first.status(), 200);
    assert_eq!(second.status(), 429);
    assert_eq!(other.status(), 200);
}

#[tokio::test]
async fn test_admin_can_create_tenant_issue_key_and_bind_route() {
    let harness = start_stage3_test_app(120, 30).await;

    let tenant = harness
        .admin_post("/v1/admin/tenants", r#"{"name":"acme"}"#)
        .await
        .json::<serde_json::Value>()
        .await
        .unwrap();

    let tenant_id = tenant["id"].as_str().unwrap();

    let key = harness
        .admin_post(
            &format!("/v1/admin/tenants/{tenant_id}/keys"),
            r#"{"name":"primary"}"#,
        )
        .await
        .json::<serde_json::Value>()
        .await
        .unwrap();

    harness
        .admin_post(
            &format!("/v1/admin/tenants/{tenant_id}/routes"),
            r#"{"route_id":"test-route"}"#,
        )
        .await;

    let raw_key = key["raw_key"].as_str().unwrap();
    let execute = reqwest::Client::new()
        .post(format!("{}/v1/execute/test-route", harness.base_url))
        .header("authorization", format!("Bearer {}", raw_key))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(execute.status(), 200);
}

#[tokio::test]
async fn test_revoked_key_stops_working_immediately_after_admin_write() {
    let harness = start_stage3_test_app(120, 30).await;

    let before = reqwest::Client::new()
        .post(format!("{}/v1/execute/test-route", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_a_key))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(before.status(), 200);

    harness
        .admin_post(
            &format!(
                "/v1/admin/tenants/{}/keys/{}/revoke",
                harness.tenant_a_id, harness.tenant_a_key_id
            ),
            r#"{"reason":"rotated"}"#,
        )
        .await;

    let after = reqwest::Client::new()
        .post(format!("{}/v1/execute/test-route", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_a_key))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(after.status(), 401);
}
