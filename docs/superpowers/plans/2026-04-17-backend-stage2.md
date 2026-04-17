# Guard Rail Backend Stage 2: Storage + Audit Trail Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add PostgreSQL-backed execution persistence, a tamper-evident audit chain, and a protected read-only audit API to the existing Guard Rail proxy without changing the `/v1/execute/{route_id}` contract.

**Architecture:** Keep the existing single `axum` binary and introduce a canonical `ExecutionRecord` that feeds both stdout logging and durable audit storage. Persist one immutable row per configured-route execution, chain rows with `previous_hash` and `record_hash`, and expose audit reads behind a simple admin bearer token. Preserve current proxy-only tests by making the audit store optional in `AppState`, while Stage 2 integration tests exercise the real PostgreSQL path.

**Tech Stack:** Rust, axum, tokio, reqwest, serde, serde_json, serde_yaml, PostgreSQL, `sqlx`, `sha2`, `hex`, `clap`, `chrono`, `uuid`

---

## File Structure

```
guard-rail-engine/
  Cargo.toml                              — add sqlx, sha2, hex, and test deps needed for Stage 2
  migrations/
    0001_create_execution_audit.sql       — create append-only audit ledger table and indexes
  src/
    lib.rs                                — export new execution, audit, auth, and storage modules
    config.rs                             — load database, audit, and admin auth settings
    main.rs                               — parse serve/migrate commands, build store, build router
    logging.rs                            — render stdout JSON from ExecutionRecord
    execution/
      mod.rs                              — canonical execution record and verdict metadata
    audit/
      mod.rs                              — audit module exports
      hash.rs                             — SHA-256 helpers, safe previews, record hashing
      api.rs                              — audit list/detail/integrity handlers and response shapes
    auth/
      mod.rs                              — auth module exports
      middleware.rs                       — bearer-token guard for audit endpoints
    storage/
      mod.rs                              — shared storage types and optional store wrapper
      postgres.rs                         — PgPool setup, schema check, inserts, queries, integrity walks
    proxy/
      mod.rs                              — build ExecutionRecord, emit logs, attempt audit persistence
  config/
    config.yaml                           — include database, audit timeout, and admin token examples
  tests/
    integration_test.rs                   — keep existing proxy tests working with no audit store
    audit_api_test.rs                     — real PostgreSQL integration coverage for persistence and API
```

## Task 1: Stage 2 Config Surface

**Files:**
- Modify: `guard-rail-engine/Cargo.toml`
- Modify: `guard-rail-engine/src/config.rs`
- Modify: `guard-rail-engine/src/lib.rs`
- Modify: `guard-rail-engine/config/config.yaml`

- [ ] **Step 1: Write the failing config tests**

```rust
#[test]
fn test_load_config_with_stage2_sections() {
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
  max_connections: 12
audit:
  write_timeout_ms: 250
admin:
  token: "stage2-admin-token"
"#;
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.write_all(yaml.as_bytes()).unwrap();

    let config = AppConfig::load(tmp.path()).unwrap();
    assert_eq!(config.database.url, "postgres://guardrail:secret@localhost:5432/guardrail");
    assert_eq!(config.database.max_connections, 12);
    assert_eq!(config.audit.write_timeout_ms, 250);
    assert_eq!(config.admin.token, "stage2-admin-token");
}

#[test]
fn test_stage2_env_overrides() {
    let yaml = r#"
server:
  host: "0.0.0.0"
  port: 8080
routes_file: "./routes.yaml"
policies_dir: "./policies/"
forwarding: {}
logging: {}
database:
  url: "postgres://guardrail:secret@localhost:5432/guardrail"
audit: {}
admin:
  token: "from-config"
"#;
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.write_all(yaml.as_bytes()).unwrap();

    unsafe {
        std::env::set_var("GUARDRAIL_DATABASE__URL", "postgres://override:secret@localhost:5432/guardrail");
        std::env::set_var("GUARDRAIL_DATABASE__MAX_CONNECTIONS", "20");
        std::env::set_var("GUARDRAIL_AUDIT__WRITE_TIMEOUT_MS", "400");
        std::env::set_var("GUARDRAIL_ADMIN__TOKEN", "from-env");
    }

    let config = AppConfig::load(tmp.path()).unwrap();
    assert_eq!(config.database.url, "postgres://override:secret@localhost:5432/guardrail");
    assert_eq!(config.database.max_connections, 20);
    assert_eq!(config.audit.write_timeout_ms, 400);
    assert_eq!(config.admin.token, "from-env");

    unsafe {
        std::env::remove_var("GUARDRAIL_DATABASE__URL");
        std::env::remove_var("GUARDRAIL_DATABASE__MAX_CONNECTIONS");
        std::env::remove_var("GUARDRAIL_AUDIT__WRITE_TIMEOUT_MS");
        std::env::remove_var("GUARDRAIL_ADMIN__TOKEN");
    }
}
```

