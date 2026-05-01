# Phase 2 Pilot-Safe Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Guard Rail pilot-safe by adding strict pre-forward audit intent durability, replay redaction, stronger production validation, and Phase 7 verification coverage.

**Architecture:** Keep the single Rust runtime and Postgres audit store. Add an `execution_intents` operational table beside the final audit ledger, redact replay artifacts before persistence, remove manual thread-safety assertions, and extend startup validation plus verification scripts.

**Tech Stack:** Rust, axum, tokio, sqlx, PostgreSQL migrations, serde YAML config, Prometheus metrics, Bash Phase 7 verification scripts.

---

## File Structure

```text
guard-rail-engine/
  config/config.yaml
  deploy/container/config.yaml
  deploy/verification/README.md
  migrations/0006_create_execution_intents.sql
  scripts/verification/hard-audit-mode.sh
  scripts/verification/replay-redaction.sh
  scripts/verification/run-phase7-suite.sh
  src/config.rs
  src/main.rs
  src/proxy/mod.rs
  src/replay/mod.rs
  src/replay/redaction.rs
  src/storage/postgres.rs
  tests/audit_api_test.rs
  tests/integration_test.rs
  tests/replay_integration_test.rs
  tests/smoke_test.rs
```

## Task 1: Add Execution Intent Storage

**Files:**
- Create: `guard-rail-engine/migrations/0006_create_execution_intents.sql`
- Modify: `guard-rail-engine/src/storage/postgres.rs`
- Test: `guard-rail-engine/tests/audit_api_test.rs`

- [ ] **Step 1: Create the migration**

Create `guard-rail-engine/migrations/0006_create_execution_intents.sql`:

```sql
create table if not exists execution_intents (
    execution_id text primary key,
    route_id text not null,
    tenant_id text,
    api_key_id text,
    method text not null,
    source_ip text not null,
    content_type text,
    user_agent text,
    request_size_bytes bigint not null,
    request_body_sha256 text not null,
    route_config_hash text not null,
    policy_set_hash text not null,
    status text not null check (status in ('pending', 'finalized', 'finalization_failed')),
    finalization_error text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    finalized_at timestamptz
);

create index if not exists idx_execution_intents_status_created_at
    on execution_intents (status, created_at desc);

create index if not exists idx_execution_intents_route_created_at
    on execution_intents (route_id, created_at desc);
```

- [ ] **Step 2: Add storage types**

In `guard-rail-engine/src/storage/postgres.rs`, add the intent input and status types near the existing row structs:

```rust
#[derive(Debug, Clone)]
pub struct ExecutionIntentRecord {
    pub execution_id: String,
    pub route_id: String,
    pub tenant_id: Option<String>,
    pub api_key_id: Option<String>,
    pub method: String,
    pub source_ip: String,
    pub content_type: Option<String>,
    pub user_agent: Option<String>,
    pub request_size_bytes: i64,
    pub request_body_sha256: String,
    pub route_config_hash: String,
    pub policy_set_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionIntentStatus {
    Finalized,
    FinalizationFailed,
}

impl ExecutionIntentStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Finalized => "finalized",
            Self::FinalizationFailed => "finalization_failed",
        }
    }
}
```

- [ ] **Step 3: Add insert and status update methods**

In `impl PostgresAuditStore`, add:

