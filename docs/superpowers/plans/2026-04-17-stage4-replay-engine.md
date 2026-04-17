# Guard Rail Backend Stage 4: Replay Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add replayable execution artifacts, deduplicated policy snapshots, and offline replay APIs to the existing Stage 3 backend so operators can compare original and replay verdicts without contacting upstream systems.

**Architecture:** Keep the existing single `axum` binary, Stage 2 audit ledger, and Stage 3 tenant isolation model. Add a replay capture lane beside `ExecutionRecord`: the proxy should build a per-execution policy snapshot from the current in-memory route and policy state, persist replay artifacts only for replayable executions, and expose replay APIs that re-run the existing policy engine in offline mode against either the stored snapshot or the current loaded policy set.

**Tech Stack:** Rust, axum, tokio, reqwest, serde, serde_json, serde_yaml, PostgreSQL, `sqlx`, `sha2`, `uuid`, `chrono`

---

## File Structure

```
guard-rail-engine/
  config/
    config.yaml                                        — add replay capture defaults
  migrations/
    0004_create_replay_artifacts.sql                   — policy_snapshots, execution_artifacts, replay_runs
  src/
    lib.rs                                             — export replay module
    config.rs                                          — add replay config and env overrides
    proxy/
      mod.rs                                           — build snapshot + artifact bundle during request handling
      forward.rs                                       — return captured response body and allowed headers
    policy/
      mod.rs                                           — expose serializable policy snapshots
      engine.rs                                        — evaluate against replay snapshot inputs without forwarding
    storage/
      postgres.rs                                      — persist/retrieve snapshots, artifacts, and replay runs
    audit/
      api.rs                                           — expose replay metadata on audit detail
    replay/
      mod.rs                                           — replay module exports
      snapshot.rs                                      — normalized snapshot builder and hashing
      engine.rs                                        — offline replay executor and diff builder
      api.rs                                           — replay endpoints and request/response types
  tests/
    replay_integration_test.rs                         — Stage 4 artifact capture and replay coverage
```

## Task 1: Replay Config And Schema Surface

**Files:**
- Modify: `guard-rail-engine/src/config.rs`
- Modify: `guard-rail-engine/config/config.yaml`
- Modify: `guard-rail-engine/src/lib.rs`
- Create: `guard-rail-engine/migrations/0004_create_replay_artifacts.sql`
- Test: `guard-rail-engine/tests/replay_integration_test.rs`

- [ ] **Step 1: Write the failing config and migration tests**

```rust
#[test]
fn test_load_config_with_stage4_replay_section() {
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
  token: "stage2-admin-token"
tenant_auth: {}
rate_limit: {}
replay:
  enabled: true
  capture_request_headers: ["content-type", "accept", "x-request-id"]
  capture_response_headers: ["content-type", "x-upstream-version"]
  max_response_body_bytes: 65536
"#;

    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    tmp.write_all(yaml.as_bytes()).unwrap();

    let config = crate::config::AppConfig::load(tmp.path()).unwrap();
    assert!(config.replay.enabled);
    assert_eq!(config.replay.capture_request_headers.len(), 3);
    assert_eq!(config.replay.max_response_body_bytes, 65536);
}

#[tokio::test]
async fn test_stage4_migration_creates_replay_tables() {
    let database_url =
        std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
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
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_load_config_with_stage4_replay_section --lib`
Expected: FAIL because `AppConfig` does not have a `replay` section yet.

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_stage4_migration_creates_replay_tables --test replay_integration_test -- --exact`
Expected: FAIL because migration `0004_create_replay_artifacts.sql` and the replay tables do not exist yet.

- [ ] **Step 3: Add the config surface and replay schema**

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
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplayConfig {
    #[serde(default = "default_replay_enabled")]
    pub enabled: bool,
    #[serde(default = "default_capture_request_headers")]
    pub capture_request_headers: Vec<String>,
    #[serde(default = "default_capture_response_headers")]
    pub capture_response_headers: Vec<String>,
    #[serde(default = "default_max_response_body_bytes")]
    pub max_response_body_bytes: usize,
}

fn default_replay_enabled() -> bool { true }
fn default_capture_request_headers() -> Vec<String> {
    vec!["content-type".into(), "accept".into(), "x-request-id".into()]
}
fn default_capture_response_headers() -> Vec<String> {
    vec!["content-type".into(), "x-upstream-version".into()]
}
fn default_max_response_body_bytes() -> usize { 65_536 }
```

