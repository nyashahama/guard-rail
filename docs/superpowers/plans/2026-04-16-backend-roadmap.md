# Guard Rail Backend Multi-Stage Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-baseline the Guard Rail backend against the current Rust codebase and define an execution plan for stages 1 through 5 that can be implemented incrementally without rewriting the core proxy.

**Architecture:** Keep the existing single `axum` binary and extend it in layers. Stage 2 introduces a shared execution record model, PostgreSQL persistence, and a tamper-evident audit API. Stage 3 adds tenant and API key boundaries around the existing route and policy runtime. Stage 4 adds replayable artifacts and deterministic re-evaluation. Stage 5 hardens the service for production with observability, shutdown, and deployment support.

**Tech Stack:** Rust, axum, tokio, reqwest, serde/serde_json/serde_yaml, PostgreSQL, `sqlx`, `sha2`, `hex`, `tracing`, `tracing-subscriber`, OpenTelemetry, Prometheus

---

## Verification Snapshot

- [x] Stage 1 code exists under `guard-rail-engine/` and matches the original stage 1 spec closely.
- [x] `cargo test` passes in `/home/nyasha-hama/projects/guard-rail/guard-rail-engine`.
- [x] Current backend boundaries are usable for extension:
  - `src/proxy/mod.rs` owns request orchestration.
  - `src/policy/` owns policy loading and evaluation.
  - `src/logging.rs` owns execution log serialization.
  - `src/reload.rs` owns file-watch reload behavior.
- [x] Current constraints that affect later stages:
  - Routes and policy bindings are YAML-backed and globally scoped.
  - Execution logging is stdout-only and coupled to `ExecutionLog`.
  - There is no persistence abstraction yet.
  - There is no auth, tenant, or admin API surface yet.

## Cross-Stage Decisions

- [ ] Introduce a shared `ExecutionRecord` domain model before adding database writes.
  - Reason: stdout logging, PostgreSQL persistence, replay capture, and metrics should all consume the same canonical execution object.
- [ ] Keep YAML routes and policy definitions through Stage 2.
  - Reason: Stage 2 is about durable auditability, not configuration management.
- [ ] Keep Stage 2 audit storage lighter than Stage 4 replay storage.
  - Stage 2 should persist request metadata, policy/version hashes, verdicts, latencies, and tamper-evident digests.
  - Stage 4 should add full replay artifacts: request body, selected headers, response body, and policy snapshot.
- [ ] Make every new API additive under `/v1/`.
  - Current execution route remains `/v1/execute/{route_id}`.
  - New audit, tenant, admin, and replay endpoints should not break stage 1 callers.
- [ ] Use PostgreSQL migrations from the first DB stage onward.
  - Pick `sqlx` for migrations and runtime queries so later stages reuse the same connection and migration path.

## Stage 1: Core Proxy + Policy Engine

**Status:** Complete and verified.

**Keep:**
- `guard-rail-engine/src/proxy/mod.rs`
- `guard-rail-engine/src/policy/`
- `guard-rail-engine/src/routes.rs`
- `guard-rail-engine/src/reload.rs`
- `guard-rail-engine/tests/integration_test.rs`

**Stage 1 close-out tasks:**
- [ ] Document Stage 1 as the frozen baseline in the main backend README or a backend-specific README.
- [ ] Stop adding new behavior directly to `ExecutionLog`; future stages should extend a shared execution model instead.
- [ ] Treat the current passing test suite as the regression floor before Stage 2 work starts.

**Verification commands:**
- [ ] Run `cargo test`
- [ ] Run `cargo run -- --config ./config/config.yaml`
- [ ] Verify `/health` returns `200 ok`
- [ ] Send one blocked and one allowed request through `/v1/execute/{route_id}`

**Exit criteria:**
- [ ] Stage 1 remains green while Stage 2 branch work begins.

## Stage 2: Storage + Audit Trail

**Goal:** Persist every execution summary in PostgreSQL, generate tamper-evident hashes, and expose a queryable audit API without changing the core proxy contract.

**Rethink applied:**
- [ ] Do not capture full response bodies yet.
- [ ] Persist enough for compliance and investigation now: execution identity, route, source IP, method, request body hash, request size, verdict, matched policy/rule, upstream status, latencies, forward error, and policy set hash.
- [ ] Make the audit trail tamper-evident by chaining hashes: `record_hash` over canonical fields and `previous_hash` from the prior execution row.

