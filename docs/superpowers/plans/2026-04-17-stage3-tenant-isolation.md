# Guard Rail Backend Stage 3: Tenant Isolation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add tenant isolation, hashed API keys, tenant-owned route authorization, tenant-scoped audit visibility, and basic per-tenant rate limiting to the existing Stage 2 backend without changing the `/v1/execute/{route_id}` contract.

**Architecture:** Keep the existing single `axum` binary and Stage 2 audit/storage flow. YAML remains the source of route and policy behavior, while PostgreSQL becomes the source of tenant security state. The runtime should load tenant route bindings and active key metadata into in-memory caches, enforce authentication and ownership before policy evaluation, and refresh that cache immediately after successful admin writes.

**Tech Stack:** Rust, axum, tokio, reqwest, serde, serde_json, serde_yaml, PostgreSQL, `sqlx`, `sha2`, `hex`, `chrono`, `uuid`

---

## File Structure

```
guard-rail-engine/
  Cargo.toml                                           — keep current deps; only add new ones if implementation proves necessary
  config/
    config.yaml                                        — add Stage 3 tenant/rate-limit settings
  migrations/
    0002_create_tenant_security.sql                    — tenants, api_keys, tenant_routes, Stage 3 audit columns and indexes
  src/
    lib.rs                                             — export new tenant module
    config.rs                                          — add tenant/rate-limit config and env overrides
    main.rs                                            — load tenant auth state, validate bindings, wire admin routes and caches
    execution/
      mod.rs                                           — extend execution record with tenant and auth fields
    auth/
      mod.rs                                           — export new auth helpers
      middleware.rs                                    — keep admin bearer guard and add tenant auth extraction helpers
      api_keys.rs                                      — key generation, hashing, prefix extraction, cache records
      context.rs                                       — role-aware auth context for admin and tenant callers
      rate_limit.rs                                    — in-memory per-tenant limiter
    tenant/
      mod.rs                                           — tenant module exports
      api.rs                                           — admin tenant/key/binding handlers
      repository.rs                                    — tenant, key, and binding queries plus cache snapshot loader
      cache.rs                                         — in-memory tenant auth/binding cache types and refresh logic
    audit/
      api.rs                                           — role-aware filtering and tenant-safe detail access
    proxy/
      mod.rs                                           — enforce tenant auth and ownership before existing execution flow
    storage/
      postgres.rs                                      — persist Stage 3 audit columns and support tenant-aware audit queries
  tests/
    auth_integration_test.rs                           — tenant auth, wrong-tenant, revocation, disable, rate limit coverage
    audit_api_test.rs                                  — tenant/admin audit visibility and auth-failure audit coverage
```

## Task 1: Stage 3 Config And Schema Surface

**Files:**
- Modify: `guard-rail-engine/src/config.rs`
- Modify: `guard-rail-engine/src/lib.rs`
- Modify: `guard-rail-engine/config/config.yaml`
- Create: `guard-rail-engine/migrations/0002_create_tenant_security.sql`

- [ ] **Step 1: Write the failing config and schema tests**

```rust
#[test]
fn test_load_config_with_stage3_sections() {
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
tenant_auth:
  header_name: "authorization"
rate_limit:
  requests_per_minute: 120
  burst: 30
"#;
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    tmp.write_all(yaml.as_bytes()).unwrap();

    let config = crate::config::AppConfig::load(tmp.path()).unwrap();
    assert_eq!(config.tenant_auth.header_name, "authorization");
    assert_eq!(config.rate_limit.requests_per_minute, 120);
    assert_eq!(config.rate_limit.burst, 30);
}

#[tokio::test]
async fn test_stage3_migration_creates_tenant_tables() {
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
          and table_name in ('tenants', 'api_keys', 'tenant_routes')
        order by table_name
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(tables, vec!["api_keys", "tenant_routes", "tenants"]);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_load_config_with_stage3_sections --lib`
Expected: FAIL with unknown `tenant_auth` or `rate_limit` fields.

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_stage3_migration_creates_tenant_tables --test audit_api_test -- --exact`
Expected: FAIL because the Stage 3 migration and tables do not exist yet.

- [ ] **Step 3: Implement the config surface and migration**

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
}

#[derive(Debug, Clone, Deserialize)]
pub struct TenantAuthConfig {
    #[serde(default = "default_authorization_header")]
    pub header_name: String,
}

fn default_authorization_header() -> String {
    "authorization".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,
    #[serde(default = "default_burst")]
    pub burst: u32,
}

fn default_requests_per_minute() -> u32 {
    120
}

fn default_burst() -> u32 {
    30
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let mut config: AppConfig = serde_yaml::from_str(&contents)?;

        if let Ok(val) = std::env::var("GUARDRAIL_TENANT_AUTH__HEADER_NAME") {
            config.tenant_auth.header_name = val;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_RATE_LIMIT__REQUESTS_PER_MINUTE") {
            config.rate_limit.requests_per_minute = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_RATE_LIMIT__BURST") {
            config.rate_limit.burst = val.parse()?;
        }

        Ok(config)
    }
}
```

```yaml
tenant_auth:
  header_name: "authorization"

rate_limit:
  requests_per_minute: 120
  burst: 30
```