```yaml
replay:
  enabled: true
  capture_request_headers: ["content-type", "accept", "x-request-id"]
  capture_response_headers: ["content-type", "x-upstream-version"]
  max_response_body_bytes: 65536
```

```rust
pub mod replay;
```

```sql
create table if not exists policy_snapshots (
    snapshot_hash text primary key,
    route_id text not null,
    route_definition jsonb not null,
    policies_definition jsonb not null,
    route_config_hash text not null,
    policy_set_hash text not null,
    created_at timestamptz not null default now()
);

create table if not exists execution_artifacts (
    execution_id text primary key references execution_audit(execution_id) on delete cascade,
    snapshot_hash text not null references policy_snapshots(snapshot_hash),
    request_body_json jsonb not null,
    request_headers jsonb not null,
    response_status integer,
    response_headers jsonb,
    response_body text,
    response_body_sha256 text,
    response_body_truncated boolean not null default false,
    created_at timestamptz not null default now()
);

create table if not exists replay_runs (
    id uuid primary key,
    execution_id text not null references execution_audit(execution_id) on delete cascade,
    policy_source text not null check (policy_source in ('snapshot', 'current')),
    evaluated_snapshot_hash text not null,
    original_verdict text not null,
    replay_verdict text not null,
    original_policy_name text,
    replay_policy_name text,
    original_rule_field text,
    replay_rule_field text,
    verdict_changed boolean not null,
    created_at timestamptz not null default now()
);

create index if not exists idx_execution_artifacts_snapshot_hash
    on execution_artifacts (snapshot_hash);
create index if not exists idx_replay_runs_execution_created_at
    on replay_runs (execution_id, created_at desc);
```

- [ ] **Step 4: Run the focused tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_load_config_with_stage4_replay_section --lib`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_stage4_migration_creates_replay_tables --test replay_integration_test -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the Stage 4 schema surface**

```bash
git add guard-rail-engine/src/config.rs guard-rail-engine/config/config.yaml guard-rail-engine/src/lib.rs guard-rail-engine/migrations/0004_create_replay_artifacts.sql guard-rail-engine/tests/replay_integration_test.rs
git commit -m "feat: add replay storage schema"
```

## Task 2: Deterministic Snapshot Builder

**Files:**
- Create: `guard-rail-engine/src/replay/snapshot.rs`
- Modify: `guard-rail-engine/src/policy/mod.rs`
- Modify: `guard-rail-engine/src/routes.rs`
- Test: `guard-rail-engine/tests/replay_integration_test.rs`

- [ ] **Step 1: Write the failing snapshot tests**

```rust
#[test]
fn test_snapshot_hash_is_stable_for_equivalent_route_and_policy_state() {
    let route = guard_rail_engine::routes::Route {
        id: "payments".into(),
        path: "/v1/execute/payments".into(),
        upstream: "http://upstream/payments".into(),
        methods: vec!["POST".into()],
        policies: vec!["block-callbacks".into()],
        timeout_ms: 5000,
    };

    let snapshot_a = guard_rail_engine::replay::snapshot::build_snapshot(
        &route,
        &[guard_rail_engine::policy::Policy {
            name: "block-callbacks".into(),
            description: "".into(),
            rules: vec![],
        }],
    );

    let snapshot_b = guard_rail_engine::replay::snapshot::build_snapshot(
        &route,
        &[guard_rail_engine::policy::Policy {
            name: "block-callbacks".into(),
            description: "".into(),
            rules: vec![],
        }],
    );

    assert_eq!(snapshot_a.snapshot_hash, snapshot_b.snapshot_hash);
}