- [ ] **Step 2: Run the config tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test config::tests::test_load_config_with_stage2_sections --lib`

Expected: FAIL with unknown `database`, `audit`, or `admin` fields on `AppConfig`.

- [ ] **Step 3: Implement the Stage 2 config surface**

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
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

fn default_max_connections() -> u32 {
    10
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditConfig {
    #[serde(default = "default_write_timeout_ms")]
    pub write_timeout_ms: u64,
}

fn default_write_timeout_ms() -> u64 {
    250
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminConfig {
    pub token: String,
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let mut config: AppConfig = serde_yaml::from_str(&contents)?;

        if let Ok(val) = std::env::var("GUARDRAIL_SERVER__HOST") {
            config.server.host = val;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_SERVER__PORT") {
            config.server.port = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_LOGGING__LEVEL") {
            config.logging.level = val;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_DATABASE__URL") {
            config.database.url = val;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_DATABASE__MAX_CONNECTIONS") {
            config.database.max_connections = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_AUDIT__WRITE_TIMEOUT_MS") {
            config.audit.write_timeout_ms = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_ADMIN__TOKEN") {
            config.admin.token = val;
        }

        Ok(config)
    }
}
```

```yaml
database:
  url: "postgres://guardrail:secret@localhost:5432/guardrail"
  max_connections: 10

audit:
  write_timeout_ms: 250

admin:
  token: "change-me"
```

```rust
pub mod audit;
pub mod auth;
pub mod config;
pub mod execution;
pub mod logging;
pub mod policy;
pub mod proxy;
pub mod reload;
pub mod routes;
pub mod storage;
```

```toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json", "migrate"] }
sha2 = "0.10"
hex = "0.4"
```

- [ ] **Step 4: Run the updated config tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test config::tests::test_load_config_with_stage2_sections --lib`

Expected: PASS

- [ ] **Step 5: Commit the config changes**

```bash
git add guard-rail-engine/Cargo.toml guard-rail-engine/src/config.rs guard-rail-engine/src/lib.rs guard-rail-engine/config/config.yaml
git commit -m "feat: add stage 2 audit configuration surface"
```

## Task 2: CLI Command And Startup Wiring

**Files:**
- Modify: `guard-rail-engine/src/main.rs`
- Create: `guard-rail-engine/src/storage/mod.rs`
- Create: `guard-rail-engine/src/storage/postgres.rs`

- [ ] **Step 1: Write the failing CLI parse tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_migrate_command_parses() {
        let cli = Cli::try_parse_from([
            "guard-rail-engine",
            "migrate",
            "--config",
            "./config/config.yaml",
        ])
        .unwrap();

        assert!(matches!(cli.command, Command::Migrate));
        assert_eq!(cli.config, PathBuf::from("./config/config.yaml"));
    }

    #[test]
    fn test_serve_is_default_command() {
        let cli = Cli::try_parse_from(["guard-rail-engine"]).unwrap();
        assert!(matches!(cli.command, Command::Serve));
    }
}
```

- [ ] **Step 2: Run the CLI tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_migrate_command_parses --bin guard-rail-engine`

Expected: FAIL because `Cli` does not define a `command` field or `migrate` subcommand.

- [ ] **Step 3: Implement `serve` and `migrate` startup wiring**

```rust
#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
    Serve,
    Migrate,
}

#[derive(Parser)]
#[command(name = "guard-rail-engine", about = "Zero-trust execution runtime")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[arg(short, long, default_value = "./config/config.yaml")]
    config: PathBuf,
}

impl Default for Command {
    fn default() -> Self {
        Command::Serve
    }
}
```

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let app_config = config::AppConfig::load(&cli.config)?;

    match cli.command {
        Command::Migrate => {
            let pool = storage::postgres::connect_pool(&app_config.database).await?;
            storage::postgres::run_migrations(&pool).await?;
            println!("Migrations applied successfully");
            return Ok(());
        }
        Command::Serve => {}
    }

    let pool = storage::postgres::connect_pool(&app_config.database).await?;
    storage::postgres::assert_schema_ready(&pool).await?;
    drop(pool);

    let state = AppState {
        routes,
        policies,
        http_client,
    };

    let app = axum::Router::new()
        .route("/v1/execute/{route_id}", axum::routing::any(proxy::handle_execute))
        .route("/health", axum::routing::get(|| async { "ok" }))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            app_config.server.request_body_limit_bytes,
        ))
        .with_state(state);

    // existing bind + serve remains unchanged in this task
}
```

