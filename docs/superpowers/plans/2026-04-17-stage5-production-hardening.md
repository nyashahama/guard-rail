# Guard Rail Backend Stage 5: Production Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add readiness, metrics, tracing-compatible structured logs, graceful shutdown, and deployment artifacts to the Stage 4 backend so it can be operated safely as a single-instance production service.

**Architecture:** Keep the existing single `axum` binary, proxy flow, Stage 2 audit ledger, Stage 3 tenant isolation, and Stage 4 replay features. Add an additive lifecycle layer in `shutdown.rs`, an additive observability layer in `src/observability/`, and wire those into `main.rs`, `proxy/mod.rs`, and `logging.rs` so operational visibility and drain behavior stay separate from policy and forwarding logic.

**Tech Stack:** Rust, axum, tokio, tracing, tracing-subscriber, OpenTelemetry-compatible tracing context, Prometheus text metrics, PostgreSQL, sqlx, Docker, systemd

---

## File Structure

```
guard-rail-engine/
  .dockerignore                                      — keep builds small and deterministic
  Dockerfile                                         — multi-stage runtime image
  deploy/
    systemd/
      guard-rail-engine.service                      — systemd service definition for single-instance deployment
  config/
    config.yaml                                      — add observability and shutdown defaults
  src/
    lib.rs                                           — export observability and shutdown modules
    config.rs                                        — add observability/shutdown config + env overrides
    logging.rs                                       — emit execution logs through tracing events
    main.rs                                          — initialize observability, lifecycle state, readiness, and graceful shutdown
    proxy/
      mod.rs                                         — instrument request lifecycle and add /ready + /metrics routes
    observability/
      mod.rs                                         — shared observability state and route helpers
      metrics.rs                                     — Prometheus registry, counters, gauges, histograms
      tracing.rs                                     — tracing subscriber init and request trace context helpers
    shutdown.rs                                      — lifecycle state, inflight request tracker, shutdown signal future
  tests/
    smoke_test.rs                                    — startup, readiness, metrics, and graceful shutdown coverage
README.md                                            — add backend operations and deployment instructions
```

## Task 1: Add Stage 5 Config And Lifecycle Primitives

**Files:**
- Modify: `guard-rail-engine/src/config.rs`
- Modify: `guard-rail-engine/config/config.yaml`
- Modify: `guard-rail-engine/src/lib.rs`
- Create: `guard-rail-engine/src/shutdown.rs`
- Test: `guard-rail-engine/src/config.rs`
- Test: `guard-rail-engine/src/shutdown.rs`

- [ ] **Step 1: Write the failing config and lifecycle tests**