#[test]
fn test_snapshot_builder_uses_only_route_referenced_policies() {
    let route = guard_rail_engine::routes::Route {
        id: "payments".into(),
        path: "/v1/execute/payments".into(),
        upstream: "http://upstream/payments".into(),
        methods: vec!["POST".into()],
        policies: vec!["policy-a".into()],
        timeout_ms: 5000,
    };

    let set = guard_rail_engine::policy::PolicySet::from_policies(vec![
        guard_rail_engine::policy::Policy {
            name: "policy-a".into(),
            description: "".into(),
            rules: vec![],
        },
        guard_rail_engine::policy::Policy {
            name: "policy-b".into(),
            description: "".into(),
            rules: vec![],
        },
    ]);

    let snapshot =
        guard_rail_engine::replay::snapshot::build_snapshot_from_set(&route, &set).unwrap();
    assert_eq!(snapshot.policies_definition.as_array().unwrap().len(), 1);
}
```

- [ ] **Step 2: Run the snapshot tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_snapshot_hash_is_stable_for_equivalent_route_and_policy_state --test replay_integration_test -- --exact`
Expected: FAIL because there is no replay snapshot builder yet.

- [ ] **Step 3: Implement normalized snapshot building**

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PolicySnapshotRecord {
    pub snapshot_hash: String,
    pub route_id: String,
    pub route_definition: serde_json::Value,
    pub policies_definition: serde_json::Value,
    pub route_config_hash: String,
    pub policy_set_hash: String,
}

pub fn build_snapshot(
    route: &crate::routes::Route,
    policies: &[crate::policy::Policy],
) -> PolicySnapshotRecord {
    let route_definition = serde_json::json!({
        "id": route.id,
        "path": route.path,
        "upstream": route.upstream,
        "methods": route.methods,
        "policies": route.policies,
        "timeout_ms": route.timeout_ms,
    });

    let policies_definition = serde_json::to_value(policies).unwrap();
    let route_config_hash = crate::audit::hash::hash_string(&route_definition.to_string());
    let policy_set_hash = crate::audit::hash::hash_string(&policies_definition.to_string());
    let snapshot_hash = crate::audit::hash::hash_string(
        &serde_json::json!({
            "route": route_definition,
            "policies": policies_definition,
        })
        .to_string(),
    );

    PolicySnapshotRecord {
        snapshot_hash,
        route_id: route.id.clone(),
        route_definition,
        policies_definition,
        route_config_hash,
        policy_set_hash,
    }
}
```

```rust
impl PolicySet {
    pub fn from_policies(policies: Vec<Policy>) -> Self {
        let by_name = policies
            .into_iter()
            .map(|policy| (policy.name.clone(), policy))
            .collect();
        Self { by_name }
    }

    pub fn policies_for_route(&self, route: &crate::routes::Route) -> Result<Vec<Policy>, String> {
        route
            .policies
            .iter()
            .map(|name| {
                self.get(name)
                    .cloned()
                    .ok_or_else(|| format!("missing referenced policy: {name}"))
            })
            .collect()
    }
}
```

```rust
impl PolicySnapshotRecord {
    pub fn from_route_and_set(
        route: &crate::routes::Route,
        policy_set: &crate::policy::PolicySet,
    ) -> Result<Self, String> {
        let policies = policy_set.policies_for_route(route)?;
        Ok(build_snapshot(route, &policies))
    }
}
```

```rust
pub fn build_snapshot_from_set(
    route: &crate::routes::Route,
    policy_set: &crate::policy::PolicySet,
) -> Result<PolicySnapshotRecord, String> {
    PolicySnapshotRecord::from_route_and_set(route, policy_set)
}
```

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Route {
    pub id: String,
    pub path: String,
    pub upstream: String,
    pub methods: Vec<String>,
    pub policies: Vec<String>,
    pub timeout_ms: u64,
}
```

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Policy {
    pub name: String,
    pub description: String,
    pub rules: Vec<Rule>,
}
```

- [ ] **Step 4: Run the snapshot tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_snapshot_hash_is_stable_for_equivalent_route_and_policy_state --test replay_integration_test -- --exact`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_snapshot_builder_uses_only_route_referenced_policies --test replay_integration_test -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the snapshot builder**

