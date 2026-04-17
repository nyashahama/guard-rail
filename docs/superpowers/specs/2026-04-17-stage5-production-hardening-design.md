# Guard Rail Backend Stage 5 Design

**Date:** 2026-04-17

**Status:** Approved design for implementation planning

## Goal

Harden the Stage 4 Guard Rail backend for single-instance production operation by adding explicit readiness semantics, Prometheus metrics, OpenTelemetry-compatible trace context, graceful shutdown and drain behavior, and deployment artifacts that let operators run the service without guessing at startup, failure, or restart state.

## Scope

Stage 5 includes:
- a readiness endpoint separate from the existing liveness endpoint
- Prometheus metrics for request volume, verdicts, upstream failures, audit persistence failures, replay persistence failures, readiness state, and request latency
- structured tracing based on `tracing` spans with stable execution attributes
- machine-readable execution logs emitted through the tracing pipeline instead of raw `println!`
- graceful shutdown with drain behavior for in-flight requests
- Docker and systemd deployment artifacts
- smoke coverage for startup, readiness, metrics exposure, and draining behavior
- configuration for observability and shutdown behavior

Stage 5 excludes:
- distributed rate limiting
- multi-node readiness coordination
- Kubernetes manifests or Helm charts
- mandatory external trace exporters
- background worker orchestration
- RBAC or operator accounts
- retention, archival, or SLO automation

## Design Choice

Three approaches were considered:

1. Keep Stage 5 minimal and add only `/ready` plus a few counters.
   - Rejected because it leaves too much operator behavior implicit. Restart behavior, in-flight drain state, log correlation, and deployment shape would still be undocumented and only partially observable.

2. Redesign the runtime around a new service container abstraction before adding observability.
   - Rejected because Stage 5 is a hardening stage, not a rewrite stage. The backend already has a clear single-process startup path and request pipeline, so a large runtime refactor would create risk without closing the immediate operational gap.

3. Add an operational layer around the existing runtime with explicit lifecycle state, additive observability modules, and deployment artifacts.
   - Recommended because it preserves the current request flow, keeps the hot path simple, and makes production behavior visible without changing tenant, audit, or replay semantics.

Stage 5 uses option 3.

## Architecture

Stage 5 keeps the existing single `axum` binary and introduces two new runtime concerns:

- `observability`
  - owns metrics registration, metrics rendering, tracing subscriber setup, and request-level instrumentation helpers
- `shutdown`
  - owns shared lifecycle state, in-flight request tracking, signal handling, and drain coordination

The existing modules keep their functional ownership:

- `proxy` still owns execution, policy evaluation, forwarding, and response generation
- `storage::postgres` still owns database interactions
- `logging` still defines the canonical execution-log shape, but emits through `tracing`
- `main` still performs process startup, but now wires lifecycle state, observability, readiness checks, and graceful shutdown

This keeps Stage 5 additive. The request contract remains `/v1/execute/{route_id}` and the existing audit, admin, and replay APIs stay in place.

## Runtime Lifecycle Model

The service should expose a small shared lifecycle state with four operator-visible states:

- `starting`
  - process boot has begun but readiness has not been achieved
- `ready`
  - routes and policies are loaded, tenant cache is loaded, database checks pass, and the server is accepting work
- `draining`
  - shutdown has started, readiness is false, and the server is waiting for in-flight requests to finish within the configured grace period
- `stopped`
  - process shutdown is complete

This lifecycle state should be stored in a shared runtime object rather than inferred ad hoc from multiple modules.

## Health And Readiness Semantics

`GET /health`
- remains a shallow liveness probe
- returns `200 ok` whenever the process is up enough to answer HTTP
- does not check the database or route state

`GET /ready`
- is the deployment and load-balancer gate
- returns `200` only when:
  - startup finished successfully
  - route and policy state is loaded
  - tenant auth cache is loaded
  - PostgreSQL responds to a lightweight readiness query
  - the service is not draining
- returns `503` when any of those checks fail

Stage 5 should fail closed on readiness. If the database becomes unavailable after startup, `/ready` should switch to `503` even though `/health` may remain `200`.

## Metrics Model

Stage 5 should expose a Prometheus text endpoint, defaulting to `GET /metrics`.

Required metrics:

- `guardrail_requests_total`
  - counter
  - labels: `route_id`, `method`, `verdict`
- `guardrail_request_latency_seconds`
  - histogram
  - labels: `route_id`, `method`, `verdict`
- `guardrail_upstream_failures_total`
  - counter
  - labels: `route_id`
- `guardrail_audit_persist_failures_total`
  - counter
  - labels: `operation`
- `guardrail_replay_persist_failures_total`
  - counter
  - labels: `operation`
- `guardrail_inflight_requests`
  - gauge