```rust
#[test]
fn test_load_config_with_stage5_sections() {
    let yaml = r#"
server:
  host: "127.0.0.1"
  port: 9090
routes_file: "./routes.yaml"
policies_dir: "./policies/"
forwarding: {}
logging: {}
database:
  url: "postgres://guardrail:secret@localhost:5432/guardrail"
audit: {}
admin:
  token: "stage-admin-token"
tenant_auth: {}
rate_limit: {}
replay: {}
observability:
  service_name: "guard-rail-engine"
  metrics_enabled: true
  metrics_path: "/metrics"
  trace_header_name: "traceparent"
  readiness_probe_timeout_ms: 250
shutdown:
  grace_period_ms: 15000
  drain_poll_interval_ms: 50
"#;

    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    tmp.write_all(yaml.as_bytes()).unwrap();

    let config = crate::config::AppConfig::load(tmp.path()).unwrap();
    assert_eq!(config.observability.metrics_path, "/metrics");
    assert_eq!(config.shutdown.grace_period_ms, 15_000);
}

#[tokio::test]
async fn test_lifecycle_state_transitions_affect_readiness() {
    let lifecycle = crate::shutdown::LifecycleState::new();

    assert!(!lifecycle.is_ready().await);

    lifecycle.mark_ready().await;
    assert!(lifecycle.is_ready().await);

    lifecycle.begin_drain().await;
    assert!(!lifecycle.is_ready().await);
    assert_eq!(lifecycle.current().await.as_str(), "draining");
}
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_load_config_with_stage5_sections --lib`
Expected: FAIL because `AppConfig` does not expose `observability` or `shutdown`.

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_lifecycle_state_transitions_affect_readiness --lib`
Expected: FAIL because `shutdown.rs` and `LifecycleState` do not exist yet.

- [ ] **Step 3: Add the config surface and lifecycle module**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub routes_file: String,
    pub policies_dir: String,
    pub forwarding: ForwardingConfig,
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
    pub audit: AuditConfig,
    pub admin: AdminConfig,
    pub tenant_auth: TenantAuthConfig,
    pub rate_limit: RateLimitConfig,
    pub replay: ReplayConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub shutdown: ShutdownConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ObservabilityConfig {
    #[serde(default = "default_service_name")]
    pub service_name: String,
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,
    #[serde(default = "default_metrics_path")]
    pub metrics_path: String,
    #[serde(default = "default_trace_header_name")]
    pub trace_header_name: String,
    #[serde(default = "default_readiness_probe_timeout_ms")]
    pub readiness_probe_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ShutdownConfig {
    #[serde(default = "default_grace_period_ms")]
    pub grace_period_ms: u64,
    #[serde(default = "default_drain_poll_interval_ms")]
    pub drain_poll_interval_ms: u64,
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePhase {
    Starting,
    Ready,
    Draining,
    Stopped,
}

#[derive(Clone)]
pub struct LifecycleState {
    inner: std::sync::Arc<tokio::sync::RwLock<RuntimePhase>>,
}

impl LifecycleState {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::RwLock::new(RuntimePhase::Starting)),
        }
    }

    pub async fn mark_ready(&self) {
        *self.inner.write().await = RuntimePhase::Ready;
    }

    pub async fn begin_drain(&self) {
        *self.inner.write().await = RuntimePhase::Draining;
    }

    pub async fn current(&self) -> RuntimePhase {
        *self.inner.read().await
    }

    pub async fn is_ready(&self) -> bool {
        matches!(self.current().await, RuntimePhase::Ready)
    }
}

impl RuntimePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimePhase::Starting => "starting",
            RuntimePhase::Ready => "ready",
            RuntimePhase::Draining => "draining",
            RuntimePhase::Stopped => "stopped",
        }
    }
}
```

```yaml
observability:
  service_name: "guard-rail-engine"
  metrics_enabled: true
  metrics_path: "/metrics"
  trace_header_name: "traceparent"
  readiness_probe_timeout_ms: 250

shutdown:
  grace_period_ms: 15000
  drain_poll_interval_ms: 50
```

```rust
pub mod observability;
pub mod shutdown;
```

- [ ] **Step 4: Run the focused tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_load_config_with_stage5_sections --lib`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_lifecycle_state_transitions_affect_readiness --lib`
Expected: PASS

- [ ] **Step 5: Commit the Stage 5 runtime primitives**

```bash
git add guard-rail-engine/src/config.rs guard-rail-engine/config/config.yaml guard-rail-engine/src/lib.rs guard-rail-engine/src/shutdown.rs
git commit -m "feat: add stage5 lifecycle config"
```

## Task 2: Add Metrics Registry And Tracing Initialization

**Files:**
- Modify: `guard-rail-engine/Cargo.toml`
- Create: `guard-rail-engine/src/observability/mod.rs`
- Create: `guard-rail-engine/src/observability/metrics.rs`
- Create: `guard-rail-engine/src/observability/tracing.rs`
- Modify: `guard-rail-engine/src/logging.rs`
- Test: `guard-rail-engine/src/observability/metrics.rs`
- Test: `guard-rail-engine/src/logging.rs`

- [ ] **Step 1: Write the failing metrics and logging tests**