```rust
pub async fn insert_execution_intent(
    &self,
    intent: &ExecutionIntentRecord,
) -> Result<(), sqlx::Error> {
    tokio::time::timeout(
        self.write_timeout,
        sqlx::query(
            r#"
            insert into execution_intents (
                execution_id, route_id, tenant_id, api_key_id, method, source_ip,
                content_type, user_agent, request_size_bytes, request_body_sha256,
                route_config_hash, policy_set_hash, status
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'pending')
            on conflict (execution_id) do nothing
            "#,
        )
        .bind(&intent.execution_id)
        .bind(&intent.route_id)
        .bind(&intent.tenant_id)
        .bind(&intent.api_key_id)
        .bind(&intent.method)
        .bind(&intent.source_ip)
        .bind(&intent.content_type)
        .bind(&intent.user_agent)
        .bind(intent.request_size_bytes)
        .bind(&intent.request_body_sha256)
        .bind(&intent.route_config_hash)
        .bind(&intent.policy_set_hash)
        .execute(&self.pool),
    )
    .await
    .map_err(|_| sqlx::Error::Protocol("execution intent insert timed out".into()))??;

    Ok(())
}

pub async fn update_execution_intent_status(
    &self,
    execution_id: &str,
    status: ExecutionIntentStatus,
    finalization_error: Option<&str>,
) -> Result<(), sqlx::Error> {
    tokio::time::timeout(
        self.write_timeout,
        sqlx::query(
            r#"
            update execution_intents
            set status = $2,
                finalization_error = $3,
                finalized_at = case when $2 = 'finalized' then now() else finalized_at end,
                updated_at = now()
            where execution_id = $1
            "#,
        )
        .bind(execution_id)
        .bind(status.as_str())
        .bind(finalization_error)
        .execute(&self.pool),
    )
    .await
    .map_err(|_| sqlx::Error::Protocol("execution intent update timed out".into()))??;

    Ok(())
}
```

- [ ] **Step 4: Add a DB-backed storage test**

In `guard-rail-engine/tests/audit_api_test.rs`, add a test using the existing `TEST_DATABASE_URL` fixture:

```rust
#[tokio::test]
async fn execution_intent_can_be_inserted_and_finalized() {
    let _db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    reset_test_database(&pool).await;
    let store = PostgresAuditStore::new(pool.clone(), std::time::Duration::from_millis(250));

    let intent = guard_rail_engine::storage::postgres::ExecutionIntentRecord {
        execution_id: "GR-EXE-intent-test".to_string(),
        route_id: "orders".to_string(),
        tenant_id: Some("tenant-a".to_string()),
        api_key_id: Some("key-a".to_string()),
        method: "POST".to_string(),
        source_ip: "127.0.0.1".to_string(),
        content_type: Some("application/json".to_string()),
        user_agent: Some("integration-test".to_string()),
        request_size_bytes: 17,
        request_body_sha256: "hash".to_string(),
        route_config_hash: "routes".to_string(),
        policy_set_hash: "policies".to_string(),
    };

    store.insert_execution_intent(&intent).await.unwrap();
    store
        .update_execution_intent_status(
            &intent.execution_id,
            guard_rail_engine::storage::postgres::ExecutionIntentStatus::Finalized,
            None,
        )
        .await
        .unwrap();

    let row: (String,) = sqlx::query_as(
        "select status from execution_intents where execution_id = $1",
    )
    .bind(&intent.execution_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.0, "finalized");
}
```

- [ ] **Step 5: Run the focused DB test**

