# Guard Rail Phase 2 Pilot-Safe Runtime Design

**Date:** 2026-05-01

**Status:** Approved design for implementation planning

## Goal

Move Guard Rail from "release blockers fixed" to "pilot-safe runtime" by hardening audit durability, replay privacy, startup validation, and verification gates without expanding into a SaaS control plane.

## Context

Phase 1 made the repository suitable for controlled pilot preparation: dependency audits are portable, CI has security gates, hot reload matches startup route-auth validation, and `required_before_response` audit persistence can force a `503` when the final audit bundle cannot be persisted before the client response.

One important limitation remains. For allowed requests, the upstream service is called before Guard Rail knows the final upstream status and response artifacts. If the final audit bundle write fails after the upstream side effect has occurred, Phase 1 can fail the client response but cannot prove that every forwarded request has durable pre-forward evidence.

Phase 2 closes that pilot risk by adding a durable pre-forward execution intent before forwarding allowed traffic. It also reduces replay data risk, removes unnecessary manual thread-safety assertions, validates production config more aggressively, and makes the Phase 7 suite the runtime hardening acceptance gate.

## Scope

Phase 2 includes:
- Add a durable pre-forward execution intent/outbox for strict audit mode.
- Reject forwarding in strict audit mode if the intent cannot be stored.
- Finalize the intent after the existing execution audit and replay bundle are persisted.
- Record failed finalization for operator recovery when final audit persistence fails after upstream forwarding.
- Add replay redaction for sensitive request/response headers and JSON fields.
- Keep replay redaction as the Phase 2 privacy mechanism; defer encryption and KMS to a later enterprise security phase.
- Remove `unsafe impl Send` and `unsafe impl Sync` for `AppState`.
- Add compile-time coverage that `AppState` remains `Send + Sync`.
- Strengthen production startup validation.
- Add focused tests and Phase 7 verification coverage for the new runtime guarantees.
- Update config examples and verification docs.

Phase 2 excludes:
- A SaaS control plane, management UI, billing, or self-serve tenant onboarding.
- KMS-backed replay encryption, key rotation, or customer-managed keys.
- Formal SOC 2, DPA, SBOM signing, or release provenance work.
- Re-architecting the audit hash chain.
- Exactly-once upstream execution semantics. Guard Rail can fail closed before forwarding when intent persistence fails, but it cannot make an external upstream side effect transactional with its own database.

## Design Choice

Three approaches were considered.

1. Only make `required_before_response` stricter.
   - This is too narrow. It still leaves no durable evidence for an allowed request when the final write fails after upstream forwarding.

2. Add a durable pre-forward intent plus replay redaction and production validation.
   - This is the recommended approach. It makes the runtime pilot-safe without pretending to solve distributed transaction semantics.

3. Build an enterprise-grade audit and replay security layer with encryption, KMS, signing, and attestation.
   - This is larger than Phase 2 and needs a key-management and release-governance design first.

Phase 2 uses option 2.

## Audit Durability Model

Strict audit mode becomes a two-step durability model for allowed traffic:

1. Before forwarding to upstream, Guard Rail inserts an execution intent with:
   - `execution_id`
   - route and tenant identifiers when available
   - method, source IP, content type, user-agent, request size, and request body hash
   - route and policy config hashes
   - status `pending`
   - creation timestamp

2. After upstream handling, Guard Rail persists the existing execution audit bundle and marks the intent as:
   - `finalized` when the final audit bundle is durable
   - `finalization_failed` when the upstream request already happened but final audit persistence failed

If pre-forward intent insertion fails in `required_before_response` mode, Guard Rail returns the existing audit persistence `503` and does not forward the request.

Blocked requests and pre-forward rejections do not need a pre-forward intent because no upstream side effect can happen. They keep using the existing final audit persistence path, still honoring `required_before_response`.

The intent table is separate from `execution_audit` so the existing audit hash chain remains a final-record ledger. A pending or failed intent is operational evidence, not a finalized verdict.

## Replay Privacy Model

Phase 2 uses redaction-first replay protection.

The replay config gains:
- `redact_request_headers`
- `redact_response_headers`
- `redact_json_fields`
- `redaction_text`

Defaults should redact common sensitive names such as:
- `authorization`
- `cookie`
- `set-cookie`
- `x-api-key`
- `api_key`
- `access_token`
- `refresh_token`
- `token`
- `secret`
- `password`
- `ssn`
- `id_number`

Redaction is applied before replay artifacts are persisted. The persisted artifacts must never contain raw values for configured redacted headers or configured JSON field names.