```rust
#[test]
fn test_metrics_snapshot_contains_request_and_readiness_series() {
    let metrics = crate::observability::metrics::Metrics::new();
    metrics.set_readiness(true);
    metrics.record_execution("test-route", "POST", "ALLOWED", 0.012);

    let snapshot = metrics.render().unwrap();
    assert!(snapshot.contains("guardrail_requests_total"));
    assert!(snapshot.contains("route_id=\"test-route\""));
    assert!(snapshot.contains("guardrail_readiness 1"));
}

#[test]
fn test_execution_log_emit_uses_tracing_fields() {
    use crate::execution::{ExecutionRecord, ExecutionVerdict};

    let record = ExecutionRecord {
        execution_id: "GR-EXE-555".to_string(),
        execution_started_at: chrono::Utc::now(),
        route_id: "route-a".to_string(),
        tenant_id: Some("tenant-a".to_string()),
        api_key_id: Some("key-a".to_string()),
        auth_outcome: None,
        upstream_url: Some("https://internal".to_string()),
        method: "POST".to_string(),
        source_ip: "127.0.0.1".to_string(),
        content_type: Some("application/json".to_string()),
        user_agent: Some("test".to_string()),
        had_authorization_header: true,
        request_size_bytes: 20,
        request_body_sha256: "hash".to_string(),
        verdict: ExecutionVerdict::Allowed,
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
        latency_forward_ms: Some(5),
        latency_total_ms: 5,
        route_config_hash: "route-hash".to_string(),
        policy_set_hash: "policy-hash".to_string(),
    };

    let log = crate::logging::ExecutionLog::from(&record);
    let json = serde_json::to_string(&log).unwrap();
    assert!(json.contains("\"execution_id\":\"GR-EXE-555\""));
    assert!(json.contains("\"route_id\":\"route-a\""));
}
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_metrics_snapshot_contains_request_and_readiness_series --lib`
Expected: FAIL because `observability::metrics::Metrics` does not exist yet.

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_execution_log_emit_uses_tracing_fields --lib`
Expected: FAIL after the test is added because `ExecutionLog::emit` still writes to stdout directly.

- [ ] **Step 3: Add metrics and tracing initialization**

```toml
prometheus = "0.13"
opentelemetry = "0.27"
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
tracing-opentelemetry = "0.28"
```

```rust
#[derive(Clone)]
pub struct Metrics {
    registry: prometheus::Registry,
    requests_total: prometheus::IntCounterVec,
    request_latency_seconds: prometheus::HistogramVec,
    upstream_failures_total: prometheus::IntCounterVec,
    audit_persist_failures_total: prometheus::IntCounterVec,
    replay_persist_failures_total: prometheus::IntCounterVec,
    inflight_requests: prometheus::IntGauge,
    readiness: prometheus::IntGauge,
    shutdown_transitions_total: prometheus::IntCounterVec,
}

pub struct InflightGuard {
    gauge: prometheus::IntGauge,
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        self.gauge.dec();
    }
}

impl Metrics {
    pub fn new() -> Self {
        let registry = prometheus::Registry::new();
        let requests_total = prometheus::IntCounterVec::new(
            prometheus::Opts::new("guardrail_requests_total", "Total executed requests"),
            &["route_id", "method", "verdict"],
        )
        .unwrap();
        let request_latency_seconds = prometheus::HistogramVec::new(
            prometheus::HistogramOpts::new(
                "guardrail_request_latency_seconds",
                "End-to-end request latency",
            ),
            &["route_id", "method", "verdict"],
        )
        .unwrap();
        let upstream_failures_total = prometheus::IntCounterVec::new(
            prometheus::Opts::new("guardrail_upstream_failures_total", "Upstream forwarding failures"),
            &["route_id"],
        )
        .unwrap();
        let audit_persist_failures_total = prometheus::IntCounterVec::new(
            prometheus::Opts::new("guardrail_audit_persist_failures_total", "Audit persistence failures"),
            &["operation"],
        )
        .unwrap();
        let replay_persist_failures_total = prometheus::IntCounterVec::new(
            prometheus::Opts::new("guardrail_replay_persist_failures_total", "Replay persistence failures"),
            &["operation"],
        )
        .unwrap();
        let inflight_requests = prometheus::IntGauge::new(
            "guardrail_inflight_requests",
            "Requests currently being served",
        )
        .unwrap();
        let readiness = prometheus::IntGauge::new("guardrail_readiness", "Readiness state").unwrap();
        let shutdown_transitions_total = prometheus::IntCounterVec::new(
            prometheus::Opts::new("guardrail_shutdown_transitions_total", "Lifecycle transitions"),
            &["state"],
        )
        .unwrap();

        registry.register(Box::new(requests_total.clone())).unwrap();
        registry
            .register(Box::new(request_latency_seconds.clone()))
            .unwrap();
        registry
            .register(Box::new(upstream_failures_total.clone()))
            .unwrap();
        registry
            .register(Box::new(audit_persist_failures_total.clone()))
            .unwrap();
        registry
            .register(Box::new(replay_persist_failures_total.clone()))
            .unwrap();
        registry.register(Box::new(inflight_requests.clone())).unwrap();
        registry.register(Box::new(readiness.clone())).unwrap();
        registry
            .register(Box::new(shutdown_transitions_total.clone()))
            .unwrap();

        Self {
            registry,
            requests_total,
            request_latency_seconds,
            upstream_failures_total,
            audit_persist_failures_total,
            replay_persist_failures_total,
            inflight_requests,
            readiness,
            shutdown_transitions_total,
        }
    }