Run:

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
cargo test --test audit_api_test execution_intent_can_be_inserted_and_finalized
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
cd /home/nyasha-hama/projects/guard-rail
git add guard-rail-engine/migrations/0006_create_execution_intents.sql guard-rail-engine/src/storage/postgres.rs guard-rail-engine/tests/audit_api_test.rs
git commit -m "feat: add execution intent storage"
```

## Task 2: Enforce Pre-Forward Intent In Strict Audit Mode

**Files:**
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Test: `guard-rail-engine/tests/smoke_test.rs`

- [ ] **Step 1: Add intent persistence helper**

In `guard-rail-engine/src/proxy/mod.rs`, import the storage types and add a helper near `persist_execution_bundle`:

```rust
use crate::storage::postgres::{ExecutionIntentRecord, ExecutionIntentStatus};
```

```rust
async fn persist_pre_forward_intent(
    intent: ExecutionIntentRecord,
    audit_store: Option<crate::storage::postgres::PostgresAuditStore>,
    metrics: Option<Arc<Metrics>>,
    mode: AuditPersistenceMode,
) -> Result<(), ()> {
    match (mode, audit_store) {
        (AuditPersistenceMode::BestEffort, _) => Ok(()),
        (AuditPersistenceMode::RequiredBeforeResponse, Some(store)) => {
            if let Err(e) = store.insert_execution_intent(&intent).await {
                tracing::error!(error = %e, execution_id = %intent.execution_id, "failed to persist pre-forward execution intent");
                if let Some(metrics) = metrics {
                    metrics.record_audit_persist_failure("insert_execution_intent");
                }
                Err(())
            } else {
                Ok(())
            }
        }
        (AuditPersistenceMode::RequiredBeforeResponse, None) => {
            tracing::error!(execution_id = %intent.execution_id, "audit persistence required but audit store is unavailable for execution intent");
            if let Some(metrics) = metrics {
                metrics.record_audit_persist_failure("missing_audit_store");
            }
            Err(())
        }
    }
}
```

- [ ] **Step 2: Add finalization helpers**

In `guard-rail-engine/src/proxy/mod.rs`, add:

```rust
async fn mark_pre_forward_intent(
    execution_id: &str,
    status: ExecutionIntentStatus,
    error: Option<&str>,
    audit_store: Option<crate::storage::postgres::PostgresAuditStore>,
    metrics: Option<Arc<Metrics>>,
    mode: AuditPersistenceMode,
) {
    if !matches!(mode, AuditPersistenceMode::RequiredBeforeResponse) {
        return;
    }

    let Some(store) = audit_store else {
        return;
    };

    if let Err(e) = store
        .update_execution_intent_status(execution_id, status, error)
        .await
    {
        tracing::error!(error = %e, execution_id, "failed to update pre-forward execution intent");
        if let Some(metrics) = metrics {
            metrics.record_audit_persist_failure("update_execution_intent_status");
        }
    }
}
```

- [ ] **Step 3: Insert the intent before forwarding**

Immediately before `forward::forward_request(...)`, build and persist the intent:

```rust
let intent = ExecutionIntentRecord {
    execution_id: execution_id.clone(),
    route_id: route_id.clone(),
    tenant_id: tenant_id.clone(),
    api_key_id: api_key_id.clone(),
    method: method_str.clone(),
    source_ip: source_ip.clone(),
    content_type: content_type.clone(),
    user_agent: user_agent.clone(),
    request_size_bytes: request_size_bytes as i64,
    request_body_sha256: request_body_sha256.clone(),
    route_config_hash: state.route_config_hash.clone(),
    policy_set_hash: state.policy_set_hash.clone(),
};

if persist_pre_forward_intent(
    intent,
    state.audit_store.clone(),
    state.metrics.clone(),
    state.audit_persistence_mode,
)
.await
.is_err()
{
    return response::audit_persistence_error_response(&execution_id);
}
```

- [ ] **Step 4: Mark successful final persistence as finalized**

After a successful `persist_execution_bundle(...)` or `persist_execution_record(...)` in the allowed upstream success and upstream error branches, call:

```rust
mark_pre_forward_intent(
    &execution_id,
    ExecutionIntentStatus::Finalized,
    None,
    state.audit_store.clone(),
    state.metrics.clone(),
    state.audit_persistence_mode,
)
.await;
```

- [ ] **Step 5: Mark final persistence failure**

When final persistence returns `Err(())` after the upstream call, call this before returning the audit persistence error:

```rust
mark_pre_forward_intent(
    &execution_id,
    ExecutionIntentStatus::FinalizationFailed,
    Some("final audit bundle persistence failed after upstream forwarding"),
    state.audit_store.clone(),
    state.metrics.clone(),
    state.audit_persistence_mode,
)
.await;
return response::audit_persistence_error_response(&execution_id);
```

- [ ] **Step 6: Add smoke coverage for missing store fail-closed behavior**

In `guard-rail-engine/tests/smoke_test.rs`, add a test that builds `AppState` with `audit_persistence_mode: RequiredBeforeResponse`, `audit_store: None`, a valid route, and an upstream server with a request counter. Execute an allowed request and assert:

```rust
assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
assert_eq!(upstream_hit_count.load(Ordering::SeqCst), 0);
```

- [ ] **Step 7: Run focused proxy tests**

Run:

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
cargo test --test smoke_test required_audit_mode_does_not_forward_without_pre_forward_intent
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
cd /home/nyasha-hama/projects/guard-rail
git add guard-rail-engine/src/proxy/mod.rs guard-rail-engine/tests/smoke_test.rs
git commit -m "feat: require audit intent before forwarding"
```

