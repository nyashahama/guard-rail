# Guard Rail Backend Stage 4 Design

**Date:** 2026-04-17

**Status:** Approved design for implementation planning

## Goal

Add deterministic replay to the Stage 3 Guard Rail backend so operators can re-evaluate a past execution against the original stored policy snapshot or the current in-memory policy set, compare verdicts, and inspect the exact request and response artifacts that explain the result.

## Scope

Stage 4 includes:
- replayable artifact capture for executions that reach policy evaluation
- persistent policy snapshots derived from the active in-memory route and referenced policies
- replay runs stored in PostgreSQL for auditability
- replay APIs under `/v1/replay/...`
- tenant-safe replay access using the existing Stage 3 auth model
- audit detail responses enriched with replay availability metadata

Stage 4 excludes:
- live upstream re-forwarding
- binary blob storage outside PostgreSQL
- retention and archival policies
- UI or dashboard work
- policy authoring or version management APIs
- distributed replay workers or background job orchestration

## Design Choice

Three approaches were considered:

1. Reuse `execution_audit` only and derive replay input from summary columns.
   - Rejected because Stage 2 and Stage 3 audit rows intentionally omit the full request payload, request headers, response body, and the policy snapshot contents required for deterministic replay.

2. Store artifacts in an external blob system and keep only references in PostgreSQL.
   - Rejected for Stage 4 because it adds operational surface area without solving the core product gap. The current backend is still a single-process service, and replay should be introduced with the smallest possible moving set.

3. Add relational artifact storage plus deduplicated policy snapshots in PostgreSQL.
   - Recommended because it fits the current architecture, keeps replay local to the existing `sqlx` storage layer, and gives deterministic re-evaluation without introducing another service.

Stage 4 uses option 3.

## Architecture

Stage 4 keeps the existing single `axum` binary and extends the proxy pipeline with a replay capture layer. The Stage 2 and Stage 3 `ExecutionRecord` remains the canonical summary ledger. Stage 4 adds a second persistence lane for replay-specific data.

The design introduces three new persisted concepts:

- `policy_snapshots`
  - one row per unique route-plus-policy snapshot hash
  - stores a normalized representation of the route definition and only the policy definitions referenced by that route
- `execution_artifacts`
  - one row per replayable execution
  - stores request payload, selected request headers, optional upstream response artifacts, and the policy snapshot hash used by the original execution
- `replay_runs`
  - one row per replay request
  - stores replay mode, result summary, and verdict-diff metadata

This keeps the existing audit history stable while making replay additive.

## Snapshot Strategy

The current server computes `route_config_hash` and `policy_set_hash` once at startup, but the reload watcher can replace routes and policies later. That makes the startup hashes insufficient for deterministic Stage 4 replay.

Stage 4 fixes that by deriving a per-execution snapshot from the current in-memory state at the point of handling the request:

1. Load the current `Route` from `RouteTable`.
2. Load only the referenced `Policy` objects from `PolicySet`.
3. Normalize both structures into a stable JSON representation.
4. Hash that normalized structure to produce `snapshot_hash`.
5. Upsert the snapshot into `policy_snapshots`.
6. Persist `snapshot_hash` with the replay artifacts and surface it through joined audit and replay reads.

Determinism comes from replaying against the persisted normalized snapshot rather than trying to reconstruct file contents after the fact.

## Artifact Capture Rules

Replay artifacts are captured only for executions that produce a policy-relevant input:

- capture for `BLOCKED` executions
- capture for `ALLOWED` executions
- do not capture replay artifacts for:
  - unknown-route `404`
  - missing/invalid API key failures
  - rate-limit rejections
  - method rejections
  - invalid JSON rejections

Those non-replayable outcomes remain visible in `execution_audit`, but they do not create `execution_artifacts` rows.

## Stored Artifact Shape

### Request artifacts

Store:
- parsed JSON payload
- original request size in bytes
- selected request headers
- route ID
- method
- tenant and key attribution copied from the execution summary

Do not store:
- raw `authorization` header values
- arbitrary full header dumps
- client IP beyond what Stage 2 and Stage 3 already keep in `execution_audit`

Header capture uses an allowlist from config. The default list should be narrow:
- `content-type`
- `accept`
- `x-request-id`

### Response artifacts

For allowed upstream calls, store:
- upstream status
- selected response headers
- response body text when valid UTF-8
- response body SHA-256
- truncation flag when the response exceeds the configured replay capture limit

For blocked executions, response artifacts are absent because no upstream call occurred.

### Policy snapshot contents

Each snapshot stores:
- `snapshot_hash`
- normalized route definition for the executed route
- normalized policy definitions referenced by the route
- derived `route_config_hash`
- derived `policy_set_hash`
- creation timestamp

This is enough to support both exact historical replay and comparison to the current policy set.

## Replay Modes

