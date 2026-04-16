# Guard Rail Backend — Stage 2: Storage + Audit Trail

## Overview

Stage 2 extends the existing Rust gateway from Stage 1 by adding PostgreSQL-backed execution persistence and a tamper-evident audit trail. Guard Rail continues to inspect requests, enforce YAML-defined policies, and forward allowed requests upstream, but now each real execution is also recorded durably in a queryable audit ledger.

This stage is deliberately narrower than replay or multi-tenancy:

- It adds durable execution summaries, not full request/response archival
- It adds an operator-facing audit API, not customer/tenant management
- It adds a tamper-evident hash chain, not mutable audit workflows

The result after Stage 2 is a working proxy with a real compliance story: every configured-route execution can be queried from PostgreSQL and verified against an append-only integrity chain.

## Scope

### Included

- PostgreSQL connection management and migrations
- Durable persistence for execution audit rows
- One canonical execution record shared by logging and storage
- Per-row SHA-256 audit hashing with `previous_hash` chaining
- Read-only audit API
- Admin-only integrity verification endpoint
- Simple operator authentication for audit endpoints via configured admin token

### Explicitly Excluded

- Full request or response body storage
- General request header persistence
- Replay APIs
- Multi-tenant isolation
- Customer API keys
- Audit row deletion or retention automation
- Automatic migrations on startup

Those capabilities belong to later stages.

## Design Principles

1. **Append-only audit ledger**
   Each execution creates at most one final immutable audit row after the request outcome is known. No insert-then-update flow.

2. **Fail open on audit persistence failure**
   Guard Rail returns the normal proxy response even if the PostgreSQL insert fails. Audit write failures are surfaced through structured logs and later observability, not by breaking live traffic.

3. **Global ledger ordering**
   The audit chain is global across all executions in database insert order, not per route and not by timestamp.

4. **Minimal sensitive data retention**
   Stage 2 stores metadata, hashes, and carefully limited previews. It does not persist raw bodies, arbitrary headers, or full matched rule payloads.

5. **Human-readable and cryptographically anchored**
   Audit rows must be easy for operators to understand, while also carrying enough hashes to prove what configuration and payload produced the result.

## Baseline From Stage 1

Stage 1 already provides:

- `POST /v1/execute/{route_id}`
- YAML-backed routes and policies
- JSON policy evaluation
- allow, block, reject, and bad-gateway responses
- stdout execution logging

Stage 2 preserves that request contract. The primary architectural change is that execution data is normalized into a canonical in-memory record and then written both to logs and PostgreSQL.

## Architecture

Guard Rail remains a single async Rust service using `axum`.

### New Runtime Components

- **Execution record model**
  A shared `ExecutionRecord` domain struct that captures the final outcome of a request.

- **PostgreSQL storage layer**
  A focused module responsible for connection pooling, migrations, audit row inserts, audit queries, and integrity range reads.

- **Audit hash module**
  Computes `request_body_sha256`, `route_config_hash`, `policy_set_hash`, `previous_hash`, and `record_hash`.

- **Audit API module**
  Exposes read-only audit queries and integrity verification endpoints.

- **Admin auth middleware**
  Protects Stage 2 audit endpoints with a single configured bearer token.

### Request Flow

```
Client
  → POST /v1/execute/{route_id}
  → Route lookup
  → Method validation
  → JSON parsing if possible
  → Policy evaluation
  → Forward allowed requests upstream when applicable
  → Build final ExecutionRecord in memory
  → Emit structured stdout log
  → Attempt synchronous PostgreSQL insert with short timeout
    → On success: audit row durably recorded
    → On failure: log persistence error, return original proxy response
  → Return response to caller
```

### What Counts As An Audit Row

Persist to `execution_audit` only when the request matched a configured Guard Rail route.

Included:

- invalid JSON on a known route
- method not allowed on a known route
- blocked requests
- allowed requests successfully forwarded
- allowed requests with upstream failure

Excluded:

- `404 route not found`

Unknown-route traffic is application noise, not a Guard Rail execution event.

## Canonical Execution Record

Every request that reaches a known route is normalized into one final `ExecutionRecord`.