## Task 3: Add Replay Redaction

**Files:**
- Modify: `guard-rail-engine/src/config.rs`
- Modify: `guard-rail-engine/src/replay/mod.rs`
- Create: `guard-rail-engine/src/replay/redaction.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Modify: `guard-rail-engine/config/config.yaml`
- Modify: `guard-rail-engine/deploy/container/config.yaml`
- Test: `guard-rail-engine/tests/replay_integration_test.rs`

- [ ] **Step 1: Extend replay config**

In `guard-rail-engine/src/config.rs`, extend `ReplayConfig`:

```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReplayConfig {
    #[serde(default = "default_replay_enabled")]
    pub enabled: bool,
    #[serde(default = "default_capture_request_headers")]
    pub capture_request_headers: Vec<String>,
    #[serde(default = "default_capture_response_headers")]
    pub capture_response_headers: Vec<String>,
    #[serde(default = "default_max_response_body_bytes")]
    pub max_response_body_bytes: usize,
    #[serde(default = "default_redact_request_headers")]
    pub redact_request_headers: Vec<String>,
    #[serde(default = "default_redact_response_headers")]
    pub redact_response_headers: Vec<String>,
    #[serde(default = "default_redact_json_fields")]
    pub redact_json_fields: Vec<String>,
    #[serde(default = "default_redaction_text")]
    pub redaction_text: String,
}
```

Add defaults:

```rust
fn default_redact_request_headers() -> Vec<String> {
    vec![
        "authorization".into(),
        "cookie".into(),
        "x-api-key".into(),
    ]
}

fn default_redact_response_headers() -> Vec<String> {
    vec!["set-cookie".into(), "x-api-key".into()]
}

fn default_redact_json_fields() -> Vec<String> {
    vec![
        "api_key".into(),
        "access_token".into(),
        "refresh_token".into(),
        "token".into(),
        "secret".into(),
        "password".into(),
        "ssn".into(),
        "id_number".into(),
    ]
}

fn default_redaction_text() -> String {
    "[REDACTED]".to_string()
}
```

- [ ] **Step 2: Add redaction module export**

In `guard-rail-engine/src/replay/mod.rs`, add:

```rust
pub mod redaction;
```

- [ ] **Step 3: Implement redaction helpers**

Create `guard-rail-engine/src/replay/redaction.rs`:

```rust
use serde_json::{Map, Value};
use std::collections::HashSet;

fn lower_set(values: &[String]) -> HashSet<String> {
    values.iter().map(|value| value.to_ascii_lowercase()).collect()
}

pub fn redact_headers(headers: Value, names: &[String], redaction_text: &str) -> Value {
    let redacted = lower_set(names);
    match headers {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if redacted.contains(&key.to_ascii_lowercase()) {
                        (key, Value::String(redaction_text.to_string()))
                    } else {
                        (key, value)
                    }
                })
                .collect::<Map<String, Value>>(),
        ),
        other => other,
    }
}

pub fn redact_json_fields(value: Value, field_names: &[String], redaction_text: &str) -> Value {
    let redacted = lower_set(field_names);
    redact_json_value(value, &redacted, redaction_text)
}

