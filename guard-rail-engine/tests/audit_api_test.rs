use guard_rail_engine::execution::{ExecutionRecord, ExecutionVerdict};
use guard_rail_engine::storage::postgres::PostgresAuditStore;

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

async fn start_stage2_test_app() -> (
    String,
    guard_rail_engine::storage::postgres::PostgresAuditStore,
    tempfile::TempDir,
) {
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::query("truncate table execution_audit restart identity")
        .execute(&pool)
        .await
        .unwrap();

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
        route_config_hash: guard_rail_engine::audit::hash::hash_string(
            &std::fs::read_to_string(config_dir.join("routes.yaml")).unwrap_or_default(),
        ),
        policy_set_hash: guard_rail_engine::audit::hash::hash_string(
            &std::fs::read_to_string(policies_dir.join("policy.yaml")).unwrap_or_default(),
        ),
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

    (format!("http://{}", addr), store, tmp)
}

#[tokio::test]
async fn test_invalid_json_on_known_route_persists_rejected_audit_row() {
    let (base_url, store, _tmp) = start_stage2_test_app().await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/execute/test-route", base_url))
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

    let row = store
        .get_execution_by_id(&execution_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.verdict, "REJECTED");
    assert_eq!(row.rejection_reason.as_deref(), Some("invalid_json"));
}

#[tokio::test]
async fn test_unknown_route_is_not_persisted() {
    let (base_url, store, _tmp) = start_stage2_test_app().await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/execute/missing-route", base_url))
        .header("content-type", "application/json")
        .body(r#"{"value":1}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(store.count_executions().await.unwrap(), 0);
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
        sqlx::query("truncate table execution_audit restart identity")
            .execute(pool)
            .await
            .unwrap();
    }

    fn sample_blocked_record() -> ExecutionRecord {
        ExecutionRecord {
            execution_id: "GR-EXE-blocked-1".to_string(),
            execution_started_at: chrono::Utc::now(),
            route_id: "test-route".to_string(),
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
