# Webhook Integration Guide

Guard Rail sits between an existing webhook client and the upstream webhook or API.

```text
Zapier / Make / custom client
  -> Guard Rail /v1/execute/{route_id}
  -> upstream webhook or API
```

The upstream payload shape does not need to change. The main change is the webhook URL and the tenant API key header.

## Common Setup

1. Create a route in `routes.yaml`.
2. Attach policies to the route.
3. Create a tenant.
4. Issue a tenant API key.
5. Bind the route to the tenant.
6. Replace the webhook URL in the client with the Guard Rail execute URL.
7. Add `authorization: Bearer <tenant-api-key>`.

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

A blocked policy returns `403` and does not forward to upstream.

The response includes an execution id header:

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