    pub fn record_execution(&self, route_id: &str, method: &str, verdict: &str, latency_seconds: f64) {
        self.requests_total.with_label_values(&[route_id, method, verdict]).inc();
        self.request_latency_seconds
            .with_label_values(&[route_id, method, verdict])
            .observe(latency_seconds);
    }

    pub fn inflight_guard(&self) -> InflightGuard {
        self.inflight_requests.inc();
        InflightGuard {
            gauge: self.inflight_requests.clone(),
        }
    }

    pub fn set_readiness(&self, ready: bool) {
        self.readiness.set(if ready { 1 } else { 0 });
    }

    pub fn render(&self) -> Result<String, prometheus::Error> {
        let encoder = prometheus::TextEncoder::new();
        let families = self.registry.gather();
        let mut buf = Vec::new();
        encoder.encode(&families, &mut buf)?;
        Ok(String::from_utf8(buf).unwrap())
    }
}
```

```rust
pub fn init_tracing(
    config: &crate::config::LoggingConfig,
    observability: &crate::config::ObservabilityConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.level));

    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .try_init()?;

    tracing::info!(service_name = %observability.service_name, "tracing initialized");
    Ok(())
}
```

```rust
pub fn emit(&self) {
    tracing::info!(
        execution_id = %self.execution_id,
        route_id = %self.route_id,
        method = %self.method,
        verdict = %self.verdict,
        upstream_status = ?self.upstream_status,
        forward_error = ?self.forward_error,
        log_type = "execution",
        payload = %serde_json::to_string(self).unwrap_or_default(),
    );
}
```

- [ ] **Step 4: Run the focused tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_metrics_snapshot_contains_request_and_readiness_series --lib`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_execution_log_emit_uses_tracing_fields --lib`
Expected: PASS

- [ ] **Step 5: Commit the observability modules**

```bash
git add guard-rail-engine/Cargo.toml guard-rail-engine/src/observability/mod.rs guard-rail-engine/src/observability/metrics.rs guard-rail-engine/src/observability/tracing.rs guard-rail-engine/src/logging.rs
git commit -m "feat: add stage5 observability modules"
```

## Task 3: Wire Readiness, Metrics, And Graceful Shutdown Into The Runtime

**Files:**
- Modify: `guard-rail-engine/src/main.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Modify: `guard-rail-engine/src/shutdown.rs`
- Modify: `guard-rail-engine/src/storage/postgres.rs`
- Test: `guard-rail-engine/tests/smoke_test.rs`

- [ ] **Step 1: Write the failing smoke tests for readiness and draining**