**Files:**
- Create: `guard-rail-engine/migrations/0001_create_execution_audit.sql`
- Create: `guard-rail-engine/src/execution/mod.rs`
- Create: `guard-rail-engine/src/storage/mod.rs`
- Create: `guard-rail-engine/src/storage/postgres.rs`
- Create: `guard-rail-engine/src/audit/mod.rs`
- Create: `guard-rail-engine/src/audit/hash.rs`
- Create: `guard-rail-engine/src/audit/api.rs`
- Create: `guard-rail-engine/tests/audit_api_test.rs`
- Modify: `guard-rail-engine/Cargo.toml`
- Modify: `guard-rail-engine/src/config.rs`
- Modify: `guard-rail-engine/src/lib.rs`
- Modify: `guard-rail-engine/src/main.rs`
- Modify: `guard-rail-engine/src/logging.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`

**Execution steps:**
- [ ] Add database configuration to `src/config.rs`.
  - Add a `database` section with `url`, `max_connections`, and `run_migrations_on_start`.
  - Add env overrides for database settings.
- [ ] Add PostgreSQL dependencies in `Cargo.toml`.
  - Add `sqlx` with `runtime-tokio-rustls`, `postgres`, `uuid`, `chrono`, `json`, and `migrate` features.
  - Add `sha2` and `hex` for audit hashing.
- [ ] Create `src/execution/mod.rs` with a canonical execution record.
  - This record should be built once in the proxy and reused by logging, storage, and later replay code.
- [ ] Refactor `src/logging.rs` to log from the shared execution record rather than owning the only execution shape.
- [ ] Create `migrations/0001_create_execution_audit.sql`.
  - Tables:
    - `execution_audit`
    - columns for execution metadata, request hash, policy hash, `previous_hash`, `record_hash`, and timestamps
  - Indexes:
    - `execution_id`
    - `route_id, created_at desc`
    - `verdict, created_at desc`
- [ ] Create `src/storage/postgres.rs` to own pool creation and execution inserts.
- [ ] Create `src/audit/hash.rs` to canonicalize execution fields and compute chained SHA-256 hashes.
- [ ] Update `src/main.rs` to create a pool, run migrations, and register audit routes.
- [ ] Update `src/proxy/mod.rs` so every request emits:
  - a shared execution record
  - stdout JSON log
  - best-effort persisted audit row
- [ ] Decide and implement failure policy for audit writes.
  - Recommendation: if proxy handling succeeded but DB write fails, return the original client response and log the persistence failure; do not turn audit outages into proxy outages in Stage 2.
- [ ] Create `src/audit/api.rs` with read-only endpoints:
  - `GET /v1/audit/executions`
  - `GET /v1/audit/executions/{execution_id}`
  - filter support for `route_id`, `verdict`, `from`, `to`, and `limit`
- [ ] Add integration tests for audit persistence and list/detail APIs in `tests/audit_api_test.rs`.

**Verification commands:**
- [ ] Run `cargo test`
- [ ] Run `sqlx migrate run`
- [ ] Send one allowed and one blocked execution, then query `GET /v1/audit/executions`
- [ ] Verify stored `record_hash` and `previous_hash` chain consistency for at least two consecutive executions

**Exit criteria:**
- [ ] Every request that reaches proxy evaluation creates an audit row.
- [ ] Audit queries return persisted rows without reading stdout logs.
- [ ] Hash chaining proves row order tampering would be detectable.

## Stage 3: Tenant Management + API Keys

**Goal:** Support multiple customers safely by introducing tenants, hashed API keys, tenant-scoped access control, and tenant-scoped audit visibility.

**Rethink applied:**
- [ ] Keep YAML route and policy definitions for now, but add tenant ownership and route-to-tenant binding in PostgreSQL.
- [ ] Stage 3 admin APIs should manage tenants and API keys, not a full policy authoring system.
- [ ] Enforce tenant scoping in read APIs before adding any new write-heavy admin surface.