```rust
use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn connect_pool(
    config: &crate::config::DatabaseConfig,
) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&config.url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn assert_schema_ready(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "select count(*) from information_schema.tables where table_name = 'execution_audit'",
    )
    .fetch_one(pool)
    .await
    .and_then(|count| {
        if count == 1 {
            Ok(())
        } else {
            Err(sqlx::Error::Protocol("execution_audit table missing".into()))
        }
    })
}
```

- [ ] **Step 4: Run the CLI tests again**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_migrate_command_parses --bin guard-rail-engine`

Expected: PASS

- [ ] **Step 5: Commit the bootstrap changes**

```bash
git add guard-rail-engine/src/main.rs guard-rail-engine/src/storage/mod.rs guard-rail-engine/src/storage/postgres.rs
git commit -m "feat: add stage 2 migrate command and schema startup checks"
```

## Task 3: Canonical Execution Record And Audit Hashing

**Files:**
- Create: `guard-rail-engine/src/execution/mod.rs`
- Create: `guard-rail-engine/src/audit/mod.rs`
- Create: `guard-rail-engine/src/audit/hash.rs`
- Modify: `guard-rail-engine/src/logging.rs`

- [ ] **Step 1: Write the failing unit tests for hashing and safe previews**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_body_uses_raw_bytes() {
        let hash_a = hash_body(br#"{"a":1}"#);
        let hash_b = hash_body(br#"{ "a": 1 }"#);
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn test_preview_url_redacts_path_and_query() {
        let preview = preview_violation_value("https://evil.sh/exfil?token=abc");
        assert_eq!(preview.as_deref(), Some("https://evil.sh"));
    }

    #[test]
    fn test_record_hash_changes_when_previous_hash_changes() {
        let record = ExecutionRecord {
            execution_id: "GR-EXE-1".to_string(),
            route_id: "transfer-api".to_string(),
            method: "POST".to_string(),
            source_ip: "127.0.0.1".to_string(),
            upstream_url: Some("https://internal/api".to_string()),
            content_type: Some("application/json".to_string()),
            user_agent: None,
            had_authorization_header: false,
            request_size_bytes: 14,
            request_body_sha256: hash_body(br#"{"amount":10}"#),
            execution_started_at: chrono::Utc::now(),
            verdict: ExecutionVerdict::Blocked,
            rejection_reason: None,
            matched_policy_name: Some("block-callbacks".to_string()),
            matched_rule_field: Some("$.callback".to_string()),
            matched_rule_condition: Some("domain_not_in".to_string()),
            matched_rule_severity: Some("critical".to_string()),
            violation_value_hash: Some(hash_string("https://evil.sh/exfil")),
            violation_value_preview: Some("https://evil.sh".to_string()),
            upstream_status: None,
            forward_error: None,
            latency_inspect_us: 10,
            latency_forward_ms: None,
            latency_total_ms: 1,
            route_config_hash: "route-hash".to_string(),
            policy_set_hash: "policy-hash".to_string(),
        };

        let first = record_hash(&record, None);
        let second = record_hash(&record, Some("previous-hash"));
        assert_ne!(first, second);
    }
}
```

- [ ] **Step 2: Run the new hash tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_hash_body_uses_raw_bytes --lib`

Expected: FAIL because `ExecutionRecord`, `hash_body`, `hash_string`, `preview_violation_value`, and `record_hash` do not exist.

- [ ] **Step 3: Implement `ExecutionRecord`, previews, and log projection**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionVerdict {
    Rejected,
    Blocked,
    Allowed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub execution_id: String,
    pub execution_started_at: chrono::DateTime<chrono::Utc>,
    pub route_id: String,
    pub upstream_url: Option<String>,
    pub method: String,
    pub source_ip: String,
    pub content_type: Option<String>,
    pub user_agent: Option<String>,
    pub had_authorization_header: bool,
    pub request_size_bytes: usize,
    pub request_body_sha256: String,
    pub verdict: ExecutionVerdict,
    pub rejection_reason: Option<String>,
    pub matched_policy_name: Option<String>,
    pub matched_rule_field: Option<String>,
    pub matched_rule_condition: Option<String>,
    pub matched_rule_severity: Option<String>,
    pub violation_value_hash: Option<String>,
    pub violation_value_preview: Option<String>,
    pub upstream_status: Option<u16>,
    pub forward_error: Option<String>,
    pub latency_inspect_us: u128,
    pub latency_forward_ms: Option<u128>,
    pub latency_total_ms: u128,
    pub route_config_hash: String,
    pub policy_set_hash: String,
}
```