fn redact_json_value(value: Value, field_names: &HashSet<String>, redaction_text: &str) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if field_names.contains(&key.to_ascii_lowercase()) {
                        (key, Value::String(redaction_text.to_string()))
                    } else {
                        (key, redact_json_value(value, field_names, redaction_text))
                    }
                })
                .collect::<Map<String, Value>>(),
        ),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| redact_json_value(item, field_names, redaction_text))
                .collect(),
        ),
        other => other,
    }
}
```

- [ ] **Step 4: Add redaction unit tests**

In `redaction.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_headers_case_insensitively() {
        let value = json!({
            "authorization": "Bearer abc",
            "content-type": "application/json"
        });

        let redacted = redact_headers(value, &["Authorization".to_string()], "[REDACTED]");

        assert_eq!(redacted["authorization"], "[REDACTED]");
        assert_eq!(redacted["content-type"], "application/json");
    }

    #[test]
    fn redacts_nested_json_fields_case_insensitively() {
        let value = json!({
            "customer": {
                "Password": "secret",
                "profile": { "ssn": "123" }
            },
            "items": [{ "token": "abc" }]
        });

        let redacted = redact_json_fields(
            value,
            &["password".to_string(), "SSN".to_string(), "token".to_string()],
            "[REDACTED]",
        );

        assert_eq!(redacted["customer"]["Password"], "[REDACTED]");
        assert_eq!(redacted["customer"]["profile"]["ssn"], "[REDACTED]");
        assert_eq!(redacted["items"][0]["token"], "[REDACTED]");
    }
}
```

- [ ] **Step 5: Apply redaction before artifact persistence**

In `guard-rail-engine/src/proxy/mod.rs`, use the helper wherever `ReplayArtifacts` is built:

```rust
let request_body_json = crate::replay::redaction::redact_json_fields(
    payload.clone(),
    &state.replay.redact_json_fields,
    &state.replay.redaction_text,
);
let request_headers = crate::replay::redaction::redact_headers(
    capture_request_headers,
    &state.replay.redact_request_headers,
    &state.replay.redaction_text,
);
```

For response headers:

```rust
let response_headers = crate::replay::redaction::redact_headers(
    snapshot::filter_headers(result.response.headers(), &state.replay.capture_response_headers),
    &state.replay.redact_response_headers,
    &state.replay.redaction_text,
);
```

Set `ReplayArtifacts.request_body_json` and `ReplayArtifacts.request_headers` to the redacted values.

- [ ] **Step 6: Update YAML examples**

In both `guard-rail-engine/config/config.yaml` and `guard-rail-engine/deploy/container/config.yaml`, add:

```yaml
replay:
  enabled: true
  capture_request_headers: ["content-type", "accept", "x-request-id"]
  capture_response_headers: ["content-type", "x-upstream-version"]
  max_response_body_bytes: 65536
  redact_request_headers: ["authorization", "cookie", "x-api-key"]
  redact_response_headers: ["set-cookie", "x-api-key"]
  redact_json_fields: ["api_key", "access_token", "refresh_token", "token", "secret", "password", "ssn", "id_number"]
  redaction_text: "[REDACTED]"
```

- [ ] **Step 7: Add replay integration coverage**

In `guard-rail-engine/tests/replay_integration_test.rs`, add a DB-backed test that sends a request with:

```json
{
  "email": "ops@example.com",
  "password": "plaintext",
  "nested": { "token": "abc123" }
}
```

Then fetch `execution_artifacts.request_body_json` and assert:

```rust
assert_eq!(body["email"], "ops@example.com");
assert_eq!(body["password"], "[REDACTED]");
assert_eq!(body["nested"]["token"], "[REDACTED]");
```

- [ ] **Step 8: Run replay tests**

Run:

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
cargo test replay::redaction
cargo test --test replay_integration_test replay_artifacts_redact_sensitive_values
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
cd /home/nyasha-hama/projects/guard-rail
git add guard-rail-engine/src/config.rs guard-rail-engine/src/replay/mod.rs guard-rail-engine/src/replay/redaction.rs guard-rail-engine/src/proxy/mod.rs guard-rail-engine/config/config.yaml guard-rail-engine/deploy/container/config.yaml guard-rail-engine/tests/replay_integration_test.rs
git commit -m "feat: redact replay artifacts before persistence"
```