**Files:**
- Create: `guard-rail-engine/migrations/0002_create_tenants_and_api_keys.sql`
- Create: `guard-rail-engine/migrations/0003_create_tenant_routes.sql`
- Create: `guard-rail-engine/src/auth/mod.rs`
- Create: `guard-rail-engine/src/auth/api_keys.rs`
- Create: `guard-rail-engine/src/auth/middleware.rs`
- Create: `guard-rail-engine/src/tenant/mod.rs`
- Create: `guard-rail-engine/src/tenant/api.rs`
- Create: `guard-rail-engine/src/tenant/repository.rs`
- Create: `guard-rail-engine/tests/auth_integration_test.rs`
- Modify: `guard-rail-engine/src/config.rs`
- Modify: `guard-rail-engine/src/main.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Modify: `guard-rail-engine/src/storage/postgres.rs`
- Modify: `guard-rail-engine/src/audit/api.rs`
- Modify: `guard-rail-engine/src/routes.rs`

**Execution steps:**
- [ ] Add migrations for `tenants`, `api_keys`, and `tenant_routes`.
  - Store API key hashes, not raw keys.
  - Store a short key prefix for operator lookup.
- [ ] Implement `src/auth/api_keys.rs`.
  - Generate operator-visible keys once.
  - Persist only a hash and metadata.
- [ ] Implement `src/auth/middleware.rs`.
  - Require an API key on protected routes.
  - Resolve tenant context before proxy execution.
- [ ] Extend `src/routes.rs` or a new repository layer to map a route to exactly one tenant.
- [ ] Extend the shared execution record so tenant identity is attached to every audit row.
- [ ] Update audit APIs to enforce tenant scoping.
  - Tenant callers can only see their own rows.
  - Admin callers can filter across tenants.
- [ ] Add admin endpoints in `src/tenant/api.rs`:
  - `POST /v1/admin/tenants`
  - `POST /v1/admin/tenants/{tenant_id}/keys`
  - `GET /v1/admin/tenants`
  - `GET /v1/admin/tenants/{tenant_id}/keys`
- [ ] Add rate limiting per tenant.
  - Keep it simple at first: in-memory token bucket keyed by tenant ID.
  - Defer distributed rate limiting to Stage 5 unless scale forces it earlier.
- [ ] Add integration coverage for:
  - missing API key
  - invalid API key
  - valid API key for wrong tenant route
  - tenant-scoped audit reads

**Verification commands:**
- [ ] Run `cargo test`
- [ ] Create two tenants and one API key each
- [ ] Verify tenant A cannot execute or read audit rows for tenant B
- [ ] Load test a single tenant enough to verify rate limiting is enforced

**Exit criteria:**
- [ ] Every execution is attributable to a tenant.
- [ ] API keys are hashed at rest.
- [ ] Audit and execution routes are tenant-isolated.

## Stage 4: Replay Engine

**Goal:** Allow deterministic re-evaluation of past executions against current or stored policy versions by capturing replayable artifacts and exposing replay APIs.

**Rethink applied:**
- [ ] This is the first stage that should capture full replay artifacts.
- [ ] Store request body, selected request headers, response status, response body, and policy snapshot references.
- [ ] Default replay mode should evaluate policies without forwarding upstream; live re-forwarding should be explicit and separately authorized.

**Files:**
- Create: `guard-rail-engine/migrations/0004_create_execution_artifacts.sql`
- Create: `guard-rail-engine/src/replay/mod.rs`
- Create: `guard-rail-engine/src/replay/api.rs`
- Create: `guard-rail-engine/src/replay/engine.rs`
- Create: `guard-rail-engine/src/replay/snapshot.rs`
- Create: `guard-rail-engine/tests/replay_integration_test.rs`
- Modify: `guard-rail-engine/src/execution/mod.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Modify: `guard-rail-engine/src/audit/api.rs`
- Modify: `guard-rail-engine/src/storage/postgres.rs`
- Modify: `guard-rail-engine/src/policy/mod.rs`
- Modify: `guard-rail-engine/src/policy/engine.rs`

**Execution steps:**
- [ ] Add artifact storage tables for request and response capture.
  - Use `jsonb` for JSON payloads and header maps when possible.
  - Keep a bytea fallback only if non-JSON forwarding becomes a requirement.
- [ ] Extend execution persistence to save replay artifacts for completed executions.
- [ ] Add policy set versioning.
  - Persist a hash of the active route and policy files for each execution.
  - Expose the hash in audit detail responses.
- [ ] Implement `src/replay/engine.rs`.
  - Load a stored execution.
  - Reconstruct the evaluation input.
  - Run policy evaluation against either:
    - the stored policy snapshot hash
    - the current loaded policy set