```rust
#[tokio::test]
async fn test_ready_endpoint_returns_503_until_runtime_ready() {
    let harness = start_harness(false).await;

    let response = reqwest::get(format!("{}/ready", harness.base_url))
        .await
        .unwrap();

    assert_eq!(response.status(), 503);

    harness.lifecycle.mark_ready().await;

    let response = reqwest::get(format!("{}/ready", harness.base_url))
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_metrics_endpoint_exposes_request_counter_after_execution() {
    let harness = start_harness(true).await;

    let client = reqwest::Client::new();
    let execution = client
        .post(format!("{}/v1/execute/open-route", harness.base_url))
        .header("content-type", "application/json")
        .body(r#"{"data":"ok"}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(execution.status(), 200);

    let metrics = reqwest::get(format!("{}/metrics", harness.base_url))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert!(metrics.contains("guardrail_requests_total"));
    assert!(metrics.contains("route_id=\"open-route\""));
}

struct SmokeHarness {
    base_url: String,
    lifecycle: guard_rail_engine::shutdown::LifecycleState,
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

async fn start_mock_upstream_with_delay(status: u16, body: &'static str, delay_ms: u64) -> String {
    let app = axum::Router::new().route(
        "/{*path}",
        axum::routing::any(move || async move {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
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

async fn build_stage5_state(
    upstream_url: &str,
    lifecycle: guard_rail_engine::shutdown::LifecycleState,
) -> guard_rail_engine::proxy::AppState {
    let tmp = tempfile::TempDir::new().unwrap();
    write_file(
        tmp.path(),
        "routes.yaml",
        &format!(
            r#"
routes:
  - id: open-route
    path: /v1/execute/open-route
    upstream: {upstream_url}/api/open
    methods: [POST]
    policies: []
    timeout_ms: 5000
"#
        ),
    );

    let policies_dir = tmp.path().join("policies");
    std::fs::create_dir_all(&policies_dir).unwrap();
    write_file(&policies_dir, "empty.yaml", "policies: []");

    let routes = guard_rail_engine::routes::RouteTable::load(&tmp.path().join("routes.yaml")).unwrap();
    let policies = guard_rail_engine::policy::PolicySet::load_dir(&policies_dir).unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost:5432")
        .unwrap();

    guard_rail_engine::proxy::AppState {
        routes: std::sync::Arc::new(tokio::sync::RwLock::new(routes)),
        policies: std::sync::Arc::new(tokio::sync::RwLock::new(policies)),
        http_client: reqwest::Client::new(),
        audit_store: None,
        route_config_hash: guard_rail_engine::audit::hash::hash_string("routes"),
        policy_set_hash: guard_rail_engine::audit::hash::hash_string("policies"),
        admin_token: "stage-admin-token".to_string(),
        tenant_repo: guard_rail_engine::tenant::repository::TenantRepository::new(pool),
        tenant_cache: guard_rail_engine::tenant::cache::TenantAuthCache::default(),
        rate_limiter: guard_rail_engine::auth::rate_limit::TenantRateLimiter::new(120, 30),
        replay: guard_rail_engine::config::ReplayConfig::default(),
        metrics: guard_rail_engine::observability::metrics::Metrics::new(),
        lifecycle,
    }
}

async fn start_harness(initial_ready: bool) -> SmokeHarness {
    let lifecycle = guard_rail_engine::shutdown::LifecycleState::new();
    if initial_ready {
        lifecycle.mark_ready().await;
    }

    let upstream = start_mock_upstream(200, r#"{"ok":true}"#).await;
    let state = build_stage5_state(&upstream, lifecycle.clone()).await;
    let app = guard_rail_engine::proxy::build_router(state, "stage-admin-token".to_string(), 1_048_576);
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

    SmokeHarness {
        base_url: format!("http://{}", addr),
        lifecycle,
    }
}

async fn start_slow_harness(initial_ready: bool) -> SmokeHarness {
    let lifecycle = guard_rail_engine::shutdown::LifecycleState::new();
    if initial_ready {
        lifecycle.mark_ready().await;
    }

    let upstream = start_mock_upstream_with_delay(200, r#"{"ok":true}"#, 250).await;
    let state = build_stage5_state(&upstream, lifecycle.clone()).await;
    let app = guard_rail_engine::proxy::build_router(state, "stage-admin-token".to_string(), 1_048_576);
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

    SmokeHarness {
        base_url: format!("http://{}", addr),
        lifecycle,
    }
}
```