## Task 4: Strengthen Production Startup Validation

**Files:**
- Modify: `guard-rail-engine/src/main.rs`
- Modify: `guard-rail-engine/deploy/container/config.yaml`
- Test: `guard-rail-engine/src/main.rs`

- [ ] **Step 1: Replace startup validation with pilot-safe checks**

In `guard-rail-engine/src/main.rs`, replace `validate_startup_security` with:

```rust
fn validate_startup_security(config: &config::AppConfig) -> Result<(), String> {
    if !matches!(config.environment, config::RuntimeEnvironment::Production) {
        return Ok(());
    }

    if config.database.url.trim().is_empty() {
        return Err("database.url must be configured in production".to_string());
    }

    if !matches!(
        config.audit.persistence_mode,
        config::AuditPersistenceMode::RequiredBeforeResponse
    ) {
        return Err(
            "audit.persistence_mode must be required_before_response in production".to_string(),
        );
    }

    if config.audit.write_timeout_ms == 0 {
        return Err("audit.write_timeout_ms must be greater than zero in production".to_string());
    }

    if config.server.request_body_limit_bytes == 0 {
        return Err(
            "server.request_body_limit_bytes must be greater than zero in production".to_string(),
        );
    }

    if let Some(admin_server) = &config.admin_server {
        let token = config.admin.token.trim();
        if token.is_empty() || token.eq_ignore_ascii_case("change-me") {
            return Err(
                "invalid admin token for production admin listener; configure a non-default token"
                    .to_string(),
            );
        }

        if admin_server.host == "0.0.0.0" || admin_server.host == "::" {
            return Err(
                "production admin listener must bind to loopback or a private management interface"
                    .to_string(),
            );
        }
    }

    if config.replay.enabled {
        if config.replay.max_response_body_bytes == 0 {
            return Err(
                "replay.max_response_body_bytes must be greater than zero when replay is enabled"
                    .to_string(),
            );
        }

        if config.replay.redact_request_headers.is_empty()
            || config.replay.redact_response_headers.is_empty()
            || config.replay.redact_json_fields.is_empty()
            || config.replay.redaction_text.trim().is_empty()
        {
            return Err(
                "replay redaction policy must be configured when replay is enabled in production"
                    .to_string(),
            );
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Update unit tests**

In the `tests` module in `main.rs`, add tests for:

```rust
#[test]
fn production_requires_database_url() {
    let mut cfg = production_config();
    cfg.database.url.clear();
    let err = validate_startup_security(&cfg).unwrap_err();
    assert!(err.contains("database.url"));
}

#[test]
fn production_requires_required_audit_mode() {
    let mut cfg = production_config();
    cfg.audit.persistence_mode = config::AuditPersistenceMode::BestEffort;
    let err = validate_startup_security(&cfg).unwrap_err();
    assert!(err.contains("audit.persistence_mode"));
}

#[test]
fn production_rejects_replay_without_redaction() {
    let mut cfg = production_config();
    cfg.replay.redact_json_fields.clear();
    let err = validate_startup_security(&cfg).unwrap_err();
    assert!(err.contains("replay redaction policy"));
}