```rust
pub fn hash_body(raw: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(raw))
}

pub fn hash_string(value: &str) -> String {
    hash_body(value.as_bytes())
}

pub fn preview_violation_value(value: &str) -> Option<String> {
    if let Ok(url) = url::Url::parse(value) {
        if let Some(host) = url.host_str() {
            return Some(format!("{}://{}", url.scheme(), host));
        }
    }

    if let Some((local, domain)) = value.split_once('@') {
        if !local.is_empty() && !domain.is_empty() {
            return Some(format!("{}***@{}", &local[..1], domain));
        }
    }

    if value.len() >= 8 {
        return Some(format!("{}***{}", &value[..4], &value[value.len() - 4..]));
    }

    None
}

pub fn record_hash(record: &ExecutionRecord, previous_hash: Option<&str>) -> String {
    let canonical = serde_json::json!({
        "execution_id": record.execution_id,
        "execution_started_at": record.execution_started_at,
        "route_id": record.route_id,
        "upstream_url": record.upstream_url,
        "method": record.method,
        "source_ip": record.source_ip,
        "content_type": record.content_type,
        "user_agent": record.user_agent,
        "had_authorization_header": record.had_authorization_header,
        "request_size_bytes": record.request_size_bytes,
        "request_body_sha256": record.request_body_sha256,
        "verdict": record.verdict,
        "rejection_reason": record.rejection_reason,
        "matched_policy_name": record.matched_policy_name,
        "matched_rule_field": record.matched_rule_field,
        "matched_rule_condition": record.matched_rule_condition,
        "matched_rule_severity": record.matched_rule_severity,
        "violation_value_hash": record.violation_value_hash,
        "violation_value_preview": record.violation_value_preview,
        "upstream_status": record.upstream_status,
        "forward_error": record.forward_error,
        "latency_inspect_us": record.latency_inspect_us,
        "latency_forward_ms": record.latency_forward_ms,
        "latency_total_ms": record.latency_total_ms,
        "route_config_hash": record.route_config_hash,
        "policy_set_hash": record.policy_set_hash,
        "previous_hash": previous_hash,
    });

    hash_body(canonical.to_string().as_bytes())
}
```

```rust
impl From<&crate::execution::ExecutionRecord> for ExecutionLog {
    fn from(record: &crate::execution::ExecutionRecord) -> Self {
        ExecutionLog {
            execution_id: record.execution_id.clone(),
            timestamp: record.execution_started_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            route_id: record.route_id.clone(),
            method: record.method.clone(),
            source_ip: record.source_ip.clone(),
            verdict: match record.verdict {
                crate::execution::ExecutionVerdict::Rejected => "REJECTED".to_string(),
                crate::execution::ExecutionVerdict::Blocked => "BLOCKED".to_string(),
                crate::execution::ExecutionVerdict::Allowed => "ALLOWED".to_string(),
            },
            policy: record.matched_policy_name.clone(),
            rule_field: record.matched_rule_field.clone(),
            violation_value: record.violation_value_preview.clone(),
            upstream: record.upstream_url.clone(),
            upstream_status: record.upstream_status,
            forward_error: record.forward_error.clone(),
            latency_inspect_us: record.latency_inspect_us,
            latency_forward_ms: record.latency_forward_ms,
            latency_total_ms: record.latency_total_ms,
        }
    }
}
```