```rust
pub mod tenant;
```

```sql
create table if not exists tenants (
    id uuid primary key,
    name text not null unique,
    status text not null check (status in ('active', 'disabled')),
    created_at timestamptz not null default now(),
    disabled_at timestamptz
);

create table if not exists api_keys (
    id uuid primary key,
    tenant_id uuid not null references tenants(id),
    key_prefix text not null,
    key_hash text not null unique,
    name text not null,
    created_at timestamptz not null default now(),
    last_used_at timestamptz,
    revoked_at timestamptz,
    revoked_reason text
);

create table if not exists tenant_routes (
    route_id text primary key,
    tenant_id uuid not null references tenants(id),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

alter table execution_audit
    add column if not exists tenant_id uuid references tenants(id),
    add column if not exists api_key_id uuid references api_keys(id),
    add column if not exists auth_outcome text;

create index if not exists idx_execution_audit_tenant_started_at
    on execution_audit (tenant_id, execution_started_at desc);
create index if not exists idx_api_keys_tenant_active
    on api_keys (tenant_id, revoked_at);
```

- [ ] **Step 4: Run the focused tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_load_config_with_stage3_sections --lib`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_stage3_migration_creates_tenant_tables --test audit_api_test -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the config and migration surface**

```bash
git add guard-rail-engine/src/config.rs guard-rail-engine/src/lib.rs guard-rail-engine/config/config.yaml guard-rail-engine/migrations/0002_create_tenant_security.sql
git commit -m "feat: add stage 3 tenant security schema"
```

## Task 2: Tenant Repository And API Key Primitives

**Files:**
- Create: `guard-rail-engine/src/auth/api_keys.rs`
- Create: `guard-rail-engine/src/tenant/mod.rs`
- Create: `guard-rail-engine/src/tenant/repository.rs`
- Modify: `guard-rail-engine/src/auth/mod.rs`
- Modify: `guard-rail-engine/src/storage/postgres.rs`

- [ ] **Step 1: Write the failing repository and key tests**

```rust
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
    let tenant = repo.create_tenant("acme").await.unwrap();
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

#[test]
fn test_hash_api_key_is_deterministic() {
    let hash_a = guard_rail_engine::auth::api_keys::hash_api_key("grk_test");
    let hash_b = guard_rail_engine::auth::api_keys::hash_api_key("grk_test");
    assert_eq!(hash_a, hash_b);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_hash_api_key_is_deterministic --lib`
Expected: FAIL because `auth::api_keys` does not exist yet.

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_create_tenant_and_api_key_persists_hash_only --test auth_integration_test -- --exact`
Expected: FAIL because the repository and integration test file do not exist yet.

- [ ] **Step 3: Implement API key helpers and the tenant repository**

```rust
#[derive(Debug, Clone)]
pub struct IssuedApiKey {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub name: String,
    pub key_prefix: String,
    pub raw_key: String,
}

pub fn generate_api_key() -> String {
    format!(
        "grk_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

pub fn key_prefix(raw_key: &str) -> String {
    raw_key.chars().take(12).collect()
}

pub fn hash_api_key(raw_key: &str) -> String {
    crate::audit::hash::hash_string(raw_key)
}
```

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Tenant {
    pub id: uuid::Uuid,
    pub name: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub disabled_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone)]
pub struct TenantRepository {
    pool: sqlx::PgPool,
}

impl TenantRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_tenant(&self, name: &str) -> Result<Tenant, sqlx::Error> {
        let id = uuid::Uuid::new_v4();
        let created_at = chrono::Utc::now();
        sqlx::query(
            "insert into tenants (id, name, status, created_at) values ($1, $2, 'active', $3)",
        )
        .bind(id)
        .bind(name)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(Tenant {
            id,
            name: name.to_string(),
            status: "active".to_string(),
            created_at,
            disabled_at: None,
        })
    }

    pub async fn create_api_key(
        &self,
        tenant_id: uuid::Uuid,
        name: &str,
    ) -> Result<crate::auth::api_keys::IssuedApiKey, sqlx::Error> {
        let id = uuid::Uuid::new_v4();
        let raw_key = crate::auth::api_keys::generate_api_key();
        let key_prefix = crate::auth::api_keys::key_prefix(&raw_key);
        let key_hash = crate::auth::api_keys::hash_api_key(&raw_key);

        sqlx::query(
            r#"
            insert into api_keys (id, tenant_id, key_prefix, key_hash, name)
            values ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(&key_prefix)
        .bind(&key_hash)
        .bind(name)
        .execute(&self.pool)
        .await?;

        Ok(crate::auth::api_keys::IssuedApiKey {
            id,
            tenant_id,
            name: name.to_string(),
            key_prefix,
            raw_key,
        })
    }
}
```

- [ ] **Step 4: Run the focused tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_hash_api_key_is_deterministic --lib`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_create_tenant_and_api_key_persists_hash_only --test auth_integration_test -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the repository and key primitives**