#[test]
fn production_rejects_public_admin_bind() {
    let mut cfg = production_config();
    cfg.admin_server.as_mut().unwrap().host = "0.0.0.0".to_string();
    let err = validate_startup_security(&cfg).unwrap_err();
    assert!(err.contains("production admin listener"));
}
```

Make sure `production_config()` supplies a non-empty database URL, strict audit mode, safe replay redaction defaults, and loopback admin listener.

- [ ] **Step 3: Keep the container sample fail-closed**

Do not add environment interpolation to `guard-rail-engine/deploy/container/config.yaml`; the current YAML loader treats those strings literally. Keep the sample production config fail-closed until an operator mounts a real config:

```yaml
database:
  url: ""

admin:
  token: ""
```

The production validation tests in Step 2 must prove that this sample cannot start as a usable production runtime without a real database URL and admin token.

- [ ] **Step 4: Run startup validation tests**

Run:

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
cargo test validate_startup_security
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd /home/nyasha-hama/projects/guard-rail
git add guard-rail-engine/src/main.rs guard-rail-engine/deploy/container/config.yaml
git commit -m "feat: reject unsafe production runtime config"
```

## Task 5: Remove Unsafe AppState Send Sync Implementations

**Files:**
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Test: `guard-rail-engine/src/proxy/mod.rs`

- [ ] **Step 1: Delete unsafe implementations**

Remove:

```rust
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}
```

- [ ] **Step 2: Add compile-time assertion test**

In the `#[cfg(test)]` module in `guard-rail-engine/src/proxy/mod.rs`, add:

```rust
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn app_state_is_send_sync_without_unsafe_impls() {
    assert_send_sync::<AppState>();
}
```

- [ ] **Step 3: Run the focused test**

Run:

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
cargo test proxy::tests::app_state_is_send_sync_without_unsafe_impls
```

Expected: PASS. If it fails, fix the non-thread-safe field instead of restoring unsafe implementations.

- [ ] **Step 4: Commit**

```bash
cd /home/nyasha-hama/projects/guard-rail
git add guard-rail-engine/src/proxy/mod.rs
git commit -m "chore: let app state thread safety be compiler-checked"
```

## Task 6: Add Phase 7 Runtime Hardening Checks

**Files:**
- Create: `guard-rail-engine/scripts/verification/hard-audit-mode.sh`
- Create: `guard-rail-engine/scripts/verification/replay-redaction.sh`
- Modify: `guard-rail-engine/scripts/verification/run-phase7-suite.sh`
- Modify: `guard-rail-engine/deploy/verification/README.md`

- [ ] **Step 1: Add hard audit verification script**

Create `guard-rail-engine/scripts/verification/hard-audit-mode.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

cd "$(cd "${SCRIPT_DIR}/../.." && pwd)"

if cargo test --test smoke_test required_audit_mode_does_not_forward_without_pre_forward_intent; then
  phase7_write_result_json "hard-audit-mode" "pass" '{"focused_test":"pass"}'
  echo "hard-audit-mode: PASS"
else
  phase7_write_result_json "hard-audit-mode" "fail" '{"focused_test":"fail"}'
  echo "hard-audit-mode: FAIL"
  exit 1
fi
```

- [ ] **Step 2: Add replay redaction verification script**

Create `guard-rail-engine/scripts/verification/replay-redaction.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

cd "$(cd "${SCRIPT_DIR}/../.." && pwd)"

if cargo test replay::redaction && cargo test --test replay_integration_test replay_artifacts_redact_sensitive_values; then
  phase7_write_result_json "replay-redaction" "pass" '{"focused_test":"pass"}'
  echo "replay-redaction: PASS"
else
  phase7_write_result_json "replay-redaction" "fail" '{"focused_test":"fail"}'
  echo "replay-redaction: FAIL"
  exit 1