- [ ] Implement `src/replay/api.rs`:
  - `POST /v1/replay/executions/{execution_id}`
  - `GET /v1/replay/executions/{execution_id}`
- [ ] Include diff output in replay results.
  - Show original verdict vs replay verdict.
  - Show the blocking policy/rule change when different.
- [ ] Add integration tests for:
  - replaying a blocked request against unchanged policies
  - replaying an allowed request after policy changes
  - replay without upstream forwarding by default

**Verification commands:**
- [ ] Run `cargo test`
- [ ] Persist one blocked and one allowed execution with artifacts
- [ ] Replay both against current policy state
- [ ] Verify replay does not call the upstream unless explicitly requested

**Exit criteria:**
- [ ] Replay results are deterministic for unchanged inputs and policy versions.
- [ ] Operators can explain why a verdict changed between original run and replay.

## Stage 5: Observability + Production Hardening

**Goal:** Make the service operable in production with metrics, traces, graceful shutdown, readiness semantics, packaging, and failure-mode visibility.

**Rethink applied:**
- [ ] Stage 5 should harden the system already built, not redesign it.
- [ ] Keep operational concerns modular and additive.

**Files:**
- Create: `guard-rail-engine/src/observability/mod.rs`
- Create: `guard-rail-engine/src/observability/metrics.rs`
- Create: `guard-rail-engine/src/observability/tracing.rs`
- Create: `guard-rail-engine/src/shutdown.rs`
- Create: `guard-rail-engine/Dockerfile`
- Create: `guard-rail-engine/.dockerignore`
- Create: `guard-rail-engine/deploy/systemd/guard-rail-engine.service`
- Create: `guard-rail-engine/tests/smoke_test.rs`
- Modify: `guard-rail-engine/src/main.rs`
- Modify: `guard-rail-engine/src/logging.rs`
- Modify: `guard-rail-engine/src/config.rs`
- Modify: `guard-rail-engine/src/proxy/mod.rs`
- Modify: `README.md`

**Execution steps:**
- [ ] Add a readiness endpoint separate from liveness.
  - `/health` can stay basic.
  - Add `/ready` that verifies DB connectivity and route/policy state loaded.
- [ ] Add graceful shutdown handling in `src/shutdown.rs`.
  - Drain in-flight requests before process exit.
- [ ] Add Prometheus metrics in `src/observability/metrics.rs`.
  - Request count
  - block count
  - upstream failure count
  - request latency
  - audit persistence failures
- [ ] Add OpenTelemetry tracing hooks in `src/observability/tracing.rs`.
  - Include `execution_id`, `tenant_id`, `route_id`, and verdict as span attributes.
- [ ] Convert ad hoc `println!` execution output into `tracing`-compatible structured events while preserving machine-readable logs.
- [ ] Add deployment artifacts.
  - Dockerfile
  - systemd unit
  - documented environment variables
- [ ] Add smoke tests for startup, readiness, and a simple proxied execution.

**Verification commands:**
- [ ] Run `cargo test`
- [ ] Build the Docker image
- [ ] Start the service with PostgreSQL and verify `/health`, `/ready`, and metrics exposure
- [ ] Kill the service during load and confirm graceful shutdown behavior

**Exit criteria:**
- [ ] The service exposes health, readiness, metrics, and traces.
- [ ] Operators can deploy and restart it without dropping requests blindly.

## Suggested Delivery Order

- [ ] Implement Stage 2 first with no tenant assumptions in the request contract.
- [ ] Implement Stage 3 next and attach tenant identity to the Stage 2 audit model.
- [ ] Implement Stage 4 only after Stage 2 persistence and Stage 3 auth boundaries are stable.
- [ ] Leave Stage 5 last unless an operational gap blocks safe testing earlier.

## Risks To Watch

- [ ] Avoid mixing full replay artifact storage into Stage 2; that will blur stage boundaries and create avoidable storage cost early.
- [ ] Avoid moving routes and policy definitions into PostgreSQL in Stage 3 unless there is a real product need for runtime policy authoring.
- [ ] Avoid making audit persistence synchronous in the client response path until there is evidence compliance requires fail-closed behavior.
- [ ] Avoid adding distributed rate limiting before a single-node tenant model is proven.
