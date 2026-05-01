use guard_rail_engine::config::ReplayConfig;
use guard_rail_engine::policy::{Policy, PolicySet};
use guard_rail_engine::proxy::AppState;
use guard_rail_engine::replay::snapshot::{build_snapshot, build_snapshot_from_set};
use guard_rail_engine::routes::{Route, RouteAuthMode};
use guard_rail_engine::storage::postgres::PostgresAuditStore;
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

pub struct TestHarness {
    pub base_url: String,
    pub tenant_key: String,
    pub store: PostgresAuditStore,
    pub state: AppState,
    _db_guard: TestDatabaseGuard,
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

async fn start_stage4_test_app() -> TestHarness {
    let db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    reset_test_database(&pool).await;

    let store = PostgresAuditStore::new(pool, std::time::Duration::from_millis(250));
    let upstream = start_mock_upstream(200, r#"{"ok":true}"#).await;

    let routes = Arc::new(RwLock::new(
        guard_rail_engine::routes::RouteTable::from_routes(vec![
            guard_rail_engine::routes::Route::new(
                "test-route".into(),
                RouteAuthMode::Public,
                format!("{upstream}/blocked"),
                vec!["POST".into()],
                vec!["block-callbacks".into()],
            ),
            guard_rail_engine::routes::Route::new(
                "open-route".into(),
                RouteAuthMode::Public,
                format!("{upstream}/open"),
                vec!["POST".into()],
                vec![],
            ),
        ]),
    ));

    let policies = Arc::new(RwLock::new(PolicySet::from_policies(vec![
        guard_rail_engine::policy::Policy {
            name: "block-callbacks".into(),
            description: "Block callback URLs".into(),
            rules: vec![guard_rail_engine::policy::Rule {
                field: "$.callback".into(),
                condition: "domain_not_in".into(),
                values: vec!["*.safe.com".into()],
                value: None,
                pattern: None,
                max_bytes: None,
                action: "block".into(),
                severity: "critical".into(),
            }],
        },
    ])));

    let state = AppState {
        routes,
        policies,
        http_client: reqwest::Client::new(),
        audit_store: Some(store.clone()),
        audit_persistence_mode: guard_rail_engine::config::AuditPersistenceMode::BestEffort,
        metrics: None,
        lifecycle: guard_rail_engine::shutdown::LifecycleState::new(),
        readiness_probe_timeout_ms: 250,
        trace_header_name: "traceparent".to_string(),
        route_config_hash: "test".into(),
        policy_set_hash: "test".into(),
        admin_token: "test-admin".into(),
        tenant_repo: guard_rail_engine::tenant::repository::TenantRepository::new(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy("postgres://localhost:5432/test")
                .unwrap(),
        ),
        tenant_cache: guard_rail_engine::tenant::cache::TenantAuthCache::default(),
        rate_limiter: guard_rail_engine::auth::rate_limit::TenantRateLimiter::new(120, 30),
        replay: ReplayConfig {
            enabled: true,
            capture_request_headers: vec!["content-type".into(), "x-request-id".into()],
            capture_response_headers: vec!["content-type".into()],
            max_response_body_bytes: 65536,
        },
    };

    let app = guard_rail_engine::proxy::build_router(
        state.clone(),
        "test-admin".into(),
        1_048_576,
        &guard_rail_engine::config::ObservabilityConfig::default(),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    TestHarness {
        base_url,
        tenant_key: "test-tenant-key".into(),
        store,
        state,
        _db_guard: db_guard,
    }
}

#[test]
fn test_snapshot_hash_is_stable_for_equivalent_route_and_policy_state() {
    let route = Route::new(
        "payments".into(),
        RouteAuthMode::Public,
        "http://upstream/payments".into(),
        vec!["POST".into()],
        vec!["block-callbacks".into()],
    );

    let snapshot_a = build_snapshot(
        &route,
        &[Policy {
            name: "block-callbacks".into(),
            description: "".into(),
            rules: vec![],
        }],
    );

    let snapshot_b = build_snapshot(
        &route,
        &[Policy {
            name: "block-callbacks".into(),
            description: "".into(),
            rules: vec![],
        }],
    );

    assert_eq!(snapshot_a.snapshot_hash, snapshot_b.snapshot_hash);
}

#[test]
fn test_snapshot_builder_uses_only_route_referenced_policies() {
    let route = Route::new(
        "payments".into(),
        RouteAuthMode::Public,
        "http://upstream/payments".into(),
        vec!["POST".into()],
        vec!["policy-a".into()],
    );

    let set = PolicySet::from_policies(vec![
        Policy {
            name: "policy-a".into(),
            description: "".into(),
            rules: vec![],
        },
        Policy {
            name: "policy-b".into(),
            description: "".into(),
            rules: vec![],
        },
    ]);

    let snapshot = build_snapshot_from_set(&route, &set).unwrap();
    assert_eq!(snapshot.policies_definition.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_stage4_migration_creates_replay_tables() {
    let _db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
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

#[tokio::test]
async fn test_blocked_execution_persists_request_artifacts_without_response_artifacts() {
    let harness = start_stage4_test_app().await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/execute/test-route", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_key))
        .header("content-type", "application/json")
        .header("x-request-id", "req-123")
        .body(r#"{"callback":"https://evil.sh"}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 403);
    let execution_id = response.headers()["x-guardrail-execution-id"]
        .to_str()
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let artifact = harness
        .store
        .get_execution_artifacts(execution_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(artifact.response_status, None);
    assert_eq!(
        artifact
            .request_headers
            .get("x-request-id")
            .unwrap()
            .as_str()
            .unwrap(),
        "req-123"
    );
}

#[tokio::test]
async fn test_allowed_execution_persists_response_artifacts_and_strips_authorization() {
    let harness = start_stage4_test_app().await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/execute/open-route", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_key))
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let execution_id = response.headers()["x-guardrail-execution-id"]
        .to_str()
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let artifact = harness
        .store
        .get_execution_artifacts(execution_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(artifact.response_status, Some(200));
    assert!(artifact.response_body.as_deref().unwrap().contains("ok"));
    assert!(artifact.request_headers.get("authorization").is_none());
}