```bash
git add guard-rail-engine/src/auth/mod.rs guard-rail-engine/src/auth/api_keys.rs guard-rail-engine/src/tenant/mod.rs guard-rail-engine/src/tenant/repository.rs guard-rail-engine/src/storage/postgres.rs guard-rail-engine/tests/auth_integration_test.rs
git commit -m "feat: add tenant repository and api key primitives"
```

## Task 3: In-Memory Tenant Cache And Startup Validation

**Files:**
- Create: `guard-rail-engine/src/tenant/cache.rs`
- Modify: `guard-rail-engine/src/main.rs`
- Modify: `guard-rail-engine/src/routes.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Modify: `guard-rail-engine/src/storage/postgres.rs`

- [ ] **Step 1: Write the failing cache and startup validation tests**

```rust
#[tokio::test]
async fn test_load_auth_cache_returns_only_active_keys_and_bindings() {
    let database_url =
        std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let repo = guard_rail_engine::tenant::repository::TenantRepository::new(pool.clone());
    let tenant = repo.create_tenant("acme").await.unwrap();
    let issued = repo.create_api_key(tenant.id, "primary").await.unwrap();
    repo.bind_route("test-route", tenant.id).await.unwrap();
    repo.revoke_api_key(issued.id, Some("rotated")).await.unwrap();

    let snapshot = repo.load_auth_snapshot().await.unwrap();
    assert!(snapshot.route_bindings.contains_key("test-route"));
    assert!(snapshot.api_keys.is_empty());
}