- [ ] **Step 4: Run the new and updated unit tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_hash_body_uses_raw_bytes --lib`

Expected: PASS

- [ ] **Step 5: Commit the execution model and hash helpers**

```bash
git add guard-rail-engine/src/execution/mod.rs guard-rail-engine/src/audit/mod.rs guard-rail-engine/src/audit/hash.rs guard-rail-engine/src/logging.rs
git commit -m "feat: add canonical execution record and audit hashing"
```

## Task 4: PostgreSQL Audit Ledger Storage

**Files:**
- Create: `guard-rail-engine/migrations/0001_create_execution_audit.sql`
- Modify: `guard-rail-engine/src/storage/mod.rs`
- Modify: `guard-rail-engine/src/storage/postgres.rs`
- Create: `guard-rail-engine/tests/audit_api_test.rs`

- [ ] **Step 1: Write the failing storage test for inserting one blocked execution**

```rust
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

    fn sample_blocked_record() -> guard_rail_engine::execution::ExecutionRecord {
        guard_rail_engine::execution::ExecutionRecord {
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
            request_body_sha256: guard_rail_engine::audit::hash::hash_body(br#"{"callback":"https://evil.sh"}"#),
            verdict: guard_rail_engine::execution::ExecutionVerdict::Blocked,
            rejection_reason: None,
            matched_policy_name: Some("block-callbacks".to_string()),
            matched_rule_field: Some("$.callback".to_string()),
            matched_rule_condition: Some("domain_not_in".to_string()),
            matched_rule_severity: Some("critical".to_string()),
            violation_value_hash: Some(guard_rail_engine::audit::hash::hash_string("https://evil.sh")),
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

    let row = store.get_execution_by_id("GR-EXE-blocked-1").await.unwrap().unwrap();
    assert_eq!(row.route_id, "test-route");
    assert_eq!(row.verdict, "BLOCKED");
    assert!(row.previous_hash.is_none());
    assert!(!row.record_hash.is_empty());
}
```

- [ ] **Step 2: Run the storage test to verify it fails**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_insert_execution_and_fetch_it_back --test audit_api_test -- --exact`

Expected: FAIL because the migration file, `PostgresAuditStore`, and query methods do not exist yet.

- [ ] **Step 3: Implement the migration and storage repository**

```sql
create table if not exists execution_audit (
    id bigserial primary key,
    execution_id text not null unique,
    execution_started_at timestamptz not null,
    audit_persisted_at timestamptz not null default now(),
    route_id text not null,
    upstream_url text,
    method text not null,
    source_ip text not null,
    content_type text,
    user_agent text,
    had_authorization_header boolean not null,
    request_size_bytes bigint not null,
    request_body_sha256 text not null,
    verdict text not null,
    rejection_reason text,
    matched_policy_name text,
    matched_rule_field text,
    matched_rule_condition text,
    matched_rule_severity text,
    violation_value_hash text,
    violation_value_preview text,
    upstream_status integer,
    forward_error text,
    latency_inspect_us bigint not null,
    latency_forward_ms bigint,
    latency_total_ms bigint not null,
    route_config_hash text not null,
    policy_set_hash text not null,
    previous_hash text,
    record_hash text not null
);

create index if not exists idx_execution_audit_route_id_desc on execution_audit (route_id, id desc);
create index if not exists idx_execution_audit_verdict_desc on execution_audit (verdict, id desc);
create index if not exists idx_execution_audit_execution_started_desc on execution_audit (execution_started_at desc, id desc);
```

```rust
#[derive(Clone)]
pub struct PostgresAuditStore {
    pool: sqlx::PgPool,
    write_timeout: std::time::Duration,
}

impl PostgresAuditStore {
    pub fn new(pool: sqlx::PgPool, write_timeout: std::time::Duration) -> Self {
        Self { pool, write_timeout }
    }

    pub async fn insert_execution(
        &self,
        record: &crate::execution::ExecutionRecord,
    ) -> Result<(), sqlx::Error> {
        let previous_hash: Option<String> = sqlx::query_scalar(
            "select record_hash from execution_audit order by id desc limit 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        let record_hash = crate::audit::hash::record_hash(record, previous_hash.as_deref());

        tokio::time::timeout(
            self.write_timeout,
            sqlx::query(
                r#"
                insert into execution_audit (
                    execution_id, execution_started_at, route_id, upstream_url, method, source_ip,
                    content_type, user_agent, had_authorization_header, request_size_bytes,
                    request_body_sha256, verdict, rejection_reason, matched_policy_name,
                    matched_rule_field, matched_rule_condition, matched_rule_severity,
                    violation_value_hash, violation_value_preview, upstream_status, forward_error,
                    latency_inspect_us, latency_forward_ms, latency_total_ms,
                    route_config_hash, policy_set_hash, previous_hash, record_hash
                ) values (
                    $1, $2, $3, $4, $5, $6,
                    $7, $8, $9, $10,
                    $11, $12, $13, $14,
                    $15, $16, $17,
                    $18, $19, $20, $21,
                    $22, $23, $24,
                    $25, $26, $27, $28
                )
                "#,
            )
            .bind(&record.execution_id)
            .bind(record.execution_started_at)
            .bind(&record.route_id)
            .bind(&record.upstream_url)
            .bind(&record.method)
            .bind(&record.source_ip)
            .bind(&record.content_type)
            .bind(&record.user_agent)
            .bind(record.had_authorization_header)
            .bind(record.request_size_bytes as i64)
            .bind(&record.request_body_sha256)
            .bind(match record.verdict {
                crate::execution::ExecutionVerdict::Rejected => "REJECTED",
                crate::execution::ExecutionVerdict::Blocked => "BLOCKED",
                crate::execution::ExecutionVerdict::Allowed => "ALLOWED",
            })
            .bind(&record.rejection_reason)
            .bind(&record.matched_policy_name)
            .bind(&record.matched_rule_field)
            .bind(&record.matched_rule_condition)
            .bind(&record.matched_rule_severity)
            .bind(&record.violation_value_hash)
            .bind(&record.violation_value_preview)
            .bind(record.upstream_status.map(i32::from))
            .bind(&record.forward_error)
            .bind(record.latency_inspect_us as i64)
            .bind(record.latency_forward_ms.map(|v| v as i64))
            .bind(record.latency_total_ms as i64)
            .bind(&record.route_config_hash)
            .bind(&record.policy_set_hash)
            .bind(&previous_hash)
            .bind(record_hash)
            .execute(&self.pool),
        )
        .await
        .map_err(|_| sqlx::Error::Protocol("audit insert timed out".into()))??;

        Ok(())
    }

    pub async fn count_executions(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("select count(*) from execution_audit")
            .fetch_one(&self.pool)
            .await
    }
}
```

- [ ] **Step 4: Run the storage test again**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_insert_execution_and_fetch_it_back --test audit_api_test -- --exact`

Expected: PASS

- [ ] **Step 5: Commit the migration and storage layer**

```bash
git add guard-rail-engine/migrations/0001_create_execution_audit.sql guard-rail-engine/src/storage/mod.rs guard-rail-engine/src/storage/postgres.rs
git commit -m "feat: add postgres-backed execution audit ledger"
```

## Task 5: Protected Audit API

**Files:**
- Create: `guard-rail-engine/src/auth/mod.rs`
- Create: `guard-rail-engine/src/auth/middleware.rs`
- Create: `guard-rail-engine/src/audit/api.rs`
- Modify: `guard-rail-engine/src/main.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Modify: `guard-rail-engine/tests/integration_test.rs`

- [ ] **Step 1: Write the failing audit API tests**

```rust
async fn build_test_router_with_audit_store() -> axum::Router {
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

    let audit_store = guard_rail_engine::storage::postgres::PostgresAuditStore::new(
        pool,
        std::time::Duration::from_millis(250),
    );

    let state = guard_rail_engine::proxy::AppState {
        routes: std::sync::Arc::new(tokio::sync::RwLock::new(
            guard_rail_engine::routes::RouteTable::load(std::path::Path::new("./config/routes.yaml")).unwrap(),
        )),
        policies: std::sync::Arc::new(tokio::sync::RwLock::new(
            guard_rail_engine::policy::PolicySet::load_dir(std::path::Path::new("./config/policies")).unwrap(),
        )),
        http_client: reqwest::Client::new(),
        audit_store: Some(audit_store),
    };

    guard_rail_engine::proxy::build_router(
        state,
        "stage2-admin-token".to_string(),
        1_048_576,
    )
}

async fn build_test_router_with_seeded_audit_rows() -> axum::Router {
    let app = build_test_router_with_audit_store().await;
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

    app
}

#[tokio::test]
async fn test_audit_list_requires_admin_token() {
    let app = build_test_router_with_audit_store().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/v1/audit/executions")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_audit_list_returns_newest_first() {
    let app = build_test_router_with_seeded_audit_rows().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/v1/audit/executions?limit=2")
                .header("authorization", "Bearer stage2-admin-token")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"][0]["execution_id"], "GR-EXE-3");
    assert_eq!(json["items"][1]["execution_id"], "GR-EXE-2");
}
```

- [ ] **Step 2: Run the audit API tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_audit_list_requires_admin_token --test audit_api_test -- --exact`

Expected: FAIL because audit routes and admin middleware do not exist.

- [ ] **Step 3: Implement admin auth and audit handlers**

```rust
#[derive(Clone)]
pub struct AppState {
    pub routes: Arc<RwLock<RouteTable>>,
    pub policies: Arc<RwLock<PolicySet>>,
    pub http_client: Client,
    pub audit_store: Option<crate::storage::postgres::PostgresAuditStore>,
}
```

```rust
pub async fn require_admin_token(
    State(expected_token): State<String>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    let header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let expected = format!("Bearer {}", expected_token);
    if header != expected {
        return Err(axum::http::StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}
```

```rust
#[derive(Debug, Deserialize)]
pub struct AuditListQuery {
    pub route_id: Option<String>,
    pub verdict: Option<String>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<i64>,
    pub cursor: Option<i64>,
    pub order: Option<String>,
}

pub async fn list_executions(
    State(state): State<crate::proxy::AppState>,
    Query(query): Query<AuditListQuery>,
) -> Result<Json<AuditListResponse>, StatusCode> {
    let store = state.audit_store.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let page = store.list_executions(query).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(page))
}

pub async fn get_execution(
    State(state): State<crate::proxy::AppState>,
    Path(execution_id): Path<String>,
) -> Result<Json<AuditExecution>, StatusCode> {
    let store = state.audit_store.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let row = store
        .get_execution_by_id(&execution_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(row))
}

pub async fn verify_integrity(
    State(state): State<crate::proxy::AppState>,
    Query(query): Query<IntegrityQuery>,
) -> Result<Json<IntegrityResponse>, StatusCode> {
    let store = state.audit_store.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let result = store.verify_integrity(query).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(result))
}
```

```rust
pub fn build_router(
    state: AppState,
    admin_token: String,
    request_body_limit_bytes: usize,
) -> axum::Router {
    let audit_routes = axum::Router::new()
        .route("/v1/audit/executions", axum::routing::get(crate::audit::api::list_executions))
        .route("/v1/audit/executions/{execution_id}", axum::routing::get(crate::audit::api::get_execution))
        .route("/v1/audit/integrity", axum::routing::get(crate::audit::api::verify_integrity))
        .route_layer(axum::middleware::from_fn_with_state(
            admin_token,
            crate::auth::middleware::require_admin_token,
        ));

    axum::Router::new()
        .route("/v1/execute/{route_id}", axum::routing::any(crate::proxy::handle_execute))
        .route("/health", axum::routing::get(|| async { "ok" }))
        .merge(audit_routes)
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            request_body_limit_bytes,
        ))
        .with_state(state)
}
```

```rust
let audit_store = storage::postgres::PostgresAuditStore::new(
    pool,
    std::time::Duration::from_millis(app_config.audit.write_timeout_ms),
);