```bash
git add guard-rail-engine/src/replay/snapshot.rs guard-rail-engine/src/policy/mod.rs guard-rail-engine/src/routes.rs guard-rail-engine/tests/replay_integration_test.rs
git commit -m "feat: add deterministic replay snapshots"
```

## Task 3: Persist Replay Artifacts From The Proxy Path

**Files:**
- Modify: `guard-rail-engine/src/proxy/forward.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Modify: `guard-rail-engine/src/storage/postgres.rs`
- Modify: `guard-rail-engine/src/main.rs`
- Test: `guard-rail-engine/tests/replay_integration_test.rs`

- [ ] **Step 1: Write the failing artifact-capture integration tests**

```rust
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
    let execution_id = response.headers()["x-guardrail-execution-id"].to_str().unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let artifact = harness.store.get_execution_artifacts(execution_id).await.unwrap().unwrap();
    assert_eq!(artifact.response_status, None);
    assert_eq!(artifact.request_headers["x-request-id"], "req-123");
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
    let execution_id = response.headers()["x-guardrail-execution-id"].to_str().unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let artifact = harness.store.get_execution_artifacts(execution_id).await.unwrap().unwrap();
    assert_eq!(artifact.response_status, Some(200));
    assert!(artifact.response_body.as_deref().unwrap().contains("ok"));
    assert!(artifact.request_headers.get("authorization").is_none());
}
```

- [ ] **Step 2: Run the artifact-capture tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_blocked_execution_persists_request_artifacts_without_response_artifacts --test replay_integration_test -- --exact`
Expected: FAIL because the proxy does not persist replay artifacts yet.

- [ ] **Step 3: Extend the forwarding path and storage bundle**

```rust
pub struct ForwardResult {
    pub status: u16,
    pub response_headers: std::collections::BTreeMap<String, String>,
    pub response_body: Option<String>,
    pub response_body_sha256: Option<String>,
    pub response_body_truncated: bool,
    pub response: axum::response::Response,
}
```

```rust
#[derive(Debug, Clone)]
pub struct ReplayArtifacts {
    pub snapshot_hash: String,
    pub request_body_json: serde_json::Value,
    pub request_headers: serde_json::Value,
    pub response_status: Option<u16>,
    pub response_headers: Option<serde_json::Value>,
    pub response_body: Option<String>,
    pub response_body_sha256: Option<String>,
    pub response_body_truncated: bool,
}
```

```rust
#[derive(Clone)]
pub struct AppState {
    pub routes: std::sync::Arc<tokio::sync::RwLock<crate::routes::RouteTable>>,
    pub policies: std::sync::Arc<tokio::sync::RwLock<crate::policy::PolicySet>>,
    pub replay: crate::config::ReplayConfig,
    // keep the existing Stage 3 fields below this line
}
```

