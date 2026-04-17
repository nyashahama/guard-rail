use guard_rail_engine::execution::{ExecutionRecord, ExecutionVerdict};
use guard_rail_engine::storage::postgres::PostgresAuditStore;
use std::sync::{Arc, OnceLock};
use tower::util::ServiceExt;

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

fn write_file(dir: &std::path::Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
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

struct Stage2TestHarness {
    base_url: String,
    store: guard_rail_engine::storage::postgres::PostgresAuditStore,
    _tmp: tempfile::TempDir,
    _db_guard: TestDatabaseGuard,
}

async fn start_stage2_test_app() -> Stage2TestHarness {
    let db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    reset_test_database(&pool).await;

    let store = guard_rail_engine::storage::postgres::PostgresAuditStore::new(
        pool,
        std::time::Duration::from_millis(250),
    );

    let upstream = start_mock_upstream(200, "ok").await;
    let tmp = tempfile::TempDir::new().unwrap();
    let config_dir = tmp.path();

    write_file(
        config_dir,
        "routes.yaml",
        &format!(
            r#"
routes:
  - id: test-route
    path: /v1/execute/test-route
    upstream: {upstream}/api/target
    methods: [POST]
    policies: [block-callbacks]
"#
        ),
    );

    let policies_dir = config_dir.join("policies");
    std::fs::create_dir_all(&policies_dir).unwrap();
    write_file(
        &policies_dir,
        "policy.yaml",
        r#"
policies:
  - name: block-callbacks
    rules:
      - field: "$.callback"
        condition: domain_not_in
        values: ["*.safe.com"]
        action: block
"#,
    );

    let route_table =
        guard_rail_engine::routes::RouteTable::load(&config_dir.join("routes.yaml")).unwrap();
    let policy_set = guard_rail_engine::policy::PolicySet::load_dir(&policies_dir).unwrap();
    let state = guard_rail_engine::proxy::AppState {
        routes: std::sync::Arc::new(tokio::sync::RwLock::new(route_table)),
        policies: std::sync::Arc::new(tokio::sync::RwLock::new(policy_set)),
        http_client: reqwest::Client::new(),
        audit_store: Some(store.clone()),
        metrics: None,
        lifecycle: guard_rail_engine::shutdown::LifecycleState::new(),
        readiness_probe_timeout_ms: 250,
        trace_header_name: "traceparent".to_string(),
        route_config_hash: guard_rail_engine::audit::hash::hash_string(
            &std::fs::read_to_string(config_dir.join("routes.yaml")).unwrap_or_default(),
        ),
        policy_set_hash: guard_rail_engine::audit::hash::hash_string(
            &std::fs::read_to_string(policies_dir.join("policy.yaml")).unwrap_or_default(),
        ),
        admin_token: "stage2-admin-token".to_string(),
        tenant_repo: guard_rail_engine::tenant::repository::TenantRepository::new(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy("postgres://localhost:5432/test")
                .unwrap(),
        ),
        tenant_cache: guard_rail_engine::tenant::cache::TenantAuthCache::default(),
        rate_limiter: guard_rail_engine::auth::rate_limit::TenantRateLimiter::new(120, 30),
        replay: guard_rail_engine::config::ReplayConfig::default(),
    };

    let app = axum::Router::new()
        .route(
            "/v1/execute/{route_id}",
            axum::routing::any(guard_rail_engine::proxy::handle_execute),
        )
        .with_state(state);

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

    Stage2TestHarness {
        base_url: format!("http://{}", addr),
        store,
        _tmp: tmp,
        _db_guard: db_guard,
    }
}

struct AuditRouterHarness {
    app: axum::Router,
    _db_guard: TestDatabaseGuard,
}

async fn build_test_router_with_audit_store() -> AuditRouterHarness {
    let db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    reset_test_database(&pool).await;

    let audit_store = guard_rail_engine::storage::postgres::PostgresAuditStore::new(
        pool,
        std::time::Duration::from_millis(250),
    );

    let dummy_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost:5432/test")
        .unwrap();
    let state = guard_rail_engine::proxy::AppState {
        routes: std::sync::Arc::new(tokio::sync::RwLock::new(
            guard_rail_engine::routes::RouteTable::load(std::path::Path::new(
                "./config/routes.yaml",
            ))
            .unwrap(),
        )),
        policies: std::sync::Arc::new(tokio::sync::RwLock::new(
            guard_rail_engine::policy::PolicySet::load_dir(std::path::Path::new(
                "./config/policies",
            ))
            .unwrap(),
        )),
        http_client: reqwest::Client::new(),
        audit_store: Some(audit_store),
        metrics: None,
        lifecycle: guard_rail_engine::shutdown::LifecycleState::new(),
        readiness_probe_timeout_ms: 250,
        trace_header_name: "traceparent".to_string(),
        route_config_hash: guard_rail_engine::audit::hash::hash_string("routes.yaml"),
        policy_set_hash: guard_rail_engine::audit::hash::hash_string("policies"),
        admin_token: "stage2-admin-token".to_string(),
        tenant_repo: guard_rail_engine::tenant::repository::TenantRepository::new(
            dummy_pool.clone(),
        ),
        tenant_cache: guard_rail_engine::tenant::cache::TenantAuthCache::default(),
        rate_limiter: guard_rail_engine::auth::rate_limit::TenantRateLimiter::new(120, 30),
        replay: guard_rail_engine::config::ReplayConfig::default(),
    };

    AuditRouterHarness {
        app: guard_rail_engine::proxy::build_router(
            state,
            "stage2-admin-token".to_string(),
            1_048_576,
            &guard_rail_engine::config::ObservabilityConfig::default(),
        ),
        _db_guard: db_guard,
    }
}

async fn build_test_router_with_seeded_audit_rows() -> AuditRouterHarness {
    let harness = build_test_router_with_audit_store().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    let store = guard_rail_engine::storage::postgres::PostgresAuditStore::new(
        pool,
        std::time::Duration::from_millis(250),
    );

    for execution_id in ["GR-EXE-1", "GR-EXE-2", "GR-EXE-3"] {
        let record = guard_rail_engine::execution::ExecutionRecord {
            execution_id: execution_id.to_string(),
            execution_started_at: chrono::Utc::now(),
            route_id: "test-route".to_string(),
            tenant_id: None,
            api_key_id: None,
            auth_outcome: None,
            upstream_url: Some("http://upstream.test/api".to_string()),
            method: "POST".to_string(),
            source_ip: "127.0.0.1".to_string(),
            content_type: Some("application/json".to_string()),
            user_agent: Some("seed-test".to_string()),
            had_authorization_header: false,
            request_size_bytes: 16,
            request_body_sha256: guard_rail_engine::audit::hash::hash_body(br#"{"ok":true}"#),
            verdict: guard_rail_engine::execution::ExecutionVerdict::Allowed,
            rejection_reason: None,
            matched_policy_name: None,
            matched_rule_field: None,
            matched_rule_condition: None,
            matched_rule_severity: None,
            violation_value_hash: None,
            violation_value_preview: None,
            upstream_status: Some(200),
            forward_error: None,
            latency_inspect_us: 10,
            latency_forward_ms: Some(4),
            latency_total_ms: 5,
            route_config_hash: "route-hash".to_string(),
            policy_set_hash: "policy-hash".to_string(),
        };
        store.insert_execution(&record).await.unwrap();
    }

    harness
}

#[tokio::test]
async fn test_invalid_json_on_known_route_persists_rejected_audit_row() {
    let harness = start_stage2_test_app().await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/execute/test-route", harness.base_url))
        .header("content-type", "application/json")
        .body("not valid json")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);

    let execution_id = response
        .headers()
        .get("x-guardrail-execution-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let row = harness
        .store
        .get_execution_by_id(&execution_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.verdict, "REJECTED");
    assert_eq!(row.rejection_reason.as_deref(), Some("invalid_json"));
}

#[tokio::test]
async fn test_unknown_route_is_not_persisted() {
    let harness = start_stage2_test_app().await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/execute/missing-route", harness.base_url))
        .header("content-type", "application/json")
        .body(r#"{"value":1}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(harness.store.count_executions().await.unwrap(), 0);
}

#[tokio::test]
async fn test_insert_execution_and_fetch_it_back() {
    async fn connect_test_pool(database_url: &str) -> sqlx::PgPool {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    async fn reset_execution_audit(pool: &sqlx::PgPool) {
        reset_test_database(pool).await;
    }

    fn sample_blocked_record() -> ExecutionRecord {
        ExecutionRecord {
            execution_id: "GR-EXE-blocked-1".to_string(),
            execution_started_at: chrono::Utc::now(),
            route_id: "test-route".to_string(),
            tenant_id: None,
            api_key_id: None,
            auth_outcome: None,
            upstream_url: Some("http://upstream.test/api".to_string()),
            method: "POST".to_string(),
            source_ip: "127.0.0.1".to_string(),
            content_type: Some("application/json".to_string()),
            user_agent: Some("integration-test".to_string()),
            had_authorization_header: false,
            request_size_bytes: 32,
            request_body_sha256: guard_rail_engine::audit::hash::hash_body(
                br#"{"callback":"https://evil.sh"}"#,
            ),
            verdict: ExecutionVerdict::Blocked,
            rejection_reason: None,
            matched_policy_name: Some("block-callbacks".to_string()),
            matched_rule_field: Some("$.callback".to_string()),
            matched_rule_condition: Some("domain_not_in".to_string()),
            matched_rule_severity: Some("critical".to_string()),
            violation_value_hash: Some(guard_rail_engine::audit::hash::hash_string(
                "https://evil.sh",
            )),
            violation_value_preview: Some("https://evil.sh".to_string()),
            upstream_status: None,
            forward_error: None,
            latency_inspect_us: 20,
            latency_forward_ms: None,
            latency_total_ms: 1,
            route_config_hash: "route-hash".to_string(),
            policy_set_hash: "policy-hash".to_string(),
        }
    }

    let _db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = connect_test_pool(&database_url).await;
    reset_execution_audit(&pool).await;

    let store = PostgresAuditStore::new(pool.clone(), std::time::Duration::from_millis(250));
    let record = sample_blocked_record();

    store.insert_execution(&record).await.unwrap();

    let row = store
        .get_execution_by_id("GR-EXE-blocked-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.route_id, "test-route");
    assert_eq!(row.verdict, "BLOCKED");
    assert!(row.previous_hash.is_none());
    assert!(!row.record_hash.is_empty());
}

#[tokio::test]
async fn test_stage3_migration_creates_tenant_tables() {
    let _db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPoolOptions::new()
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
          and table_name in ('tenants', 'api_keys', 'tenant_routes')
        order by table_name
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(tables, vec!["api_keys", "tenant_routes", "tenants"]);
}

#[tokio::test]
async fn test_audit_list_returns_newest_first() {
    let harness = build_test_router_with_seeded_audit_rows().await;

    let req = axum::http::Request::builder()
        .uri("/v1/audit/executions?limit=2")
        .header("authorization", "Bearer stage2-admin-token")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = harness.app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"][0]["execution_id"], "GR-EXE-3");
    assert_eq!(json["items"][1]["execution_id"], "GR-EXE-2");
}

async fn start_stage3_test_app(requests_per_minute: u32, burst: u32) -> Stage3TestHarness {
    use guard_rail_engine::auth::rate_limit::TenantRateLimiter;
    use guard_rail_engine::storage::postgres::PostgresAuditStore;
    use guard_rail_engine::tenant::cache::TenantAuthCache;
    use guard_rail_engine::tenant::repository::TenantRepository;

    let db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    reset_test_database(&pool).await;

    let repo = TenantRepository::new(pool.clone());
    let tenant_a = repo.create_tenant("tenant-a").await.unwrap();
    let tenant_b = repo.create_tenant("tenant-b").await.unwrap();
    let key_a = repo.create_api_key(tenant_a.id, "primary-a").await.unwrap();
    let key_b = repo.create_api_key(tenant_b.id, "primary-b").await.unwrap();
    repo.bind_route("test-route", tenant_a.id).await.unwrap();
    repo.bind_route("tenant-b-route", tenant_b.id)
        .await
        .unwrap();

    let snapshot = repo.load_auth_snapshot().await.unwrap();
    let tenant_cache = TenantAuthCache::default();
    tenant_cache.replace(snapshot).await;

    let app = axum::Router::new().route(
        "/{*path}",
        axum::routing::any(|| async move { (axum::http::StatusCode::OK, "ok") }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let upstream = format!("http://{}", addr);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let tmp = tempfile::TempDir::new().unwrap();
    write_file(
        tmp.path(),
        "routes.yaml",
        &format!(
            r#"
routes:
  - id: test-route
    path: /v1/execute/test-route
    upstream: {upstream}/tenant-a
    methods: [POST]
    policies: []
  - id: tenant-b-route
    path: /v1/execute/tenant-b-route
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
    let store = PostgresAuditStore::new(pool, std::time::Duration::from_millis(250));

    let state = guard_rail_engine::proxy::AppState {
        routes: std::sync::Arc::new(tokio::sync::RwLock::new(routes)),
        policies: std::sync::Arc::new(tokio::sync::RwLock::new(policies)),
        http_client: reqwest::Client::new(),
        audit_store: Some(store),
        metrics: None,
        lifecycle: guard_rail_engine::shutdown::LifecycleState::new(),
        readiness_probe_timeout_ms: 250,
        trace_header_name: "traceparent".to_string(),
        route_config_hash: guard_rail_engine::audit::hash::hash_string("routes"),
        policy_set_hash: guard_rail_engine::audit::hash::hash_string("policies"),
        admin_token: "stage2-admin-token".to_string(),
        tenant_repo: repo,
        tenant_cache,
        rate_limiter: TenantRateLimiter::new(requests_per_minute, burst),
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

    Stage3TestHarness {
        base_url: format!("http://{}", addr),
        tenant_a_id: tenant_a.id,
        tenant_a_key: key_a.raw_key,
        tenant_b_id: tenant_b.id,
        tenant_b_key: key_b.raw_key,
        _db_guard: db_guard,
    }
}

struct Stage3TestHarness {
    base_url: String,
    tenant_a_id: uuid::Uuid,
    tenant_a_key: String,
    tenant_b_id: uuid::Uuid,
    tenant_b_key: String,
    _db_guard: TestDatabaseGuard,
}

#[tokio::test]
async fn test_tenant_audit_list_returns_only_owned_rows() {
    #[allow(dead_code)]
    struct AuditListView {
        items: serde_json::Value,
    }

    impl AuditListView {
        #[allow(dead_code)]
        fn contains_auth_outcome(&self, needle: &str) -> bool {
            self.items
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["auth_outcome"] == needle)
        }
    }

    let harness = start_stage3_test_app(120, 30).await;

    reqwest::Client::new()
        .post(format!("{}/v1/execute/test-route", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_a_key))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();
    reqwest::Client::new()
        .post(format!("{}/v1/execute/tenant-b-route", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_b_key))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let response = reqwest::Client::new()
        .get(format!("{}/v1/audit/executions", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_a_key))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        json["items"][0]["tenant_id"].as_str().unwrap(),
        harness.tenant_a_id.to_string()
    );
}

#[tokio::test]
async fn test_tenant_audit_detail_for_other_tenant_returns_404() {
    let harness = start_stage3_test_app(120, 30).await;

    reqwest::Client::new()
        .post(format!("{}/v1/execute/tenant-b-route", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_b_key))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let admin_list: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "{}/v1/audit/executions?tenant_id={}",
            harness.base_url, harness.tenant_b_id
        ))
        .header("authorization", "Bearer stage2-admin-token")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tenant_b_execution_id = admin_list["items"][0]["execution_id"].as_str().unwrap();

    let response = reqwest::Client::new()
        .get(format!(
            "{}/v1/audit/executions/{}",
            harness.base_url, tenant_b_execution_id
        ))
        .header("authorization", format!("Bearer {}", harness.tenant_a_key))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}