let state = proxy::AppState {
    routes,
    policies,
    http_client,
    audit_store: Some(audit_store),
};

let app = proxy::build_router(
    state,
    app_config.admin.token.clone(),
    app_config.server.request_body_limit_bytes,
);
```

```rust
// Existing non-DB tests keep compiling by supplying no audit store
let state = guard_rail_engine::proxy::AppState {
    routes: Arc::new(RwLock::new(route_table)),
    policies: Arc::new(RwLock::new(policy_set)),
    http_client: Client::new(),
    audit_store: None,
};
```

- [ ] **Step 4: Run the audit API tests again**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_audit_list_requires_admin_token --test audit_api_test -- --exact`

Expected: PASS

- [ ] **Step 5: Commit the audit API**

```bash
git add guard-rail-engine/src/auth/mod.rs guard-rail-engine/src/auth/middleware.rs guard-rail-engine/src/audit/api.rs guard-rail-engine/src/main.rs
git commit -m "feat: add protected stage 2 audit api"
```

## Task 6: Proxy Persistence Integration And End-To-End Tests

**Files:**
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Modify: `guard-rail-engine/tests/integration_test.rs`
- Modify: `guard-rail-engine/tests/audit_api_test.rs`

- [ ] **Step 1: Write the failing end-to-end tests**

