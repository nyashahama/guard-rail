# API Reference (Pilot)

This document covers the HTTP surfaces implemented today. It is intentionally scoped to the pilot runtime posture.

## Authentication model

- **Execution routes** are tenant-aware depending on `auth_mode` in route config.
  - `public`: no tenant API key required.
  - `tenant_bound`: requires `Authorization: Bearer <tenant_key>`.
- **Admin routes** require admin token via `Authorization: Bearer <admin_token>`.
- **Audit and replay routes**
  - `GET /v1/audit/executions`, `GET /v1/audit/executions/{execution_id}`, `GET /v1/replay/executions/{execution_id}`, `POST /v1/replay/executions/{execution_id}` require tenant auth (`Authorization: Bearer <tenant_key>`).
  - `GET /v1/audit/integrity` requires admin auth and is served on the admin listener.
- All execution records can include replay bundle metadata; no dedicated token exchange exists in this pilot.

## Common response envelope

Blocked execution response (`403`) has body:

```json
{
  "status": "blocked",
  "execution_id": "GR-EXE-...",
  "policy": "callback-allowlist",
  "rule_field": "$.callback",
  "message": "domain_not_in condition triggered on field $.callback"
}
```

Rejected response (`400`, `401`, `405`, etc.) uses:

```json
{
  "status": "rejected",
  "execution_id": "GR-EXE-...",
  "message": "Invalid JSON body"
}
```

Error responses from runtime-to-upstream path (`502`) use:

```json
{
  "status": "error",
  "execution_id": "GR-EXE-...",
  "message": "..."
}
```

## Runtime endpoints

### `GET /health`

Returns plain text `ok` when listener is up.

### `GET /ready`

Returns:
- `200 OK` when both lifecycle and DB readiness pass.
- `503 Service Unavailable` when either readiness input fails.

### `GET /metrics`

Enabled when `observability.metrics_enabled: true` in config.
Exposed at `observability.metrics_path` (default `/metrics`).

### `ANY /v1/execute/{route_id}`

Main enforcement and forwarding endpoint.

Query/body behavior:
- JSON body is required; non-JSON body returns `400` with `Invalid JSON body`.
- Method must match the route `methods` list, otherwise `405 Method Not Allowed`.
- Tenant-bound route without valid bearer key returns `401`.

Return behavior:
- Policy block: `403` with block response.
- Allowed: upstream request is proxied and returned with upstream status and headers.
- Route not found: `404 Route not found`.

Example execution call:

    curl -i -X POST "http://localhost:8080/v1/execute/pilot-webhook" \
      -H "Authorization: Bearer <tenant_key>" \
      -H "Content-Type: application/json" \
      -d '{"callback":"https://api.safe.com/hook","amount":100}'

Expected: `200` with upstream JSON body.

## Admin endpoints

These are served by the admin listener (default host/port from `admin_server`).

### `POST /v1/admin/tenants`

Create a tenant. Body:

```json
{ "name": "acme" }
```

Response:

```json
{
  "id": "...",
  "name": "acme",
  "status": "active",
  "created_at": "2026-...",
  "disabled_at": null
}
```

### `GET /v1/admin/tenants`

List tenants.

### `POST /v1/admin/tenants/{tenant_id}/keys`

Create a tenant API key. Body:

```json
{ "name": "service-client" }
```

Response includes one-time raw key and is returned only here:

```json
{
  "id": "...",
  "tenant_id": "...",
  "name": "service-client",
  "key_prefix": "gs_",
  "raw_key": "gr_..."
}
```

### `GET /v1/admin/tenants/{tenant_id}/keys`

List active and revoked keys by tenant.

### `POST /v1/admin/tenants/{tenant_id}/keys/{key_id}/revoke`

Revoke an API key.

### `POST /v1/admin/tenants/{tenant_id}/routes`

Bind a tenant to a route:

```json
{ "route_id": "pilot-webhook" }
```

## Audit endpoints

These are tenant scoped by bearer key, or admin scoped when token is used.

### `GET /v1/audit/executions`

Supported query params:
- `tenant_id`
- `route_id`
- `verdict`
- `from`, `to` (RFC3339 timestamps)
- `limit` (max 1000, default 50)
- `cursor` (offset pagination)
- `order=asc|desc` (default desc)

Sample response:

```json
{
  "items": [
    {
      "execution_id": "GR-EXE-...",
      "route_id": "pilot-webhook",
      "tenant_id": "...",
      "verdict": "ALLOWED",
      "matched_policy_name": null,
      "latency_total_ms": 120
    }
  ],
  "total": 1,
  "next_cursor": null
}
```

### `GET /v1/audit/executions/{execution_id}`

Includes execution detail plus replay availability fields.

### `GET /v1/audit/integrity?from_execution_id={uuid}&to_execution_id={uuid}`

Verifies checksum chain for execution history. Admin token required.

Response:

```json
{ "chain_valid": true, "first_invalid_record": null, "checked_from": "...", "checked_to": "..." }
```

## Replay endpoints

### `GET /v1/replay/executions/{execution_id}`

Get replay summary for an execution when replay artifacts exist.

### `POST /v1/replay/executions/{execution_id}`

Re-run policy decision against stored snapshot or current policy set.

Request body:

```json
{ "policy_source": "snapshot" }
```

`policy_source` defaults to `snapshot` and supports:
- `snapshot` (policies at execution time)
- `current` (current policy config)

Typical response:

```json
{
  "execution_id": "GR-EXE-...",
  "policy_source": "snapshot",
  "evaluated_snapshot_hash": "...",
  "original_verdict": "BLOCKED",
  "replay_verdict": "BLOCKED",
  "original_policy_name": "callback-allowlist",
  "replay_policy_name": "callback-allowlist",
  "original_rule_field": "$.callback",
  "replay_rule_field": "$.callback",
  "verdict_changed": false
}
```

## Pilot-facing caveats

- This API is implemented for the pilot deployment. There is no enterprise admin console yet.
- Some production-hardening knobs are present in code paths but not all are documented as platform promises.
- All runtime behavior should remain aligned to this API shape and the onboarded route/policy model.