```rust
pub async fn insert_execution_bundle(
    &self,
    record: &crate::execution::ExecutionRecord,
    artifacts: Option<&crate::proxy::ReplayArtifacts>,
    snapshot: Option<&crate::replay::snapshot::PolicySnapshotRecord>,
) -> Result<(), sqlx::Error> {
    let mut tx = self.pool.begin().await?;
    self.insert_execution_with_tx(&mut tx, record).await?;

    if let Some(snapshot) = snapshot {
        sqlx::query(
            r#"
            insert into policy_snapshots (
                snapshot_hash, route_id, route_definition, policies_definition,
                route_config_hash, policy_set_hash
            ) values ($1, $2, $3, $4, $5, $6)
            on conflict (snapshot_hash) do nothing
            "#,
        )
        .bind(&snapshot.snapshot_hash)
        .bind(&snapshot.route_id)
        .bind(&snapshot.route_definition)
        .bind(&snapshot.policies_definition)
        .bind(&snapshot.route_config_hash)
        .bind(&snapshot.policy_set_hash)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(artifacts) = artifacts {
        sqlx::query(
            r#"
            insert into execution_artifacts (
                execution_id, snapshot_hash, request_body_json, request_headers,
                response_status, response_headers, response_body, response_body_sha256,
                response_body_truncated
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(&record.execution_id)
        .bind(&artifacts.snapshot_hash)
        .bind(&artifacts.request_body_json)
        .bind(&artifacts.request_headers)
        .bind(artifacts.response_status.map(i32::from))
        .bind(&artifacts.response_headers)
        .bind(&artifacts.response_body)
        .bind(&artifacts.response_body_sha256)
        .bind(artifacts.response_body_truncated)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await
}
```

```rust
let snapshot = crate::replay::snapshot::build_snapshot_from_set(&route, &policies).unwrap();
let replay_artifacts = crate::proxy::ReplayArtifacts {
    snapshot_hash: snapshot.snapshot_hash.clone(),
    request_body_json: payload.clone(),
    request_headers: crate::replay::snapshot::filter_headers(
        &headers,
        &state.replay.capture_request_headers,
    ),
    response_status: None,
    response_headers: None,
    response_body: None,
    response_body_sha256: None,
    response_body_truncated: false,
};
```

- [ ] **Step 4: Run the artifact-capture tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_blocked_execution_persists_request_artifacts_without_response_artifacts --test replay_integration_test -- --exact`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_allowed_execution_persists_response_artifacts_and_strips_authorization --test replay_integration_test -- --exact`
Expected: PASS

- [ ] **Step 5: Commit proxy-side artifact capture**

```bash
git add guard-rail-engine/src/proxy/forward.rs guard-rail-engine/src/proxy/mod.rs guard-rail-engine/src/storage/postgres.rs guard-rail-engine/src/main.rs guard-rail-engine/tests/replay_integration_test.rs
git commit -m "feat: capture replay artifacts during execution"
```

## Task 4: Offline Replay Engine And API Surface

