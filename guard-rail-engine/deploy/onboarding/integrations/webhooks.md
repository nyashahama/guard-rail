# Webhook Integration Guide

Guard Rail sits between an existing webhook client and the upstream webhook or API.

```text
Zapier / Make / custom client
  -> Guard Rail /v1/execute/{route_id}
  -> upstream webhook or API
```

The upstream payload shape does not need to change. The main change is the webhook URL and the tenant API key header.

## Common Setup

1. Create a route in `routes.yaml` with `auth_mode: tenant_bound`.
2. Set the route `upstream` to the original webhook or API URL the client used before Guard Rail.
3. Attach policies to the route.
4. Create a tenant.
5. Issue a tenant API key.
6. Bind the route to the tenant.
7. Replace the webhook URL in the client with the Guard Rail execute URL.
8. Add `authorization: Bearer <tenant-api-key>` for Guard Rail.

The `authorization` header authenticates the client to Guard Rail. The runtime strips `Authorization` before forwarding upstream so tenant API keys do not leak. If the upstream also needs auth, do not rely on the same incoming `Authorization` header; configure upstream auth outside this guide or use a different upstream-supported mechanism.

## Zapier

Use a webhook action that can set custom headers.

URL:

```text
https://guardrail.example.com/v1/execute/pilot-webhook
```

Headers:

```text
authorization: Bearer <tenant-api-key>
content-type: application/json
```

Body: keep the same JSON payload that Zapier previously sent to the upstream service.

## Make

Use the HTTP module.

Method:

```text
POST
```

URL:

```text
https://guardrail.example.com/v1/execute/pilot-webhook
```

Headers:

```text
authorization: Bearer <tenant-api-key>
content-type: application/json
```

Body type: raw JSON.

## Custom Webhook Client

```bash
curl -i -X POST https://guardrail.example.com/v1/execute/pilot-webhook \
  -H "authorization: Bearer ${GUARDRAIL_TENANT_KEY}" \
  -H "content-type: application/json" \
  -d '{"callback":"https://api.safe.example/hook","value":"ok"}'
```

## Blocked Responses

A policy block returns `403` and does not forward when audit persistence succeeds. If strict audit persistence fails, the runtime can return `503`.

For evaluated execution attempts where the runtime creates and persists an execution record, the response includes an execution id header:

```text
x-guardrail-execution-id: <execution-id>
```

Use that id to fetch audit evidence:

```bash
curl -sS "https://guardrail.example.com/v1/audit/executions/${EXECUTION_ID}" \
  -H "authorization: Bearer ${GUARDRAIL_TENANT_KEY}" \
  | python3 -m json.tool
```

## Pilot Notes

- Do not put the admin token in Zapier, Make, or client-side workflow tools.
- Use tenant API keys only for execution, audit, and replay calls.
- Start with one or two routes.
- Keep policies narrow enough that operators can explain every block.
- Use callback allowlists when payloads contain callback URLs.
- Use replay to compare original and current policy behavior without calling upstream.