### Core Fields

- `execution_id`
- `execution_started_at`
- `route_id`
- `upstream_url`
- `method`
- `source_ip`
- `content_type`
- `user_agent` if present
- `had_authorization_header`
- `request_size_bytes`
- `request_body_sha256`
- `verdict`
- `rejection_reason` for rejected requests
- `matched_policy_name`
- `matched_rule_field`
- `matched_rule_condition`
- `matched_rule_severity`
- `violation_value_hash`
- `violation_value_preview`
- `upstream_status`
- `forward_error`
- `latency_inspect_us`
- `latency_forward_ms`
- `latency_total_ms`
- `route_config_hash`
- `policy_set_hash`

### Verdicts

- `REJECTED`
- `BLOCKED`
- `ALLOWED`

`REJECTED` includes requests like invalid JSON and method mismatch on a configured route.  
`ALLOWED` covers both successful upstream forwarding and forwarding failures, with the latter indicated by `forward_error`.

## PostgreSQL Schema

Stage 2 adds a single primary ledger table.

### `execution_audit`

- `id` — monotonic primary key used for ledger order
- `execution_id` — unique external execution identifier (`GR-EXE-{uuid}`)
- `execution_started_at` — when Guard Rail started handling the request
- `audit_persisted_at` — when the row was durably inserted
- `route_id`
- `upstream_url`
- `method`
- `source_ip`
- `content_type`
- `user_agent`
- `had_authorization_header`
- `request_size_bytes`
- `request_body_sha256`
- `verdict`
- `rejection_reason`
- `matched_policy_name`
- `matched_rule_field`
- `matched_rule_condition`
- `matched_rule_severity`
- `violation_value_hash`
- `violation_value_preview`
- `upstream_status`
- `forward_error`
- `latency_inspect_us`
- `latency_forward_ms`
- `latency_total_ms`
- `route_config_hash`
- `policy_set_hash`
- `previous_hash`
- `record_hash`

### Indexes

- unique index on `execution_id`
- index on `(route_id, id desc)`
- index on `(verdict, id desc)`
- index on `(execution_started_at desc, id desc)`

The table is append-only at the application layer.

## Hashing And Integrity

### Request Body Hash

`request_body_sha256` is computed from the raw bytes exactly as received.

This is intentional:

- it works for both valid and invalid JSON
- it preserves evidence of what actually hit the proxy
- it avoids ambiguity introduced by JSON normalization

### Configuration Hashes

Store configuration state as separate fields:

- `route_config_hash`
- `policy_set_hash`

These remain separate so operators can tell whether a route change or policy change altered behavior. Stage 2 stores them as distinct fields.

### Violation Value Handling

Do not store the exact matched violation value in Stage 2.

Instead store:

- `violation_value_hash`
- `violation_value_preview` when safe

Preview rules:

- URLs: keep scheme + host, redact path and query
- emails: mask the local part
- IDs or tokens: show only a short prefix and suffix
- clearly sensitive values: store no preview at all

### Audit Chain

The ledger uses a global chained hash:

- `previous_hash` references the immediately prior persisted audit row by `id`
- `record_hash` is SHA-256 over the canonical serialized row content plus `previous_hash`

This yields one append-only tamper-evident chain for the whole service.

### Canonical Ordering

Integrity verification walks rows by database insert order (`id`), not by execution timestamp.

Timestamps are useful for incident analysis, but they do not define ledger order.

## Audit API

Stage 2 adds an operator-facing read-only API under `/v1/audit`.

### Authentication

Audit endpoints require an admin bearer token from configuration or environment, for example:

- `GUARDRAIL_ADMIN_TOKEN`

This auth mechanism is intentionally narrow and temporary. It protects sensitive audit data now without pulling Stage 3 tenant auth into Stage 2.

### List Executions

`GET /v1/audit/executions`

#### Filters

- `route_id`
- `verdict`
- `from`
- `to`
- `limit`
- `cursor`
- optional `order=asc`

#### Behavior

- default ordering is newest first
- pagination is cursor-based, not offset-based
- response returns `items`, `next_cursor`, and applied filters

### Execution Detail