**Files:**
- Create: `guard-rail-engine/src/replay/mod.rs`
- Create: `guard-rail-engine/src/replay/engine.rs`
- Create: `guard-rail-engine/src/replay/api.rs`
- Modify: `guard-rail-engine/src/policy/engine.rs`
- Modify: `guard-rail-engine/src/storage/postgres.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Test: `guard-rail-engine/tests/replay_integration_test.rs`

- [ ] **Step 1: Write the failing replay API tests**

```rust
#[tokio::test]
async fn test_snapshot_replay_returns_original_verdict_without_forwarding_upstream() {
    let harness = start_stage4_test_app().await;
    let execution_id = harness.seed_blocked_execution().await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/replay/executions/{}", harness.base_url, execution_id))
        .header("authorization", format!("Bearer {}", harness.tenant_key))
        .header("content-type", "application/json")
        .body(r#"{"policy_source":"snapshot"}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["original_verdict"], "BLOCKED");
    assert_eq!(body["replay_verdict"], "BLOCKED");
    assert_eq!(harness.upstream_call_count().await, 0);
}

#[tokio::test]
async fn test_current_replay_can_change_verdict_after_policy_change() {
    let harness = start_stage4_test_app().await;
    let execution_id = harness.seed_blocked_execution().await;
    harness.replace_policies_with_allow_all().await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/replay/executions/{}", harness.base_url, execution_id))
        .header("authorization", format!("Bearer {}", harness.tenant_key))
        .header("content-type", "application/json")
        .body(r#"{"policy_source":"current"}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["original_verdict"], "BLOCKED");
    assert_eq!(body["replay_verdict"], "ALLOWED");
    assert_eq!(body["verdict_changed"], true);
}
```

- [ ] **Step 2: Run the replay tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_snapshot_replay_returns_original_verdict_without_forwarding_upstream --test replay_integration_test -- --exact`
Expected: FAIL because there is no replay engine or API yet.

- [ ] **Step 3: Implement offline replay execution and endpoints**

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReplayRequest {
    #[serde(default)]
    pub policy_source: ReplayPolicySource,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayPolicySource {
    #[default]
    Snapshot,
    Current,
}
```

```rust
pub async fn replay_execution(
    state: &crate::proxy::AppState,
    execution_id: &str,
    policy_source: ReplayPolicySource,
) -> Result<ReplayResult, ReplayError> {
    let original = state
        .audit_store
        .as_ref()
        .ok_or(ReplayError::Unavailable)?
        .get_execution_by_id(execution_id)
        .await?
        .ok_or(ReplayError::NotFound)?;

    let artifacts = state
        .audit_store
        .as_ref()
        .unwrap()
        .get_execution_artifacts(execution_id)
        .await?
        .ok_or(ReplayError::ArtifactsMissing)?;

    let evaluation_input = ReplayEvaluationInput {
        payload: artifacts.request_body_json.clone(),
        raw_bytes: original.request_size_bytes,
    };

    let replay_verdict = match policy_source {
        ReplayPolicySource::Snapshot => evaluate_snapshot(&evaluation_input, &artifacts).await?,
        ReplayPolicySource::Current => evaluate_current(state, &original.route_id, &evaluation_input).await?,
    };

    Ok(build_replay_result(original, artifacts, replay_verdict, policy_source))
}
```

```rust
pub async fn create_replay(
    State(state): State<crate::proxy::AppState>,
    Extension(access): Extension<crate::auth::context::AuditAccess>,
    Path(execution_id): Path<String>,
    Json(request): Json<ReplayRequest>,
) -> Result<Json<ReplayResult>, StatusCode> {
    crate::replay::engine::authorize_replay(&state, &access, &execution_id).await?;
    let result = crate::replay::engine::replay_execution(&state, &execution_id, request.policy_source)
        .await
        .map_err(map_replay_error)?;
    Ok(Json(result))
}
```

```rust
let replay_routes = axum::Router::new()
    .route(
        "/v1/replay/executions/{execution_id}",
        axum::routing::post(crate::replay::api::create_replay)
            .get(crate::replay::api::get_replay_summary),
    )
    .layer(axum::middleware::from_fn_with_state(
        state.clone(),
        crate::auth::middleware::require_audit_access,
    ));
```

- [ ] **Step 4: Run the replay tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_snapshot_replay_returns_original_verdict_without_forwarding_upstream --test replay_integration_test -- --exact`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_current_replay_can_change_verdict_after_policy_change --test replay_integration_test -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the replay engine**

```bash
git add guard-rail-engine/src/replay/mod.rs guard-rail-engine/src/replay/engine.rs guard-rail-engine/src/replay/api.rs guard-rail-engine/src/policy/engine.rs guard-rail-engine/src/storage/postgres.rs guard-rail-engine/src/proxy/mod.rs guard-rail-engine/tests/replay_integration_test.rs
git commit -m "feat: add offline replay api"
```

## Task 5: Replay Metadata, Tenant Safety, And Final Verification

**Files:**
- Modify: `guard-rail-engine/src/audit/api.rs`
- Modify: `guard-rail-engine/src/storage/postgres.rs`
- Modify: `guard-rail-engine/tests/replay_integration_test.rs`

- [ ] **Step 1: Write the failing metadata and access-control tests**

```rust
#[tokio::test]
async fn test_audit_detail_exposes_replay_metadata() {
    let harness = start_stage4_test_app().await;
    let execution_id = harness.seed_allowed_execution().await;

    let response = reqwest::Client::new()
        .get(format!("{}/v1/audit/executions/{}", harness.base_url, execution_id))
        .header("authorization", format!("Bearer {}", harness.tenant_key))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["replay_available"], true);
    assert!(body["snapshot_hash"].as_str().is_some());
}

#[tokio::test]
async fn test_tenant_cannot_replay_other_tenant_execution() {
    let harness = start_stage4_test_app().await;
    let execution_id = harness.seed_tenant_b_execution().await;

    let response = reqwest::Client::new()
        .post(format!("{}/v1/replay/executions/{}", harness.base_url, execution_id))
        .header("authorization", format!("Bearer {}", harness.tenant_a_key))
        .header("content-type", "application/json")
        .body(r#"{"policy_source":"snapshot"}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}
```

- [ ] **Step 2: Run the metadata and access tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_audit_detail_exposes_replay_metadata --test replay_integration_test -- --exact`
Expected: FAIL because audit detail does not surface replay metadata yet.

- [ ] **Step 3: Expose replay metadata and enforce tenant-safe replay reads**

```rust
#[derive(Debug, serde::Serialize)]
pub struct ExecutionAuditDetail {
    #[serde(flatten)]
    pub row: crate::storage::postgres::ExecutionAuditRow,
    pub replay_available: bool,
    pub snapshot_hash: Option<String>,
    pub latest_replay_run_id: Option<uuid::Uuid>,
}
```

```rust
pub async fn get_execution_detail(
    &self,
    execution_id: &str,
) -> Result<Option<ExecutionAuditDetailRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
        select
            ea.execution_id,
            ea.execution_started_at,
            ea.route_id,
            ea.tenant_id,
            art.snapshot_hash,
            exists(select 1 from execution_artifacts where execution_id = ea.execution_id) as replay_available,
            (
                select rr.id
                from replay_runs rr
                where rr.execution_id = ea.execution_id
                order by rr.created_at desc
                limit 1
            ) as latest_replay_run_id
        from execution_audit ea
        left join execution_artifacts art on art.execution_id = ea.execution_id
        where ea.execution_id = $1
        "#,
    )
    .bind(execution_id)
    .fetch_optional(&self.pool)
    .await
}
```

```rust
match access {
    AuditAccess::Admin => Ok(Json(detail)),
    AuditAccess::Tenant { tenant_id } if detail.row.tenant_id == Some(tenant_id) => Ok(Json(detail)),
    AuditAccess::Tenant { .. } => Err(StatusCode::NOT_FOUND),
}
```

- [ ] **Step 4: Run the full Stage 4 verification suite**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test --test replay_integration_test`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo run -- migrate`
Expected: `Migrations applied successfully`

- [ ] **Step 5: Commit the replay metadata integration**

```bash
git add guard-rail-engine/src/audit/api.rs guard-rail-engine/src/storage/postgres.rs guard-rail-engine/tests/replay_integration_test.rs
git commit -m "feat: expose replay metadata in audit apis"
```

## Self-Review

- Spec coverage:
  - artifact capture: covered by Task 3
  - deterministic snapshotting from in-memory state: covered by Task 2
  - replay APIs and stored replay runs: covered by Task 4
  - audit discoverability and tenant safety: covered by Task 5
- Placeholder scan:
  - no `TBD`, `TODO`, or deferred test instructions remain
- Type consistency:
  - `snapshot_hash`, `ReplayPolicySource`, and replay metadata names are consistent across Tasks 2 through 5

Plan complete and saved to `docs/superpowers/plans/2026-04-17-stage4-replay-engine.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