```rust
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

async fn start_stage2_test_app(
) -> (
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

    let route_table = guard_rail_engine::routes::RouteTable::load(&config_dir.join("routes.yaml")).unwrap();
    let policy_set = guard_rail_engine::policy::PolicySet::load_dir(&policies_dir).unwrap();
    let state = guard_rail_engine::proxy::AppState {
        routes: std::sync::Arc::new(tokio::sync::RwLock::new(route_table)),
        policies: std::sync::Arc::new(tokio::sync::RwLock::new(policy_set)),
        http_client: reqwest::Client::new(),
        audit_store: Some(store.clone()),
    };

    let app = axum::Router::new()
        .route("/v1/execute/{route_id}", axum::routing::any(guard_rail_engine::proxy::handle_execute))
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
    let row = store.get_execution_by_id(&execution_id).await.unwrap().unwrap();
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
    assert_eq!(store.count_executions().await.unwrap(), 0);
}
```

- [ ] **Step 2: Run the new end-to-end tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_invalid_json_on_known_route_persists_rejected_audit_row --test audit_api_test -- --exact`

Expected: FAIL because `proxy::handle_execute` still builds `ExecutionLog` directly and does not persist a final execution record.

- [ ] **Step 3: Integrate the proxy with the execution record and optional audit store**

```rust
let mut record = crate::execution::ExecutionRecord {
    execution_id: execution_id.clone(),
    execution_started_at: chrono::Utc::now(),
    route_id: route_id.clone(),
    upstream_url: Some(route.upstream.clone()),
    method: method.to_string(),
    source_ip,
    content_type: headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned),
    user_agent: headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned),
    had_authorization_header: headers.contains_key(axum::http::header::AUTHORIZATION),
    request_size_bytes: body.len(),
    request_body_sha256: crate::audit::hash::hash_body(&body),
    verdict: crate::execution::ExecutionVerdict::Rejected,
    rejection_reason: None,
    matched_policy_name: None,
    matched_rule_field: None,
    matched_rule_condition: None,
    matched_rule_severity: None,
    violation_value_hash: None,
    violation_value_preview: None,
    upstream_status: None,
    forward_error: None,
    latency_inspect_us: 0,
    latency_forward_ms: None,
    latency_total_ms: 0,
    route_config_hash: crate::audit::hash::hash_string(&serde_json::to_string(&route).unwrap()),
    policy_set_hash: crate::audit::hash::hash_string(&serde_json::to_string(&route.policies).unwrap()),
};
```

```rust
async fn emit_and_persist(state: &AppState, record: &crate::execution::ExecutionRecord) {
    crate::logging::ExecutionLog::from(record).emit();

    if let Some(store) = state.audit_store.as_ref() {
        if let Err(error) = store.insert_execution(record).await {
            tracing::error!(
                execution_id = %record.execution_id,
                route_id = %record.route_id,
                error = %error,
                "failed to persist audit row"
            );
        }
    }
}
```

```rust
record.verdict = crate::execution::ExecutionVerdict::Rejected;
record.rejection_reason = Some("invalid_json".to_string());
record.latency_total_ms = total_start.elapsed().as_millis();
emit_and_persist(&state, &record).await;
return response::reject_response(&execution_id, "Invalid JSON body");
```

- [ ] **Step 4: Run the full Stage 2 test suite**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test`

