# Guard Rail API Reference

The runtime exposes a main listener for execution, tenant-scoped audit, replay, health, readiness, and metrics. The admin listener is separate and should bind to loopback or a private operator network.

All examples use:

```bash
export GUARDRAIL_URL=http://127.0.0.1:18080
export GUARDRAIL_ADMIN_URL=http://127.0.0.1:18081
export GUARDRAIL_ADMIN_TOKEN=quickstart-admin-token
export GUARDRAIL_TENANT_KEY=<tenant-key>
```

## Authentication

Tenant requests use:

```text
authorization: Bearer <tenant-api-key>
```

Admin requests use:

```text
authorization: Bearer <admin-token>
```

## Execute Route

```http
POST /v1/execute/{route_id}
```

Forwards the request to the configured upstream only when route lookup, method validation, tenant authorization, rate limiting, and policies pass.

```bash
curl -i -X POST "${GUARDRAIL_URL}/v1/execute/pilot-webhook" \
  -H "authorization: Bearer ${GUARDRAIL_TENANT_KEY}" \
  -H "content-type: application/json" \
  -d '{"callback":"https://hooks.safe.example/ok","value":"hello"}'
```

Important response header:

```text
x-guardrail-execution-id: <execution-id>
```

Common statuses:

| Status | Meaning |
| --- | --- |
| `200`-`599` | Upstream response status was returned after Guard Rail allowed forwarding. |
| `400` | Invalid payload, malformed request, or tenant-bound route has no binding. |
| `401` | Missing or invalid tenant API key. |
| `403` | Policy blocked the request. |
| `404` | Route is unknown or the authenticated tenant is not bound to the route. |
| `405` | Method is not allowed for the route. |
| `413` | Request body exceeded the configured runtime body limit. |
| `429` | Tenant rate limit was exceeded. |
| `502` | Upstream forwarding failed before a response was received. |
| `503` | Strict audit persistence failed or readiness dependency is unavailable. |

## List Audit Executions

```http
GET /v1/audit/executions
```

```bash
curl -sS "${GUARDRAIL_URL}/v1/audit/executions?route_id=pilot-webhook&limit=20" \
  -H "authorization: Bearer ${GUARDRAIL_TENANT_KEY}" \
  | python3 -m json.tool
```

Supported filters:

| Query | Description |
| --- | --- |
| `tenant_id` | Admin-only tenant filter. Tenant callers are scoped to their tenant. |
| `route_id` | Route id filter. |
| `verdict` | Verdict filter, such as `ALLOWED` or `BLOCKED`. |
| `from` | RFC3339 lower timestamp bound. |
| `to` | RFC3339 upper timestamp bound. |
| `limit` | Page size. |
| `cursor` | Pagination cursor. |
| `order` | Sort order supported by the runtime. |

## Get Audit Execution

```http
GET /v1/audit/executions/{execution_id}
```

```bash
curl -sS "${GUARDRAIL_URL}/v1/audit/executions/${EXECUTION_ID}" \
  -H "authorization: Bearer ${GUARDRAIL_TENANT_KEY}" \
  | python3 -m json.tool
```

Tenant callers receive `404` for executions owned by another tenant.

## Replay Execution

```http
POST /v1/replay/executions/{execution_id}
```

```bash
curl -sS -X POST "${GUARDRAIL_URL}/v1/replay/executions/${EXECUTION_ID}" \
  -H "authorization: Bearer ${GUARDRAIL_TENANT_KEY}" \
  -H "content-type: application/json" \
  -d '{"policy_source":"snapshot"}' \
  | python3 -m json.tool
```

Replay evaluates stored request artifacts against either the stored policy snapshot or the current loaded policy set. It does not call the upstream service.

## Replay Summary

```http
GET /v1/replay/executions/{execution_id}
```

```bash
curl -sS "${GUARDRAIL_URL}/v1/replay/executions/${EXECUTION_ID}" \
  -H "authorization: Bearer ${GUARDRAIL_TENANT_KEY}" \
  | python3 -m json.tool
```

## Health, Readiness, And Metrics

```bash
curl -i "${GUARDRAIL_URL}/health"
curl -i "${GUARDRAIL_URL}/ready"
curl -sS "${GUARDRAIL_URL}/metrics"
```

`/health` confirms the process is reachable. `/ready` confirms the runtime is ready to serve traffic. `/metrics` is available when `observability.metrics_enabled` is `true`.

## Admin: Create Tenant

```http
POST /v1/admin/tenants
```

```bash
curl -sS -X POST "${GUARDRAIL_ADMIN_URL}/v1/admin/tenants" \
  -H "authorization: Bearer ${GUARDRAIL_ADMIN_TOKEN}" \
  -H "content-type: application/json" \
  -d '{"name":"acme-pilot"}' \
  | python3 -m json.tool
```

## Admin: List Tenants

```http
GET /v1/admin/tenants
```

```bash
curl -sS "${GUARDRAIL_ADMIN_URL}/v1/admin/tenants" \
  -H "authorization: Bearer ${GUARDRAIL_ADMIN_TOKEN}" \
  | python3 -m json.tool
```

## Admin: Create Tenant API Key

```http
POST /v1/admin/tenants/{tenant_id}/keys
```

```bash
curl -sS -X POST "${GUARDRAIL_ADMIN_URL}/v1/admin/tenants/${TENANT_ID}/keys" \
  -H "authorization: Bearer ${GUARDRAIL_ADMIN_TOKEN}" \
  -H "content-type: application/json" \
  -d '{"name":"primary"}' \
  | python3 -m json.tool
```

The raw key is returned only at creation time.

## Admin: List Tenant API Keys

```http
GET /v1/admin/tenants/{tenant_id}/keys
```

```bash
curl -sS "${GUARDRAIL_ADMIN_URL}/v1/admin/tenants/${TENANT_ID}/keys" \
  -H "authorization: Bearer ${GUARDRAIL_ADMIN_TOKEN}" \
  | python3 -m json.tool
```

## Admin: Revoke Tenant API Key

```http
POST /v1/admin/tenants/{tenant_id}/keys/{key_id}/revoke
```

```bash
curl -sS -X POST "${GUARDRAIL_ADMIN_URL}/v1/admin/tenants/${TENANT_ID}/keys/${KEY_ID}/revoke" \
  -H "authorization: Bearer ${GUARDRAIL_ADMIN_TOKEN}" \
  -H "content-type: application/json" \
  -d '{"reason":"rotation"}'
```

## Admin: Bind Route

```http
POST /v1/admin/tenants/{tenant_id}/routes
```

```bash
curl -sS -X POST "${GUARDRAIL_ADMIN_URL}/v1/admin/tenants/${TENANT_ID}/routes" \
  -H "authorization: Bearer ${GUARDRAIL_ADMIN_TOKEN}" \
  -H "content-type: application/json" \
  -d '{"route_id":"pilot-webhook"}'
```

## Admin: Verify Audit Integrity

```http
GET /v1/audit/integrity
```

```bash
curl -sS "${GUARDRAIL_ADMIN_URL}/v1/audit/integrity?from_execution_id=${FROM_ID}&to_execution_id=${TO_ID}" \
  -H "authorization: Bearer ${GUARDRAIL_ADMIN_TOKEN}" \
  | python3 -m json.tool
```
