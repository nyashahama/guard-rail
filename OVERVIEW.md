# Guard Rail Overview

## Repository Split

This repository contains two distinct surfaces:

- `guard-rail-engine/` is the runtime. It is the container-first Rust service that enforces route and policy rules, persists audit and replay data, and exposes the operational endpoints used in the pilot deployment.
- `app/` is the marketing site. It presents the product story, but it is not part of the control plane and does not participate in request enforcement.

That split matters operationally: the landing page can change independently of the runtime, while the runtime remains the source of truth for request handling, policy enforcement, and execution records.

## Current Product Shape

Guard Rail is a policy enforcement runtime for internal API traffic. The current implementation is centered on a pilot deployment that evaluates a request before forwarding it to an upstream API.

The runtime currently exposes these surfaces:
- route execution for `POST /v1/execute/{route_id}`-style traffic
- route and method validation before forwarding
- authentication and authorization checks around runtime access
- audit logging and audit persistence for every execution
- replay capture and replay persistence for later investigation
- readiness and metrics endpoints for health and operations
- admin and maintenance commands for the pilot deployment path

The important boundary is that Guard Rail governs the request path, not the upstream application itself.

## Deployment Model

The blessed deployment path is container-first:

`container image + external Postgres + reverse proxy/LB`

That model reflects the current runtime shape:
- the image is the deployable unit
- Postgres lives outside the container and stores durable state
- a reverse proxy or load balancer fronts the runtime
- config is mounted in from disk
- secrets are injected through the environment

This is the deployment path the docs and verification package are aligned to. Other packaging choices may exist in the repo, but they are not the recommended pilot path.

## Request Flow

At a high level, the runtime does the following on each request:

1. Resolve the target route.
2. Check the request method and basic request shape.
3. Authenticate the caller and apply route-level access controls.
4. Evaluate configured policies against the request payload and metadata.
5. Record the execution in audit and replay surfaces.
6. Forward the request upstream only if the checks pass.

The exact policies are data-driven, so most behavior is controlled through repo-managed configuration rather than code changes.

## Runtime Surfaces

### Routing

Routes define which upstream a client can reach, which HTTP methods are allowed, and which policies apply. The runtime resolves the route first so it can reject invalid or unauthorized traffic before any upstream call is made.

### Authentication and Authorization

The runtime has explicit request-access surfaces rather than assuming open access. That keeps the request path tied to runtime-issued or runtime-validated credentials instead of treating the engine as a public passthrough.

### Audit

Every execution is recorded so operators can reconstruct what happened, why a request was allowed or blocked, and which route or policy was involved.

### Replay

Replay artifacts capture the request context needed for later analysis and verification. In the current pilot posture, replay is an operational record, not a productized end-user workflow.

### Runtime Health

Readiness and metrics are part of the current operational contract. They exist so the runtime can be deployed behind a proxy/LB and observed like a production service, even though the product itself is still in pilot posture.

## Data and Operations

The operator-facing docs are split by concern:

- `guard-rail-engine/deploy/container/DATA_OPERATIONS.md` covers cleanup, backup, restore, and rollback.
- `guard-rail-engine/deploy/observability/` covers metrics, alerts, dashboards, and runbooks.
- `guard-rail-engine/deploy/verification/README.md` covers the pilot verification suite and its expected scenarios.

Those documents are the authoritative operational references for the current runtime posture.

## Verification-Backed Position

This repo should be described as a verified pilot runtime, not as a roadmap-era platform promise.

That means the docs should stay aligned with what is actually covered by the current deployment and verification package:
- containerized runtime deployment
- external Postgres
- request enforcement before forwarding
- audit and replay persistence
- readiness, metrics, and operational runbooks

It does not mean claiming broader platform features, multi-tenant control-plane behavior, or unspecified enterprise guarantees that are not backed by the current code and verification assets.

## Non-Goals In This Overview

This file is intentionally not a full API reference or implementation spec. It does not enumerate every config key or every endpoint, and it should not promise features that belong in future work.

For the concrete operator procedures and verification steps, use the deployment docs linked above.