Expected: PASS, including the existing proxy integration tests and the new PostgreSQL-backed `audit_api_test.rs` suite.

- [ ] **Step 5: Commit the proxy integration**

```bash
git add guard-rail-engine/src/proxy/mod.rs guard-rail-engine/tests/integration_test.rs guard-rail-engine/tests/audit_api_test.rs
git commit -m "feat: persist execution audit rows from proxy path"
```

## Manual Verification

- [ ] Start PostgreSQL locally:

```bash
docker run --rm -d \
  --name guardrail-stage2-postgres \
  -e POSTGRES_DB=guardrail \
  -e POSTGRES_USER=guardrail \
  -e POSTGRES_PASSWORD=secret \
  -p 55432:5432 \
  postgres:16-alpine
```

- [ ] Apply migrations explicitly:

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
GUARDRAIL_DATABASE__URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail \
cargo run -- migrate --config ./config/config.yaml
```

Expected: `Migrations applied successfully`

- [ ] Start the service:

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
GUARDRAIL_DATABASE__URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail \
GUARDRAIL_ADMIN__TOKEN=stage2-admin-token \
cargo run -- serve --config ./config/config.yaml
```

- [ ] Send one blocked request:

```bash
curl -i \
  -H 'content-type: application/json' \
  -d '{"callback":"https://evil.sh/exfil","amount":100}' \
  http://127.0.0.1:8080/v1/execute/transfer-api
```

Expected: `403 Forbidden` with `x-guardrail-execution-id`

- [ ] Query the persisted audit rows:

```bash
curl -s \
  -H 'authorization: Bearer stage2-admin-token' \
  'http://127.0.0.1:8080/v1/audit/executions?limit=5' | jq
```

Expected: newest-first JSON response with one `BLOCKED` item and a non-empty `record_hash`

- [ ] Verify integrity for the recent range:

```bash
curl -s \
  -H 'authorization: Bearer stage2-admin-token' \
  'http://127.0.0.1:8080/v1/audit/integrity?limit=10' | jq
```

Expected: `{ "valid": true, ... }`

## Self-Review

- [ ] Spec coverage check
  - Config, migration mode, admin auth, append-only storage, hashing, audit list/detail/integrity endpoints, fail-open persistence, and PostgreSQL-backed tests are all mapped to tasks.
- [ ] Placeholder scan
  - No `TBD`, `TODO`, or deferred implementation markers remain in the task steps.
- [ ] Type consistency check
  - `ExecutionRecord`, `ExecutionVerdict`, `PostgresAuditStore`, `AppState.audit_store`, `build_router`, and the audit handler names are used consistently across the plan.
