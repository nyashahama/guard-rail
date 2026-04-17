# Guard Rail Backend Stage 3 Design

**Date:** 2026-04-17

**Status:** Approved design for implementation planning

## Goal

Add tenant isolation to the Stage 2 Guard Rail backend so every execution is attributable to a tenant and API key, tenant requests are constrained to tenant-owned routes, tenant callers can read only their own audit history, and operators can manage tenant security state without introducing a full control plane.

## Scope

Stage 3 includes:
- tenants with lifecycle state
- hashed API keys with operator metadata
- tenant-to-route ownership bindings
- tenant authentication on execution requests
- tenant context attached to execution and audit records
- tenant-scoped audit reads on the existing audit endpoints
- failed-auth auditing
- basic in-memory per-tenant rate limiting
- admin APIs for tenant, key, and route-binding management

Stage 3 excludes:
- policy CRUD APIs
- route CRUD APIs
- tenant self-service management
- key scopes
- key expiry
- distributed cache invalidation
- distributed rate limiting
- hard tenant deletion
- separate operator accounts or RBAC

## Architecture

Stage 3 keeps the existing single-binary `axum` service and extends the Stage 2 backend with a narrow tenant security layer. The execution contract remains `/v1/execute/{route_id}`. The new behavior is inserted before the existing policy evaluation and upstream forwarding flow.

The architecture keeps two distinct sources of truth:
- YAML remains the source of route and policy behavior.
- PostgreSQL becomes the source of tenant security state.

Tenant security state in PostgreSQL includes:
- tenant records
- API key records and lifecycle metadata
- route ownership bindings

Runtime state is loaded into memory at startup:
- route definitions from YAML
- policy definitions from YAML
- tenant-route bindings from PostgreSQL
- active API key metadata from PostgreSQL

The request path remains optimized around in-memory lookup rather than per-request database reads. Stage 3 therefore follows the existing backend shape instead of introducing a DB-dependent hot path.

## Request Flow

For tenant execution requests, enforcement happens before the existing proxy logic:

1. Read the presented API key from the request.
2. Resolve the key against the active in-memory key cache.
3. Reject missing, invalid, revoked, or disabled-tenant keys.
4. Resolve the requested `route_id`.
5. Verify the route exists in loaded YAML.
6. Verify the route is bound to the same tenant as the authenticated key.
7. If the tenant does not own the route, return `404`.
8. Apply per-tenant rate limiting.
9. Attach tenant context and `api_key_id` to request state.
10. Continue through the existing execution, audit, and forwarding pipeline.

This keeps Stage 3 additive: the existing policy engine and upstream forwarding behavior remain intact once tenant authorization succeeds.

## Security Rules

- Tenant identity is derived from the API key only.
- Each route belongs to exactly one tenant.
- Every executable route must have an explicit tenant binding.
- Execution routes fail closed if unbound.
- Disabled tenants lose all tenant-key access immediately.
- Revoked keys stop working immediately for new requests.
- Wrong-tenant route access returns `404`, not `403`.
- Tenant callers can only read audit rows for their own tenant.
- Admin callers retain global tenant and audit visibility.

This is intentionally a narrow boundary-hardening stage. The purpose is isolation and attribution, not full configuration management.

## Admin Surface

Stage 3 keeps the existing global admin bearer token as the control-plane gate. It does not introduce a separate operator authentication model.

Admin APIs should support:
- create tenant
- list tenants
- disable tenant
- create API key for tenant
- list keys for tenant
- revoke key
- update key metadata such as `name`
- create or update tenant-route binding

Admin writes update PostgreSQL first and then trigger an immediate in-process refresh of the tenant auth and binding cache. If the refresh fails, the API must treat the mutation as failed rather than reporting the change as live.

Bindings may only be created for route IDs that already exist in the loaded YAML config. Stage 3 does not support pending bindings for future routes.

Tenants are soft-disabled only. Hard deletion is out of scope because it would undermine audit attribution and lifecycle traceability.

## Audit Surface

Stage 3 keeps the existing `/v1/audit/...` endpoints and makes them role-aware instead of splitting them into separate namespaces.