- [ ] **Step 2: Run the focused smoke tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_ready_endpoint_returns_503_until_runtime_ready --test smoke_test -- --exact`
Expected: FAIL because `/ready` and the harness lifecycle integration do not exist yet.

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_metrics_endpoint_exposes_request_counter_after_execution --test smoke_test -- --exact`
Expected: FAIL because `/metrics` is not exposed and the proxy does not emit metrics yet.

- [ ] **Step 3: Wire runtime state into `AppState`, router, and `main`**

```rust
#[derive(Clone)]
pub struct AppState {
    pub routes: Arc<RwLock<RouteTable>>,
    pub policies: Arc<RwLock<PolicySet>>,
    pub http_client: Client,
    pub audit_store: Option<crate::storage::postgres::PostgresAuditStore>,
    pub route_config_hash: String,
    pub policy_set_hash: String,
    pub admin_token: String,
    pub tenant_repo: crate::tenant::repository::TenantRepository,
    pub tenant_cache: crate::tenant::cache::TenantAuthCache,
    pub rate_limiter: crate::auth::rate_limit::TenantRateLimiter,
    pub replay: ReplayConfig,
    pub metrics: crate::observability::metrics::Metrics,
    pub lifecycle: crate::shutdown::LifecycleState,
}
```

```rust
async fn ready_handler(State(state): State<AppState>) -> impl IntoResponse {
    let lifecycle_ready = state.lifecycle.is_ready().await;
    let db_ready = match &state.audit_store {
        Some(store) => store.readiness_check().await.is_ok(),
        None => true,
    };

    if lifecycle_ready && db_ready {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.metrics.render() {
        Ok(body) => (axum::http::StatusCode::OK, body).into_response(),
        Err(err) => {
            tracing::error!(error = %err, "failed to render metrics");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
```

```rust
pub async fn readiness_check(&self) -> Result<(), sqlx::Error> {
    sqlx::query("select 1").execute(&self.pool).await.map(|_| ())
}
```

```rust
let metrics = crate::observability::metrics::Metrics::new();
let lifecycle = crate::shutdown::LifecycleState::new();

crate::observability::tracing::init_tracing(&app_config.logging, &app_config.observability)?;

let state = AppState {
    routes,
    policies,
    http_client,
    audit_store: Some(audit_store),
    route_config_hash,
    policy_set_hash,
    admin_token: app_config.admin.token.clone(),
    tenant_repo,
    tenant_cache,
    rate_limiter: crate::auth::rate_limit::TenantRateLimiter::new(
        app_config.rate_limit.requests_per_minute,
        app_config.rate_limit.burst,
    ),
    replay: app_config.replay.clone(),
    metrics: metrics.clone(),
    lifecycle: lifecycle.clone(),
};

lifecycle.mark_ready().await;
metrics.set_readiness(true);

axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
    .with_graceful_shutdown(crate::shutdown::shutdown_signal(
        lifecycle.clone(),
        metrics.clone(),
        app_config.shutdown.clone(),
    ))
    .await?;
```

```rust
let main_router = axum::Router::new()
    .route("/v1/execute/{route_id}", axum::routing::any(handle_execute))
    .route("/health", axum::routing::get(|| async { "ok" }))
    .route("/ready", axum::routing::get(ready_handler))
    .route("/metrics", axum::routing::get(metrics_handler));
```

```rust
let inflight = state.metrics.inflight_guard();
let request_span = tracing::info_span!(
    "guardrail_request",
    execution_id = %execution_id,
    route_id = %route_id,
    tenant_id = ?tenant_id,
    method = %method_str
);
let _entered = request_span.enter();

state.metrics.record_execution(
    &route_id,
    &method_str,
    "ALLOWED",
    total_start.elapsed().as_secs_f64(),
);
drop(inflight);
```

