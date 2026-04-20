use axum::Router;
use reqwest::Client;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

async fn start_mock_upstream(status: u16, body: &'static str, delay_ms: u64) -> String {
    let app = Router::new().route(
        "/{*path}",
        axum::routing::any(move || async move {
            if delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
            (
                axum::http::StatusCode::from_u16(status).unwrap(),
                body.to_string(),
            )
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{}", addr)
}

fn write_file(dir: &std::path::Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

struct SmokeHarness {
    base_url: String,
    lifecycle: guard_rail_engine::shutdown::LifecycleState,
    metrics: Arc<guard_rail_engine::observability::metrics::Metrics>,
    _tmp: TempDir,
}

async fn start_harness(initial_ready: bool) -> SmokeHarness {
    start_harness_with_delay(initial_ready, 0).await
}

async fn start_slow_harness(initial_ready: bool) -> SmokeHarness {
    start_harness_with_delay(initial_ready, 250).await
}

async fn start_harness_with_delay(initial_ready: bool, delay_ms: u64) -> SmokeHarness {
    let upstream = start_mock_upstream(200, r#"{"ok":true}"#, delay_ms).await;
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path();

    write_file(
        config_dir,
        "routes.yaml",
        &format!(
            r#"
routes:
  - id: open-route
    auth_mode: public
    upstream: {upstream}/api/open
    methods: [POST]
    policies: []
    timeout_ms: 5000
"#
        ),
    );

    let policies_dir = config_dir.join("policies");
    std::fs::create_dir_all(&policies_dir).unwrap();
    write_file(&policies_dir, "policy.yaml", "policies: []\n");

    let routes =
        guard_rail_engine::routes::RouteTable::load(&config_dir.join("routes.yaml")).unwrap();
    let policies = guard_rail_engine::policy::PolicySet::load_dir(&policies_dir).unwrap();
    let metrics = Arc::new(guard_rail_engine::observability::metrics::Metrics::new().unwrap());
    let lifecycle = guard_rail_engine::shutdown::LifecycleState::new();

    if initial_ready {
        lifecycle.mark_ready().await;
        metrics.set_readiness(true);
        metrics.record_shutdown_transition("ready");
    }

    let state = guard_rail_engine::proxy::AppState {
        routes: Arc::new(RwLock::new(routes)),
        policies: Arc::new(RwLock::new(policies)),
        http_client: Client::new(),
        audit_store: None,
        metrics: Some(Arc::clone(&metrics)),
        lifecycle: lifecycle.clone(),
        readiness_probe_timeout_ms: 250,
        trace_header_name: "traceparent".to_string(),
        route_config_hash: guard_rail_engine::audit::hash::hash_string("routes"),
        policy_set_hash: guard_rail_engine::audit::hash::hash_string("policies"),
        admin_token: "stage5-admin-token".to_string(),
        tenant_repo: guard_rail_engine::tenant::repository::TenantRepository::new(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy("postgres://localhost:5432")
                .unwrap(),
        ),
        tenant_cache: guard_rail_engine::tenant::cache::TenantAuthCache::default(),
        rate_limiter: guard_rail_engine::auth::rate_limit::TenantRateLimiter::new(120, 30),
        replay: guard_rail_engine::config::ReplayConfig::default(),
    };

    let app = guard_rail_engine::proxy::build_router(
        state,
        "stage5-admin-token".to_string(),
        1_048_576,
        &guard_rail_engine::config::ObservabilityConfig::default(),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    SmokeHarness {
        base_url: format!("http://{}", addr),
        lifecycle,
        metrics,
        _tmp: tmp,
    }
}

#[tokio::test]
async fn test_ready_endpoint_returns_503_until_runtime_ready() {
    let harness = start_harness(false).await;

    let response = reqwest::get(format!("{}/ready", harness.base_url))
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);

    harness.lifecycle.mark_ready().await;
    harness.metrics.set_readiness(true);
    harness.metrics.record_shutdown_transition("ready");

    let response = reqwest::get(format!("{}/ready", harness.base_url))
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn test_metrics_endpoint_exposes_request_counter_after_execution() {
    let harness = start_harness(true).await;
    let client = Client::new();

    let response = client
        .post(format!("{}/v1/execute/open-route", harness.base_url))
        .header("content-type", "application/json")
        .body(r#"{"data":"hello"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let metrics = reqwest::get(format!("{}/metrics", harness.base_url))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert!(metrics.contains("guardrail_requests_total"));
    assert!(metrics.contains("route_id=\"open-route\""));
    assert!(metrics.contains("verdict=\"ALLOWED\""));
    assert!(metrics.contains("guardrail_readiness 1"));
}

#[tokio::test]
async fn test_health_stays_200_while_ready_returns_503_during_drain() {
    let harness = start_harness(true).await;

    harness.lifecycle.begin_drain().await;
    harness.metrics.set_readiness(false);
    harness.metrics.record_shutdown_transition("draining");

    let health = reqwest::get(format!("{}/health", harness.base_url))
        .await
        .unwrap();
    assert_eq!(health.status(), reqwest::StatusCode::OK);

    let ready = reqwest::get(format!("{}/ready", harness.base_url))
        .await
        .unwrap();
    assert_eq!(ready.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_sigterm_drains_inflight_request_before_exit() {
    let harness = start_slow_harness(true).await;
    let client = Client::new();
    let base_url = harness.base_url.clone();

    let in_flight = tokio::spawn(async move {
        client
            .post(format!("{}/v1/execute/open-route", base_url))
            .header("content-type", "application/json")
            .body(r#"{"data":"slow"}"#)
            .send()
            .await
            .unwrap()
            .status()
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    harness.lifecycle.begin_drain().await;
    harness.metrics.set_readiness(false);
    harness.metrics.record_shutdown_transition("draining");

    let status = in_flight.await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK);

    let ready = reqwest::get(format!("{}/ready", harness.base_url))
        .await
        .unwrap();
    assert_eq!(ready.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
}