Admin callers:
- can query across tenants
- can apply tenant-aware filters
- can inspect auth-failure events across the system

Tenant-key callers:
- can list only audit rows whose `tenant_id` matches theirs
- can fetch detail rows only when the row belongs to their tenant
- receive `404` for rows belonging to other tenants
- can see auth-failure events only when those events are attributable to their tenant

Events with no resolved tenant context, such as missing-key or unknown-key scans, remain admin-only.

## Data Model

### `tenants`

Fields:
- `id`
- `name`
- `status`
- `created_at`
- `disabled_at`

Status begins as a narrow lifecycle model:
- `active`
- `disabled`

### `api_keys`

Fields:
- `id`
- `tenant_id`
- `key_prefix`
- `key_hash`
- `name`
- `created_at`
- `last_used_at`
- `revoked_at`
- `revoked_reason`

Rules:
- raw API keys are shown only at creation time
- only hashes are stored at rest
- multiple active keys per tenant are allowed
- keys remain valid until revoked
- `last_used_at` updates on successful authentication only

### `tenant_routes`

Fields:
- `route_id`
- `tenant_id`
- `created_at`
- `updated_at`

This table is the source of tenant authorization for executable routes. YAML remains the source of route behavior, but not route ownership.

### Execution and Audit Records

Stage 3 extends the Stage 2 execution and audit model with:
- `tenant_id`
- nullable `api_key_id`
- auth outcome classification for failed-auth events

This allows attribution at both tenant and key level without storing redundant raw credential material in the ledger.

## Failure Policy

Hard failures:
- missing API key
- invalid API key
- revoked API key
- disabled tenant
- wrong-tenant route access
- rate limit exceeded
- tenant-binding cache refresh failure after admin mutation

Behavior:
- authentication and route-ownership checks fail closed
- wrong-tenant route access returns `404`
- per-tenant rate limiting returns `429`
- disabled tenants lose both execution and tenant-audit access

Best-effort behavior:
- `last_used_at` updates are asynchronous or otherwise non-blocking
- metadata update failures must not turn a valid execution into a failed one

This mirrors the Stage 2 decision to keep secondary metadata writes from becoming proxy-availability dependencies.

## Caching And Refresh

Stage 3 should not put PostgreSQL in the hot path for every execution request. Tenant bindings and active API key metadata should be loaded into an in-memory cache.

Cache behavior:
- load at startup
- refresh immediately after successful admin writes
- keep request-time authorization in memory

Stage 3 does not include:
- distributed cache invalidation
- background polling
- eventual-consistency control-plane semantics

The immediate-refresh model is chosen because the system is still single-instance oriented in this stage and predictable operator behavior matters more than distributed coordination.

## Rate Limiting

Stage 3 includes basic in-memory per-tenant rate limiting.

Properties:
- keyed by tenant ID
- enforced before execution proceeds
- returns `429` on limit breach
- isolated per tenant so one tenant cannot monopolize the process

This is explicitly documented as single-instance only. Cross-node enforcement is deferred to a later hardening stage.

## Testing Requirements

Required integration coverage:
- missing API key rejects the request and audits the event
- invalid API key rejects the request and audits the event
- revoked API key rejects the request and audits the event
- disabled tenant rejects execution requests and tenant-key audit reads
- valid key for wrong-tenant route returns `404`
- valid key for owned route preserves existing execution behavior
- tenant caller sees only owned audit list rows
- tenant caller can fetch detail for owned rows only
- admin caller can query across tenants
- auth-failure audit events are tenant-visible only when attributable
- route binding changes take effect after admin mutation-triggered refresh
- per-tenant rate limiting returns `429` without affecting other tenants
- startup fails or routes are unavailable when executable routes lack tenant bindings

## Operational Notes

- Stage 3 intentionally reuses the existing admin bearer token to avoid expanding into operator identity design.
- Stage 3 intentionally does not add policy or route authoring APIs.
- Stage 3 intentionally does not add key expiry, scopes, or RBAC.
- Stage 3 intentionally does not support hard tenant deletion.

These omissions are deliberate. The stage is successful when tenant isolation, attribution, and operator-managed key lifecycle are in place without diluting the current backend architecture.