fi
```

- [ ] **Step 3: Make scripts executable**

Run:

```bash
cd /home/nyasha-hama/projects/guard-rail
chmod +x guard-rail-engine/scripts/verification/hard-audit-mode.sh
chmod +x guard-rail-engine/scripts/verification/replay-redaction.sh
```

- [ ] **Step 4: Add scripts to the suite**

In `guard-rail-engine/scripts/verification/run-phase7-suite.sh`, update `scripts=(...)`:

```bash
scripts=(
  "load-allowed-and-blocked.sh"
  "reload-under-traffic.sh"
  "drain-under-load.sh"
  "db-degradation.sh"
  "upstream-degradation.sh"
  "hard-audit-mode.sh"
  "replay-redaction.sh"
  "dependency-audits.sh"
)
```

- [ ] **Step 5: Update verification README**

In `guard-rail-engine/deploy/verification/README.md`, document:

```markdown
### Hard audit mode

Verifies that `required_before_response` does not forward allowed traffic when the pre-forward execution intent cannot be persisted.

### Replay redaction

Verifies that configured sensitive request headers and JSON fields are redacted before replay artifacts are persisted.
```

- [ ] **Step 6: Run verification script checks**

Run:

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
bash scripts/verification/hard-audit-mode.sh
bash scripts/verification/replay-redaction.sh
```

Expected: both scripts print `PASS`.

- [ ] **Step 7: Commit**

```bash
cd /home/nyasha-hama/projects/guard-rail
git add guard-rail-engine/scripts/verification/hard-audit-mode.sh guard-rail-engine/scripts/verification/replay-redaction.sh guard-rail-engine/scripts/verification/run-phase7-suite.sh guard-rail-engine/deploy/verification/README.md
git commit -m "test: add pilot runtime verification checks"
```

## Task 7: Full Verification And Documentation Pass

**Files:**
- Modify: `docs/superpowers/specs/2026-05-01-phase2-pilot-safe-runtime-design.md`
- Modify: `docs/superpowers/plans/2026-05-01-phase2-pilot-safe-runtime.md`

- [ ] **Step 1: Run formatting and core Rust checks**

Run:

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test --lib
cargo test --test integration_test
cargo test --test smoke_test
```

Expected: all commands exit `0`.

- [ ] **Step 2: Run DB-backed integration tests**

Run:

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
cargo test --test audit_api_test
cargo test --test auth_integration_test
cargo test --test replay_integration_test
cargo test --test data_ops_integration_test
```

Expected: all commands exit `0`.

- [ ] **Step 3: Run dependency audit gate**

Run:

```bash
cd /home/nyasha-hama/projects/guard-rail
REQUIRE_DEPENDENCY_AUDIT=true bash guard-rail-engine/scripts/verification/dependency-audits.sh
```

Expected: `dependency-audits: PASS`.

- [ ] **Step 4: Run the Phase 7 suite**

Run:

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
bash scripts/verification/run-phase7-suite.sh
```

Expected: every Phase 7 script prints `PASS`, and the suite exits `0`.

- [ ] **Step 5: Self-review audit guarantee wording**

Confirm docs do not claim external upstream side effects are transactional. The accepted wording is:

```markdown
Guard Rail persists a pre-forward execution intent before allowed upstream forwarding in strict audit mode. Final audit records are persisted after upstream response. If finalization fails after forwarding, the intent is marked `finalization_failed` when possible.
```

- [ ] **Step 6: Commit final doc updates**

```bash
cd /home/nyasha-hama/projects/guard-rail
git add docs/superpowers/specs/2026-05-01-phase2-pilot-safe-runtime-design.md docs/superpowers/plans/2026-05-01-phase2-pilot-safe-runtime.md
git commit -m "docs: add phase 2 pilot runtime plan"
```

## Self-Review Checklist

- [ ] Every Phase 2 spec requirement maps to at least one task.
- [ ] The plan uses a durable pre-forward intent instead of changing the final audit hash-chain semantics.
- [ ] Replay privacy is redaction-first and does not imply KMS-backed encryption.
- [ ] Production validation is scoped to runtime pilot safety.
- [ ] The plan includes focused tests before implementation checks.
- [ ] The plan includes the full Phase 7 verification suite as the exit gate.