- `guardrail_readiness`
  - gauge with values `1` for ready and `0` for not ready
- `guardrail_shutdown_transitions_total`
  - counter
  - labels: `state`

Metrics failures must never fail client traffic. If metrics rendering or metric registration fails, the service should log the error and keep serving requests.

## Tracing And Logging

Stage 5 should keep `tracing` as the structured event pipeline and make two changes:

1. Create a request span for every execution request with:
   - `execution_id`
   - `route_id`
   - `tenant_id` when present
   - `api_key_id` when present
   - request method
   - final verdict

2. Change execution-log emission from `println!` to `tracing::info!` with the existing machine-readable JSON fields preserved.

Trace behavior should be OpenTelemetry-compatible, not exporter-dependent:

- accept an incoming `traceparent` header when present
- generate a local request correlation ID when absent
- enrich spans with execution and tenant fields
- leave exporter wiring optional through config so Stage 5 does not require an external collector

This gives operators consistent correlation across logs, traces, audit rows, and replay runs without making the backend dependent on an observability platform.

## Request Instrumentation Points

Instrumentation should stay close to the existing request flow in `proxy::handle_execute`:

- increment in-flight gauge at request start and decrement on completion
- record total request counters and latency for:
  - blocked responses
  - rejected responses
  - allowed responses
  - upstream failure responses
- increment audit persistence failure metrics when asynchronous audit or replay writes fail
- record readiness state changes and shutdown transitions from lifecycle code, not from the proxy

The Stage 2 and Stage 4 best-effort persistence policy remains unchanged:

- original traffic still returns the client response even if audit or artifact persistence fails
- metrics and tracing must make that failure visible

## Graceful Shutdown

Stage 5 should support `SIGINT` and `SIGTERM`.

Shutdown sequence:

1. Flip lifecycle state from `ready` to `draining`.
2. Set `/ready` to return `503` immediately.
3. Stop accepting new connections through `axum::serve(...).with_graceful_shutdown(...)`.
4. Wait for in-flight requests to reach zero or for the grace-period deadline.
5. Emit final lifecycle and drain summary events.
6. Exit cleanly.

If the grace period expires first, the service should log how many requests were still in flight and then exit. Stage 5 should not attempt to checkpoint or replay interrupted upstream calls during shutdown.

## Deployment Artifacts

Stage 5 should add two deployment shapes:

### Docker

- multi-stage build
- compile the Rust binary in a builder image
- copy only the binary, config directory, and required runtime files into a slim runtime image
- expose the service port
- default command runs `guard-rail-engine serve --config ./config/config.yaml`

### systemd

Provide a `guard-rail-engine.service` unit that:

- runs after `network.target`
- reads environment overrides from an optional environment file
- restarts on failure with a short delay
- sends `SIGTERM` for normal stop behavior
- sets a stop timeout longer than the configured application grace period

Stage 5 does not add container orchestration manifests. Docker and systemd are enough for the current single-instance target.

## Configuration Surface

Stage 5 should add two config sections:

### `observability`

Fields:
- `service_name`
- `metrics_enabled`
- `metrics_path`
- `trace_header_name`
- `readiness_probe_timeout_ms`
- optional exporter endpoint for future OTel use

### `shutdown`

Fields:
- `grace_period_ms`
- `drain_poll_interval_ms`

These settings should support environment overrides using the existing `GUARDRAIL_*` pattern.

## Failure Policy

Hard failures at startup:
- database connection failure
- schema not ready
- route or policy load failure
- tenant route binding validation failure
- observability subscriber initialization failure

Best-effort runtime behavior:
- metrics collection failure does not fail client traffic
- audit persistence failure does not fail client traffic
- replay artifact persistence failure does not fail client traffic
- trace export failure does not fail client traffic

Readiness policy:
- readiness returns `503` when database checks fail or the service is draining
- liveness remains `200` unless the process is unhealthy enough to stop serving HTTP

## Testing Requirements

Required coverage:

- config loading for the new `observability` and `shutdown` sections
- `/health` remains `200` while `/ready` can return `503`
- `/ready` returns `200` only after startup state is marked ready
- `/ready` returns `503` during drain mode
- `/metrics` exposes request, latency, inflight, and readiness metrics
- execution requests increment counters and histograms with the expected labels
- audit persistence failures increment failure counters without changing client responses
- a long-running request is allowed to finish during graceful shutdown when it completes within the grace window

## Out-Of-Scope Clarifications

Stage 5 is intentionally single-instance. The following remain deferred:

- distributed rate limiting
- cross-node tenant auth cache coordination
- shared leader election or work scheduling
- managed OpenTelemetry collector deployment
- autoscaling and multi-zone failover

Those concerns should be addressed only after the single-instance operational model is proven in practice.