- [ ] **Step 4: Run the focused smoke tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_ready_endpoint_returns_503_until_runtime_ready --test smoke_test -- --exact`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_metrics_endpoint_exposes_request_counter_after_execution --test smoke_test -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the runtime wiring**

```bash
git add guard-rail-engine/src/main.rs guard-rail-engine/src/proxy/mod.rs guard-rail-engine/src/shutdown.rs guard-rail-engine/tests/smoke_test.rs
git commit -m "feat: wire stage5 readiness and metrics"
```

## Task 4: Add Graceful Drain Verification And Deployment Artifacts

**Files:**
- Create: `guard-rail-engine/Dockerfile`
- Create: `guard-rail-engine/.dockerignore`
- Create: `guard-rail-engine/deploy/systemd/guard-rail-engine.service`
- Modify: `README.md`
- Modify: `guard-rail-engine/tests/smoke_test.rs`

- [ ] **Step 1: Write the failing shutdown smoke test**

```rust
#[tokio::test]
async fn test_sigterm_drains_inflight_request_before_exit() {
    let harness = start_slow_harness(true).await;

    let client = reqwest::Client::new();
    let in_flight = tokio::spawn({
        let base = harness.base_url.clone();
        async move {
            client
                .post(format!("{}/v1/execute/open-route", base))
                .header("content-type", "application/json")
                .body(r#"{"data":"slow"}"#)
                .send()
                .await
                .unwrap()
                .status()
        }
    });

    harness.lifecycle.begin_drain().await;
    let status = in_flight.await.unwrap();

    assert_eq!(status, reqwest::StatusCode::OK);

    let ready = reqwest::get(format!("{}/ready", harness.base_url))
        .await
        .unwrap();
    assert_eq!(ready.status(), 503);
}
```

- [ ] **Step 2: Run the focused shutdown smoke test to verify it fails**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_sigterm_drains_inflight_request_before_exit --test smoke_test -- --exact`
Expected: FAIL because drain coordination is not complete yet.

- [ ] **Step 3: Add Docker, systemd, docs, and drain verification**

```dockerfile
FROM rust:1.86-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY config ./config
COPY migrations ./migrations
RUN cargo build --release --bin guard-rail-engine

FROM debian:bookworm-slim
WORKDIR /srv/guard-rail-engine
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/guard-rail-engine /usr/local/bin/guard-rail-engine
COPY config ./config
EXPOSE 8080
CMD ["guard-rail-engine", "serve", "--config", "./config/config.yaml"]
```

```text
target
.git
.next
node_modules
tmp
```

```ini
[Unit]
Description=Guard Rail Engine
After=network.target

[Service]
WorkingDirectory=/srv/guard-rail-engine
EnvironmentFile=-/etc/guard-rail-engine.env
ExecStart=/usr/local/bin/guard-rail-engine serve --config /srv/guard-rail-engine/config/config.yaml
Restart=on-failure
RestartSec=3
TimeoutStopSec=20
KillSignal=SIGTERM

[Install]
WantedBy=multi-user.target
```

~~~md
## Guard Rail Engine Operations

Run migrations:

```bash
cd guard-rail-engine
cargo run -- migrate --config ./config/config.yaml
```

Serve locally:

```bash
cd guard-rail-engine
cargo run -- serve --config ./config/config.yaml
```

Operational endpoints:
- `GET /health`
- `GET /ready`
- `GET /metrics`
~~~

- [ ] **Step 4: Run verification for tests and packaging**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_sigterm_drains_inflight_request_before_exit --test smoke_test -- --exact`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && docker build -t guard-rail-engine:stage5 .`
Expected: PASS with a tagged image built locally.

- [ ] **Step 5: Commit the deployment artifacts**

```bash
git add guard-rail-engine/Dockerfile guard-rail-engine/.dockerignore guard-rail-engine/deploy/systemd/guard-rail-engine.service guard-rail-engine/tests/smoke_test.rs README.md
git commit -m "feat: add stage5 deployment artifacts"
```

## Self-Review Coverage

- The spec’s readiness requirement maps to Task 1 and Task 3.
- The spec’s metrics and tracing requirement maps to Task 2 and Task 3.
- The spec’s graceful shutdown requirement maps to Task 1, Task 3, and Task 4.
- The spec’s Docker, systemd, and operations-doc requirement maps to Task 4.
- No placeholder markers or deferred “TODO” language remain in the tasks above.