#[test]
fn test_validate_all_routes_bound_returns_error_for_unbound_route() {
    let routes = guard_rail_engine::routes::RouteTable::from_routes(vec![
        guard_rail_engine::routes::Route {
            id: "test-route".to_string(),
            path: "/v1/execute/test-route".to_string(),
            upstream: "http://upstream".to_string(),
            methods: vec!["POST".to_string()],
            policies: vec![],
            timeout_ms: 5000,
        },
    ]);

    let snapshot = guard_rail_engine::tenant::cache::TenantAuthSnapshot::default();
    let err = guard_rail_engine::tenant::cache::validate_all_routes_bound(&routes, &snapshot)
        .unwrap_err();
    assert!(err.contains("test-route"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_validate_all_routes_bound_returns_error_for_unbound_route --lib`
Expected: FAIL because `RouteTable::from_routes` and tenant cache helpers do not exist yet.

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_load_auth_cache_returns_only_active_keys_and_bindings --test auth_integration_test -- --exact`
Expected: FAIL because snapshot loading and bind/revoke helpers do not exist yet.

- [ ] **Step 3: Implement the tenant auth cache and startup validation**

```rust
#[derive(Debug, Clone, Default)]
pub struct TenantAuthSnapshot {
    pub route_bindings: std::collections::HashMap<String, uuid::Uuid>,
    pub api_keys: std::collections::HashMap<String, CachedApiKey>,
}

#[derive(Debug, Clone)]
pub struct CachedApiKey {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub name: String,
    pub key_prefix: String,
    pub tenant_status: String,
}

#[derive(Clone, Default)]
pub struct TenantAuthCache {
    inner: std::sync::Arc<tokio::sync::RwLock<TenantAuthSnapshot>>,
}

impl TenantAuthCache {
    pub async fn replace(&self, snapshot: TenantAuthSnapshot) {
        *self.inner.write().await = snapshot;
    }

    pub async fn snapshot(&self) -> TenantAuthSnapshot {
        self.inner.read().await.clone()
    }
}

impl crate::tenant::repository::TenantRepository {
    pub async fn bind_route(
        &self,
        route_id: &str,
        tenant_id: uuid::Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            insert into tenant_routes (route_id, tenant_id)
            values ($1, $2)
            on conflict (route_id) do update
            set tenant_id = excluded.tenant_id,
                updated_at = now()
            "#,
        )
        .bind(route_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke_api_key(
        &self,
        key_id: uuid::Uuid,
        reason: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "update api_keys set revoked_at = now(), revoked_reason = $2 where id = $1",
        )
        .bind(key_id)
        .bind(reason)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_auth_snapshot(
        &self,
    ) -> Result<crate::tenant::cache::TenantAuthSnapshot, sqlx::Error> {
        let route_rows = sqlx::query("select route_id, tenant_id from tenant_routes")
            .fetch_all(&self.pool)
            .await?;
        let key_rows = sqlx::query(
            r#"
            select
                api_keys.id,
                api_keys.tenant_id,
                api_keys.key_hash,
                api_keys.key_prefix,
                api_keys.name,
                api_keys.revoked_at,
                tenants.status as tenant_status
            from api_keys
            join tenants on tenants.id = api_keys.tenant_id
            where api_keys.revoked_at is null
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let route_bindings = route_rows
            .into_iter()
            .map(|row| {
                (
                    sqlx::Row::get::<String, _>(&row, "route_id"),
                    sqlx::Row::get::<uuid::Uuid, _>(&row, "tenant_id"),
                )
            })
            .collect();

        let api_keys = key_rows
            .into_iter()
            .map(|row| {
                let key_hash = sqlx::Row::get::<String, _>(&row, "key_hash");
                let cached = crate::tenant::cache::CachedApiKey {
                    id: sqlx::Row::get(&row, "id"),
                    tenant_id: sqlx::Row::get(&row, "tenant_id"),
                    name: sqlx::Row::get(&row, "name"),
                    key_prefix: sqlx::Row::get(&row, "key_prefix"),
                    tenant_status: sqlx::Row::get(&row, "tenant_status"),
                };
                (key_hash, cached)
            })
            .collect();

        Ok(crate::tenant::cache::TenantAuthSnapshot {
            route_bindings,
            api_keys,
        })
    }
}

pub fn validate_all_routes_bound(
    routes: &crate::routes::RouteTable,
    snapshot: &TenantAuthSnapshot,
) -> Result<(), String> {
    let unbound: Vec<String> = routes
        .route_ids()
        .into_iter()
        .filter(|route_id| !snapshot.route_bindings.contains_key(route_id))
        .collect();

    if unbound.is_empty() {
        Ok(())
    } else {
        Err(format!("Unbound executable routes: {}", unbound.join(", ")))
    }
}
```

```rust
impl RouteTable {
    pub fn from_routes(routes: Vec<Route>) -> Self {
        let by_id = routes
            .into_iter()
            .map(|route| (route.id.clone(), route))
            .collect();
        Self { by_id }
    }

    pub fn route_ids(&self) -> Vec<String> {
        let mut ids = self.by_id.keys().cloned().collect::<Vec<_>>();
        ids.sort();
        ids
    }
}
```

```rust
#[derive(Clone)]
pub struct AppState {
    pub routes: std::sync::Arc<tokio::sync::RwLock<RouteTable>>,
    pub policies: std::sync::Arc<tokio::sync::RwLock<PolicySet>>,
    pub http_client: reqwest::Client,
    pub audit_store: Option<crate::storage::postgres::PostgresAuditStore>,
    pub route_config_hash: String,
    pub policy_set_hash: String,
    pub admin_token: String,
    pub tenant_repo: crate::tenant::repository::TenantRepository,
    pub tenant_cache: crate::tenant::cache::TenantAuthCache,
    pub rate_limiter: crate::auth::rate_limit::TenantRateLimiter,
}

let tenant_repo = tenant::repository::TenantRepository::new(pool.clone());
let tenant_cache = tenant::cache::TenantAuthCache::default();
let auth_snapshot = tenant_repo.load_auth_snapshot().await?;
tenant::cache::validate_all_routes_bound(&route_table, &auth_snapshot)
    .map_err(|err| format!("Tenant binding validation failed: {err}"))?;
tenant_cache.replace(auth_snapshot).await;

let state = proxy::AppState {
    routes,
    policies,
    http_client,
    audit_store: Some(audit_store),
    route_config_hash,
    policy_set_hash,
    admin_token: app_config.admin.token.clone(),
    tenant_repo,
    tenant_cache,
    rate_limiter: auth::rate_limit::TenantRateLimiter::new(
        app_config.rate_limit.requests_per_minute,
        app_config.rate_limit.burst,
    ),
};
```

- [ ] **Step 4: Run the focused tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test test_validate_all_routes_bound_returns_error_for_unbound_route --lib`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_load_auth_cache_returns_only_active_keys_and_bindings --test auth_integration_test -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the cache and startup wiring**

```bash
git add guard-rail-engine/src/tenant/cache.rs guard-rail-engine/src/main.rs guard-rail-engine/src/routes.rs guard-rail-engine/src/proxy/mod.rs guard-rail-engine/src/storage/postgres.rs guard-rail-engine/tests/auth_integration_test.rs
git commit -m "feat: load tenant auth cache at startup"
```

## Task 4: Tenant Authentication, Rate Limiting, And Execution Audit Extension

**Files:**
- Create: `guard-rail-engine/src/auth/context.rs`
- Create: `guard-rail-engine/src/auth/rate_limit.rs`
- Modify: `guard-rail-engine/src/auth/middleware.rs`
- Modify: `guard-rail-engine/src/execution/mod.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Modify: `guard-rail-engine/src/storage/postgres.rs`
- Modify: `guard-rail-engine/src/logging.rs`
- Test: `guard-rail-engine/tests/auth_integration_test.rs`

- [ ] **Step 1: Write the failing auth-path integration tests**

```rust
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

struct Stage3TestApp {
    base_url: String,
    store: guard_rail_engine::storage::postgres::PostgresAuditStore,
    tenant_a_id: uuid::Uuid,
    tenant_a_key_id: uuid::Uuid,
    tenant_a_key: String,
    tenant_b_id: uuid::Uuid,
    tenant_b_key: String,
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

async fn start_stage3_test_app(
    requests_per_minute: u32,
    burst: u32,
) -> Stage3TestApp {
    let database_url =
        std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::query("truncate table execution_audit, tenant_routes, api_keys, tenants restart identity cascade")
        .execute(&pool)
        .await
        .unwrap();

    let repo = guard_rail_engine::tenant::repository::TenantRepository::new(pool.clone());
    let tenant_a = repo.create_tenant("tenant-a").await.unwrap();
    let tenant_b = repo.create_tenant("tenant-b").await.unwrap();
    let key_a = repo.create_api_key(tenant_a.id, "primary-a").await.unwrap();
    let key_b = repo.create_api_key(tenant_b.id, "primary-b").await.unwrap();
    repo.bind_route("test-route", tenant_a.id).await.unwrap();
    repo.bind_route("tenant-b-route", tenant_b.id).await.unwrap();

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

    let routes = guard_rail_engine::routes::RouteTable::load(&tmp.path().join("routes.yaml")).unwrap();
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
        route_config_hash: guard_rail_engine::audit::hash::hash_string("routes"),
        policy_set_hash: guard_rail_engine::audit::hash::hash_string("policies"),
        admin_token: "stage2-admin-token".to_string(),
        tenant_repo: repo.clone(),
        tenant_cache: tenant_cache.clone(),
        rate_limiter: guard_rail_engine::auth::rate_limit::TenantRateLimiter::new(
            requests_per_minute,
            burst,
        ),
    };

    let app = guard_rail_engine::proxy::build_router(state, "stage2-admin-token".to_string(), 1_048_576);
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

    let rows = harness.store.list_executions(
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
    assert_eq!(rows.items[0].auth_outcome.as_deref(), Some("missing_api_key"));
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_missing_api_key_returns_401_and_audits_event --test auth_integration_test -- --exact`
Expected: FAIL because tenant auth is not enforced and auth failures are not audited yet.

- [ ] **Step 3: Implement tenant auth context, rate limiting, and execution-record extensions**

```rust
#[derive(Debug, Clone)]
pub enum RequestAuthContext {
    Admin,
    Tenant {
        tenant_id: uuid::Uuid,
        api_key_id: uuid::Uuid,
        key_prefix: String,
    },
}

#[derive(Debug, Clone)]
pub enum AuditAccess {
    Admin,
    Tenant { tenant_id: uuid::Uuid },
}

#[derive(Debug, Clone)]
pub enum RequestAuthFailure {
    MissingApiKey,
    InvalidApiKey,
    RevokedApiKey {
        tenant_id: uuid::Uuid,
        api_key_id: uuid::Uuid,
    },
    TenantDisabled {
        tenant_id: uuid::Uuid,
        api_key_id: uuid::Uuid,
    },
}
```

```rust
#[derive(Debug, Clone, Default)]
pub struct TenantRateLimiter {
    inner: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<uuid::Uuid, BucketState>>>,
    requests_per_minute: u32,
    burst: u32,
}

#[derive(Debug, Clone)]
struct BucketState {
    tokens: f64,
    last_refill: std::time::Instant,
}

impl TenantRateLimiter {
    pub fn new(requests_per_minute: u32, burst: u32) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            requests_per_minute,
            burst,
        }
    }

    pub async fn allow(&self, tenant_id: uuid::Uuid) -> bool {
        let mut guard = self.inner.write().await;
        let state = guard.entry(tenant_id).or_insert_with(|| BucketState {
            tokens: self.burst as f64,
            last_refill: std::time::Instant::now(),
        });

        let now = std::time::Instant::now();
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        let refill_rate = self.requests_per_minute as f64 / 60.0;
        state.tokens = (state.tokens + elapsed * refill_rate).min(self.burst as f64);
        state.last_refill = now;

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub execution_id: String,
    pub execution_started_at: chrono::DateTime<chrono::Utc>,
    pub route_id: String,
    pub tenant_id: Option<uuid::Uuid>,
    pub api_key_id: Option<uuid::Uuid>,
    pub auth_outcome: Option<String>,
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
pub async fn authenticate_tenant_request(
    headers: &axum::http::HeaderMap,
    cache: &crate::tenant::cache::TenantAuthCache,
) -> Result<RequestAuthContext, RequestAuthFailure> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let raw_key = header
        .strip_prefix("Bearer ")
        .ok_or(RequestAuthFailure::MissingApiKey)?;
    let hashed = crate::auth::api_keys::hash_api_key(raw_key);
    let snapshot = cache.snapshot().await;
    let cached = snapshot
        .api_keys
        .get(&hashed)
        .ok_or(RequestAuthFailure::InvalidApiKey)?;

    if cached.revoked_at.is_some() {
        return Err(RequestAuthFailure::RevokedApiKey {
            tenant_id: cached.tenant_id,
            api_key_id: cached.id,
        });
    }

    if cached.tenant_status != "active" {
        return Err(RequestAuthFailure::TenantDisabled {
            tenant_id: cached.tenant_id,
            api_key_id: cached.id,
        });
    }

    Ok(RequestAuthContext::Tenant {
        tenant_id: cached.tenant_id,
        api_key_id: cached.id,
        key_prefix: cached.key_prefix.clone(),
    })
}
```

```rust
let auth_context = match crate::auth::middleware::authenticate_tenant_request(
    &headers,
    &state.tenant_cache,
)
.await
{
    Ok(ctx) => ctx,
    Err(outcome) => {
        let (tenant_id, api_key_id, auth_outcome) = match outcome {
            crate::auth::context::RequestAuthFailure::MissingApiKey => {
                (None, None, "missing_api_key".to_string())
            }
            crate::auth::context::RequestAuthFailure::InvalidApiKey => {
                (None, None, "invalid_api_key".to_string())
            }
            crate::auth::context::RequestAuthFailure::RevokedApiKey {
                tenant_id,
                api_key_id,
            } => (Some(tenant_id), Some(api_key_id), "revoked_api_key".to_string()),
            crate::auth::context::RequestAuthFailure::TenantDisabled {
                tenant_id,
                api_key_id,
            } => (Some(tenant_id), Some(api_key_id), "tenant_disabled".to_string()),
        };

        let record = ExecutionRecord {
            execution_id: execution_id.clone(),
            execution_started_at,
            route_id: route_id.clone(),
            tenant_id,
            api_key_id,
            auth_outcome: Some(auth_outcome),
            upstream_url: None,
            method: method.to_string(),
            source_ip: source_ip.clone(),
            content_type: headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(String::from),
            user_agent: headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .map(String::from),
            had_authorization_header: headers.contains_key("authorization"),
            request_size_bytes: body.len(),
            request_body_sha256: crate::audit::hash::hash_body(&body),
            verdict: crate::execution::ExecutionVerdict::Rejected,
            rejection_reason: Some("auth_failed".to_string()),
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
            latency_total_ms: total_start.elapsed().as_millis(),
            route_config_hash: state.route_config_hash.clone(),
            policy_set_hash: state.policy_set_hash.clone(),
        };
        spawn_emit_and_persist(record, state.audit_store.clone());
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
};
```

- [ ] **Step 4: Run the focused auth-path tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_missing_api_key_returns_401_and_audits_event --test auth_integration_test -- --exact`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_valid_key_for_other_tenant_route_returns_404 --test auth_integration_test -- --exact`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_tenant_rate_limit_returns_429_without_blocking_other_tenant --test auth_integration_test -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the execution-path enforcement**

```bash
git add guard-rail-engine/src/auth/context.rs guard-rail-engine/src/auth/rate_limit.rs guard-rail-engine/src/auth/middleware.rs guard-rail-engine/src/execution/mod.rs guard-rail-engine/src/proxy/mod.rs guard-rail-engine/src/storage/postgres.rs guard-rail-engine/src/logging.rs guard-rail-engine/tests/auth_integration_test.rs
git commit -m "feat: enforce tenant auth on execution requests"
```

## Task 5: Admin APIs And Immediate Cache Refresh

**Files:**
- Create: `guard-rail-engine/src/tenant/api.rs`
- Modify: `guard-rail-engine/src/main.rs`
- Modify: `guard-rail-engine/src/auth/middleware.rs`
- Modify: `guard-rail-engine/src/tenant/repository.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Test: `guard-rail-engine/tests/auth_integration_test.rs`

- [ ] **Step 1: Write the failing admin API integration tests**

```rust
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_admin_can_create_tenant_issue_key_and_bind_route --test auth_integration_test -- --exact`
Expected: FAIL because the Stage 3 admin routes do not exist yet.

- [ ] **Step 3: Implement the admin APIs and immediate refresh behavior**

```rust
#[derive(Debug, serde::Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct BindRouteRequest {
    pub route_id: String,
}

pub async fn create_tenant(
    State(state): State<crate::proxy::AppState>,
    Json(request): Json<CreateTenantRequest>,
) -> Result<Json<crate::tenant::repository::Tenant>, StatusCode> {
    let tenant = state
        .tenant_repo
        .create_tenant(&request.name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state.refresh_tenant_auth_cache().await?;
    Ok(Json(tenant))
}
```

```rust
impl AppState {
    pub async fn refresh_tenant_auth_cache(&self) -> Result<(), axum::http::StatusCode> {
        let snapshot = self
            .tenant_repo
            .load_auth_snapshot()
            .await
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        crate::tenant::cache::validate_all_routes_bound(&self.routes.read().await, &snapshot)
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        self.tenant_cache.replace(snapshot).await;
        Ok(())
    }
}
```

```rust
let admin_routes = axum::Router::new()
    .route("/v1/admin/tenants", axum::routing::post(crate::tenant::api::create_tenant).get(crate::tenant::api::list_tenants))
    .route("/v1/admin/tenants/{tenant_id}/keys", axum::routing::post(crate::tenant::api::create_api_key).get(crate::tenant::api::list_api_keys))
    .route("/v1/admin/tenants/{tenant_id}/routes", axum::routing::post(crate::tenant::api::bind_route))
    .route_layer(axum::middleware::from_fn_with_state(
        admin_token.clone(),
        crate::auth::middleware::require_admin_token,
    ))
    .with_state(state.clone());
```

- [ ] **Step 4: Run the focused admin tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_admin_can_create_tenant_issue_key_and_bind_route --test auth_integration_test -- --exact`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_revoked_key_stops_working_immediately_after_admin_write --test auth_integration_test -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the admin control surface**

```bash
git add guard-rail-engine/src/tenant/api.rs guard-rail-engine/src/main.rs guard-rail-engine/src/auth/middleware.rs guard-rail-engine/src/tenant/repository.rs guard-rail-engine/src/proxy/mod.rs guard-rail-engine/tests/auth_integration_test.rs
git commit -m "feat: add stage 3 tenant admin APIs"
```

## Task 6: Role-Aware Audit API

**Files:**
- Modify: `guard-rail-engine/src/audit/api.rs`
- Modify: `guard-rail-engine/src/auth/context.rs`
- Modify: `guard-rail-engine/src/auth/middleware.rs`
- Modify: `guard-rail-engine/src/storage/postgres.rs`
- Test: `guard-rail-engine/tests/audit_api_test.rs`

- [ ] **Step 1: Write the failing audit visibility tests**

```rust
#[tokio::test]
async fn test_tenant_audit_list_returns_only_owned_rows() {
    struct AuditListView {
        items: serde_json::Value,
    }

    impl AuditListView {
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

    let response = reqwest::Client::new()
        .get(format!("{}/v1/audit/executions", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_a_key))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 1);
    assert_eq!(json["items"][0]["tenant_id"], harness.tenant_a_id.to_string());
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

    let admin_list: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/v1/audit/executions?tenant_id={}", harness.base_url, harness.tenant_b_id))
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_tenant_audit_list_returns_only_owned_rows --test audit_api_test -- --exact`
Expected: FAIL because audit endpoints currently rely on admin-only bearer auth and return all rows.

- [ ] **Step 3: Implement role-aware audit access**

```rust
#[derive(Debug, Deserialize, Default)]
pub struct AuditListQuery {
    pub tenant_id: Option<uuid::Uuid>,
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
    Extension(access): Extension<crate::auth::context::AuditAccess>,
    Query(query): Query<AuditListQuery>,
) -> Result<Json<AuditListResponse>, StatusCode> {
    let store = state.audit_store.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let page = store
        .list_executions(query, access)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(page))
}
```

```rust
pub async fn list_executions(
    &self,
    query: crate::audit::api::AuditListQuery,
    access: crate::auth::context::AuditAccess,
) -> Result<crate::audit::api::AuditListResponse, sqlx::Error> {
    match access {
        crate::auth::context::AuditAccess::Admin => {
            self.list_executions_for_tenant(query.tenant_id, query).await
        }
        crate::auth::context::AuditAccess::Tenant { tenant_id } => {
            self.list_executions_for_tenant(Some(tenant_id), query).await
        }
    }
}

pub async fn list_executions_for_tenant(
    &self,
    tenant_filter: Option<uuid::Uuid>,
    query: crate::audit::api::AuditListQuery,
) -> Result<crate::audit::api::AuditListResponse, sqlx::Error> {
    let limit = query.limit.unwrap_or(50).min(1000);
    let offset = query.cursor.unwrap_or(0);
    let rows = sqlx::query(
        r#"
        select execution_id, execution_started_at, route_id, tenant_id, api_key_id, auth_outcome,
               upstream_url, method, source_ip, content_type, user_agent, had_authorization_header,
               request_size_bytes, request_body_sha256, verdict, rejection_reason,
               matched_policy_name, matched_rule_field, matched_rule_condition, matched_rule_severity,
               violation_value_hash, violation_value_preview, upstream_status, forward_error,
               latency_inspect_us, latency_forward_ms, latency_total_ms,
               route_config_hash, policy_set_hash, previous_hash, record_hash
        from execution_audit
        where ($1::uuid is null or tenant_id = $1)
        order by execution_started_at desc
        limit $2 offset $3
        "#,
    )
    .bind(tenant_filter)
    .bind(limit)
    .bind(offset)
    .fetch_all(&self.pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|row| map_execution_row(&row))
        .collect::<Vec<_>>();
    let next_cursor = if items.len() as i64 == limit {
        Some(offset + limit)
    } else {
        None
    };

    Ok(crate::audit::api::AuditListResponse {
        total: items.len() as i64,
        items,
        next_cursor,
    })
}

fn map_execution_row(row: &sqlx::postgres::PgRow) -> ExecutionAuditRow {
    ExecutionAuditRow {
        execution_id: sqlx::Row::get(row, "execution_id"),
        execution_started_at: sqlx::Row::get(row, "execution_started_at"),
        route_id: sqlx::Row::get(row, "route_id"),
        tenant_id: sqlx::Row::get(row, "tenant_id"),
        api_key_id: sqlx::Row::get(row, "api_key_id"),
        auth_outcome: sqlx::Row::get(row, "auth_outcome"),
        upstream_url: sqlx::Row::get(row, "upstream_url"),
        method: sqlx::Row::get(row, "method"),
        source_ip: sqlx::Row::get(row, "source_ip"),
        content_type: sqlx::Row::get(row, "content_type"),
        user_agent: sqlx::Row::get(row, "user_agent"),
        had_authorization_header: sqlx::Row::get(row, "had_authorization_header"),
        request_size_bytes: sqlx::Row::get::<i64, _>(row, "request_size_bytes") as usize,
        request_body_sha256: sqlx::Row::get(row, "request_body_sha256"),
        verdict: sqlx::Row::get(row, "verdict"),
        rejection_reason: sqlx::Row::get(row, "rejection_reason"),
        matched_policy_name: sqlx::Row::get(row, "matched_policy_name"),
        matched_rule_field: sqlx::Row::get(row, "matched_rule_field"),
        matched_rule_condition: sqlx::Row::get(row, "matched_rule_condition"),
        matched_rule_severity: sqlx::Row::get(row, "matched_rule_severity"),
        violation_value_hash: sqlx::Row::get(row, "violation_value_hash"),
        violation_value_preview: sqlx::Row::get(row, "violation_value_preview"),
        upstream_status: sqlx::Row::get::<Option<i32>, _>(row, "upstream_status").map(|v| v as u16),
        forward_error: sqlx::Row::get(row, "forward_error"),
        latency_inspect_us: sqlx::Row::get::<i64, _>(row, "latency_inspect_us") as u128,
        latency_forward_ms: sqlx::Row::get::<Option<i64>, _>(row, "latency_forward_ms").map(|v| v as u128),
        latency_total_ms: sqlx::Row::get::<i64, _>(row, "latency_total_ms") as u128,
        route_config_hash: sqlx::Row::get(row, "route_config_hash"),
        policy_set_hash: sqlx::Row::get(row, "policy_set_hash"),
        previous_hash: sqlx::Row::get(row, "previous_hash"),
        record_hash: sqlx::Row::get(row, "record_hash"),
    }
}
```

```rust
pub async fn require_audit_access(
    State(state): State<crate::proxy::AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if crate::auth::middleware::has_valid_admin_token(request.headers(), &state.admin_token) {
        let mut request = request;
        request.extensions_mut().insert(crate::auth::context::AuditAccess::Admin);
        return Ok(next.run(request).await);
    }

    let tenant = crate::auth::middleware::authenticate_tenant_request(
        request.headers(),
        &state.tenant_cache,
    )
    .await
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let mut request = request;
    if let crate::auth::context::RequestAuthContext::Tenant { tenant_id, .. } = tenant {
        request
            .extensions_mut()
            .insert(crate::auth::context::AuditAccess::Tenant { tenant_id });
    }
    Ok(next.run(request).await)
}

pub fn has_valid_admin_token(headers: &axum::http::HeaderMap, expected_token: &str) -> bool {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    header == format!("Bearer {}", expected_token)
}
```

- [ ] **Step 4: Run the focused audit visibility tests to verify they pass**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_tenant_audit_list_returns_only_owned_rows --test audit_api_test -- --exact`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test test_tenant_audit_detail_for_other_tenant_returns_404 --test audit_api_test -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the role-aware audit API**

```bash
git add guard-rail-engine/src/audit/api.rs guard-rail-engine/src/auth/context.rs guard-rail-engine/src/auth/middleware.rs guard-rail-engine/src/storage/postgres.rs guard-rail-engine/tests/audit_api_test.rs
git commit -m "feat: add tenant-scoped audit access"
```

## Task 7: Full Verification And Stage 3 Closeout

**Files:**
- Modify: `guard-rail-engine/tests/auth_integration_test.rs`
- Modify: `guard-rail-engine/tests/audit_api_test.rs`
- Modify: `guard-rail-engine/config/config.yaml`

- [ ] **Step 1: Fill in the remaining Stage 3 integration coverage**

```rust
#[tokio::test]
async fn test_disabled_tenant_loses_audit_read_access() {
    let harness = start_stage3_test_app(120, 30).await;

    reqwest::Client::new()
        .post(format!(
            "{}/v1/admin/tenants/{}/disable",
            harness.base_url, harness.tenant_a_id
        ))
        .header("authorization", "Bearer stage2-admin-token")
        .send()
        .await
        .unwrap();

    let response = reqwest::Client::new()
        .get(format!("{}/v1/audit/executions", harness.base_url))
        .header("authorization", format!("Bearer {}", harness.tenant_a_key))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_auth_failure_for_unknown_key_is_admin_visible_only() {
    let harness = start_stage3_test_app(120, 30).await;

    reqwest::Client::new()
        .post(format!("{}/v1/execute/test-route", harness.base_url))
        .header("authorization", "Bearer grk_invalid")
        .header("content-type", "application/json")
        .body(r#"{"ok":true}"#)
        .send()
        .await
        .unwrap();

    let admin = AuditListView {
        items: reqwest::Client::new()
            .get(format!("{}/v1/audit/executions", harness.base_url))
            .header("authorization", "Bearer stage2-admin-token")
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap()["items"]
            .clone(),
    };
    let tenant = AuditListView {
        items: reqwest::Client::new()
            .get(format!("{}/v1/audit/executions", harness.base_url))
            .header("authorization", format!("Bearer {}", harness.tenant_a_key))
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap()["items"]
            .clone(),
    };

    assert!(admin.contains_auth_outcome("invalid_api_key"));
    assert!(!tenant.contains_auth_outcome("invalid_api_key"));
}
```

- [ ] **Step 2: Run the focused regression tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test --test integration_test`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test --test auth_integration_test`
Expected: PASS

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test --test audit_api_test`
Expected: PASS

- [ ] **Step 3: Run the full backend suite**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:55432/guardrail cargo test`
Expected: PASS

- [ ] **Step 4: Commit the final verification and Stage 3 test coverage**

```bash
git add guard-rail-engine/tests/auth_integration_test.rs guard-rail-engine/tests/audit_api_test.rs guard-rail-engine/config/config.yaml
git commit -m "test: complete stage 3 tenant isolation coverage"
```