`POST /v1/replay/executions/{execution_id}` accepts a replay mode:

- `snapshot`
  - default
  - use the stored policy snapshot captured at original execution time
  - no upstream forwarding
- `current`
  - use the currently loaded in-memory route and policy definitions
  - no upstream forwarding

Stage 4 does not support live re-forwarding. If that feature is needed later, it should be a separate explicit mode with stricter authorization and different audit treatment.

## Replay Flow

When a replay is requested:

1. Load the original audit row.
2. Apply the same tenant/admin access rules as audit detail.
3. Load the corresponding `execution_artifacts` row.
4. Resolve the replay policy source:
   - stored snapshot
   - current in-memory route and policies
5. Reconstruct the policy evaluation input using:
   - stored request JSON
   - stored request size in bytes
   - stored route policy list
6. Run the existing policy engine offline.
7. Build a replay result that compares:
   - original verdict
   - replay verdict
   - original blocking policy/rule
   - replay blocking policy/rule
8. Persist a `replay_runs` row.
9. Return the replay result without contacting the upstream service.

## API Surface

### `POST /v1/replay/executions/{execution_id}`

Request body:
- `policy_source`: `"snapshot"` or `"current"`; default `"snapshot"`

Response shape:
- original execution summary
- replay result summary
- `verdict_changed`
- `policy_changed`
- `rule_changed`
- `original_snapshot_hash`
- `evaluated_snapshot_hash`
- `replay_run_id`

### `GET /v1/replay/executions/{execution_id}`

Response shape:
- original execution summary
- artifact availability metadata
- latest replay runs for that execution, newest first

This endpoint is read-only. It does not trigger a new replay.

### Audit detail enrichment

`GET /v1/audit/executions/{execution_id}` should add:
- `replay_available`
- `snapshot_hash`
- `latest_replay_run_id` when one exists

This keeps replay discoverable from the existing audit workflow.

## Access Control

Replay uses the Stage 3 access model:

- admin callers can replay any execution
- tenant callers can replay only executions whose `tenant_id` matches theirs
- cross-tenant replay requests return `404`

Replay reads should reuse the same audit-access middleware where practical instead of introducing a second authorization model.

## Failure Policy

Hard failures:
- replay requested for unknown execution
- replay requested for an execution without artifacts
- stored snapshot missing for `snapshot` mode
- cross-tenant replay attempt

Best-effort behavior:
- replay-run persistence failure should fail the request because a replay without a stored result weakens auditability
- artifact capture failure during original proxy execution should not fail the client response
  - the Stage 2 principle still applies: proxy availability is primary
  - missing artifacts simply make that execution non-replayable later

This creates a clear split:
- original traffic remains available even if replay storage is degraded
- replay operations require durable writes

## Data Model

### `policy_snapshots`

Fields:
- `snapshot_hash`
- `route_id`
- `route_definition`
- `policies_definition`
- `route_config_hash`
- `policy_set_hash`
- `created_at`

Rules:
- `snapshot_hash` is the primary identity
- multiple executions may reference the same snapshot

### `execution_artifacts`

Fields:
- `execution_id`
- `snapshot_hash`
- `request_body_json`
- `request_headers`
- `response_status`
- `response_headers`
- `response_body`
- `response_body_sha256`
- `response_body_truncated`
- `created_at`

Rules:
- one artifact row per replayable execution
- blocked executions may have null response fields

### `replay_runs`

Fields:
- `id`
- `execution_id`
- `policy_source`
- `evaluated_snapshot_hash`
- `original_verdict`
- `replay_verdict`
- `original_policy_name`
- `replay_policy_name`
- `original_rule_field`
- `replay_rule_field`
- `verdict_changed`
- `created_at`

Rules:
- multiple replay runs per execution are allowed
- the latest run is what audit detail should surface

## Testing Requirements

Required integration coverage:
- blocked execution stores replayable request artifacts and no response artifact
- allowed execution stores request artifacts and upstream response artifacts
- captured request headers do not include the raw tenant `authorization` header
- stored snapshot replay of an unchanged policy returns the original verdict
- current-policy replay after a policy change can return a different verdict
- replay never forwards upstream in either Stage 4 mode
- tenant caller cannot replay another tenant’s execution
- audit detail exposes replay metadata when artifacts exist
- snapshot hashing is stable across equivalent in-memory structures

## Operational Notes

- Stage 4 intentionally keeps replay synchronous and on-demand.
- Stage 4 intentionally uses PostgreSQL as the only persistence system.
- Stage 4 intentionally narrows capture to JSON request bodies and text-friendly responses because the current backend is API-focused and already parses JSON before policy evaluation.
- Stage 4 intentionally corrects the startup-hash limitation by snapshotting from in-memory route and policy state at execution time.

These constraints keep the stage focused on explainable deterministic replay rather than turning it into a broader storage or control-plane project.