`GET /v1/audit/executions/{execution_id}`

Returns persisted fields only. It does not compute per-record integrity status inline. Integrity checks remain a separate concern.

### Integrity Verification

`GET /v1/audit/integrity`

#### Inputs

- `from`
- `to`
- `limit`
- optional `full=true`

#### Behavior

- bounded range verification is the default
- whole-chain verification requires explicit opt-in with `full=true`
- verification walks the chain in ledger order

#### Response

- `valid`
- `rows_scanned`
- `range_start`
- `range_end`
- `first_broken_execution_id` if invalid

## Migration Strategy

Migrations do **not** run automatically on normal service startup.

### Operator Flow

1. Apply migrations explicitly
2. Start or restart Guard Rail
3. Guard Rail checks schema compatibility on boot
4. If required migrations are missing, startup fails clearly

This avoids coupling app boot to unexpected schema mutation in production.

## Configuration

Stage 2 extends config with:

```yaml
database:
  url: "postgres://guardrail:secret@localhost:5432/guardrail"
  max_connections: 10

audit:
  write_timeout_ms: 250

admin:
  token: "change-me"
```

Environment variables override these values, following the existing double-underscore pattern.

Examples:

- `GUARDRAIL_DATABASE__URL`
- `GUARDRAIL_DATABASE__MAX_CONNECTIONS`
- `GUARDRAIL_AUDIT__WRITE_TIMEOUT_MS`
- `GUARDRAIL_ADMIN__TOKEN`

## Failure Behavior

### Audit Insert Failure

If PostgreSQL insert or integrity-chain preparation fails:

- return the normal proxy response
- emit a structured error log with `execution_id`
- count the failure in metrics once observability is added later

This is Stage 2 fail-open behavior.

### Audit API Authentication Failure

- missing token → `401 Unauthorized`
- invalid token → `401 Unauthorized`

### Integrity Verification Failure

If the chain is broken, the API returns a successful response indicating invalid integrity, not a transport error.

## Security Posture

Stage 2 avoids turning PostgreSQL into a secret archive.

Persist:

- metadata
- hashes
- limited safe previews
- human-readable policy identifiers

Do not persist:

- raw request body
- raw response body
- arbitrary request headers
- authorization token values
- full regex patterns
- full allowlists or blocklists
- full serialized rule definitions

## Testing Strategy

### Unit Tests

- hash canonicalization and SHA-256 output
- safe preview generation for violation values
- admin auth middleware behavior
- cursor encoding and decoding

### Integration Tests

- successful audit row insertion for:
  - invalid JSON on known route
  - blocked request
  - allowed forwarded request
  - allowed request with upstream failure
- no audit row for unknown route
- audit list API returns newest-first paginated results
- audit detail returns persisted row fields
- integrity endpoint verifies a valid chain
- integrity endpoint reports first broken row when the chain is corrupted
- audit endpoints reject missing or invalid admin token

### Database Test Model

Use a real PostgreSQL test database for Stage 2 integration tests. SQLite is not an acceptable stand-in because the audit ledger and migration path are PostgreSQL-specific.

## Project Structure

```
guard-rail-engine/
  migrations/
    0001_create_execution_audit.sql
  src/
    execution/
      mod.rs            — canonical execution record
    storage/
      mod.rs            — storage traits and shared types
      postgres.rs       — PostgreSQL pool, inserts, reads, integrity queries
    audit/
      mod.rs            — audit module exports
      hash.rs           — request/config/ledger hash helpers
      api.rs            — audit list, detail, integrity endpoints
    auth/
      mod.rs            — auth module exports
      middleware.rs     — admin bearer token enforcement for audit routes
```

Existing files updated:

- `src/config.rs`
- `src/lib.rs`
- `src/main.rs`
- `src/logging.rs`
- `src/proxy/mod.rs`

## Result Of Stage 2

After this stage, Guard Rail remains the same gateway externally for execution traffic, but now:

- every real configured-route execution is durably queryable
- the audit ledger is append-only and tamper-evident
- operators can verify ledger integrity without direct database access
- execution, storage, and audit boundaries are established clearly enough for the next stage to extend them without redesigning the proxy path