Header capture allowlists still control which headers are stored at all. Redaction is a second safety layer for cases where a captured header or response header later becomes sensitive.

For JSON payloads, Phase 2 redacts object keys by case-insensitive field name at any nesting depth. It does not implement full JSONPath matching; that would add complexity not needed for pilots.

Non-JSON response bodies remain governed by capture size and hash behavior. Phase 2 should redact response headers but not attempt content-aware redaction of arbitrary text or binary bodies.

## Production Config Validation

Startup validation should reject production configs that are unsafe for pilots:
- empty or default admin token when the admin listener is enabled
- admin listener bound to `0.0.0.0` unless an explicit future override exists
- empty database URL
- `audit.persistence_mode` not set to `required_before_response`
- replay enabled without a non-empty redaction policy
- zero `audit.write_timeout_ms`
- zero server request body limit
- zero replay response body limit when replay is enabled

Development config remains permissive enough for local tests and examples.

## Thread-Safety Cleanup

`AppState` currently has manual unsafe `Send` and `Sync` implementations. Phase 2 removes them and lets the compiler prove the type is safe.

A focused compile-time test should assert:

```rust
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn app_state_is_send_sync_without_unsafe_impls() {
    assert_send_sync::<AppState>();
}
```

If this fails, implementation must fix the non-thread-safe field rather than restoring unsafe implementations.

## Verification Strategy

Phase 2 implementation is accepted only when targeted tests and the full Phase 7 suite pass.

Focused tests must cover:
- strict mode inserts an intent before forwarding
- strict mode does not forward when intent insertion fails
- strict mode marks an intent finalized after successful bundle persistence
- strict mode marks finalization failed when the upstream was called but final persistence fails
- best-effort mode preserves current behavior
- replay redacts configured headers
- replay redacts configured JSON field names at nested depths
- production config rejects unsafe pilot settings
- `AppState` is `Send + Sync` without unsafe implementations

Phase 7 verification should add or document checks for:
- hard audit behavior under database degradation
- replay redaction evidence
- production config rejection behavior
- existing load, reload, drain, upstream degradation, and dependency audit checks

## Error Handling

Pre-forward intent persistence failure in strict mode returns the existing deterministic audit persistence error:
- HTTP `503 Service Unavailable`
- `x-guardrail-execution-id`
- JSON body with `status: "error"`
- metrics increment for audit persistence failure

Final audit persistence failure after upstream forwarding:
- returns the same `503` to the client
- marks the execution intent `finalization_failed` when possible
- logs the execution ID and persistence operation
- records audit and replay persistence failure metrics

If marking finalization failed also fails, Guard Rail logs that secondary failure. It cannot undo the upstream request.

## File Impact

Expected files:
- `guard-rail-engine/migrations/0006_create_execution_intents.sql`
- `guard-rail-engine/src/config.rs`
- `guard-rail-engine/src/main.rs`
- `guard-rail-engine/src/proxy/mod.rs`
- `guard-rail-engine/src/replay/redaction.rs`
- `guard-rail-engine/src/replay/mod.rs`
- `guard-rail-engine/src/storage/postgres.rs`
- `guard-rail-engine/config/config.yaml`
- `guard-rail-engine/deploy/container/config.yaml`
- `guard-rail-engine/deploy/verification/README.md`
- `guard-rail-engine/scripts/verification/run-phase7-suite.sh`
- `guard-rail-engine/scripts/verification/hard-audit-mode.sh`
- `guard-rail-engine/scripts/verification/replay-redaction.sh`
- `guard-rail-engine/tests/replay_integration_test.rs`
- `guard-rail-engine/tests/smoke_test.rs`
- `guard-rail-engine/tests/audit_api_test.rs`
- `guard-rail-engine/tests/integration_test.rs`

Additional tests may live in existing modules when that keeps related fixtures together.

## Exit Criteria

Phase 2 is complete when:
- strict audit mode cannot forward allowed traffic until a durable execution intent exists
- strict audit mode records whether each forwarded intent was finalized or failed finalization
- replay artifacts redact configured sensitive headers and JSON fields before storage
- production startup validation rejects unsafe pilot configs
- `AppState` has no manual unsafe `Send` or `Sync` implementation
- targeted Rust tests pass
- DB-backed integration tests pass
- `cargo fmt -- --check` passes
- `cargo clippy -- -D warnings` passes
- the Phase 7 verification suite is updated and run successfully in the local pilot environment
- docs describe the exact audit guarantee without claiming transactional upstream side effects
