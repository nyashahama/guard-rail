# Webhooks Guide (Pilot)

This guide maps Guard Rail to common webhook clients in a pilot deployment.

## General webhook pattern

Treat each outgoing callback or inbound integration as a route:
- create a route id (for example `partner-webhook`)
- define allowed callback behavior in policy (`callback-allowlist`)
- enforce tenant/API key auth for partner-sourced traffic if needed

Execution URL format:

    https://<guard-rail-host>/v1/execute/<route_id>

Pilot note: there is no webhook-specific connector inside runtime yet; integrations call the runtime like any other HTTP client.

## Zapier

Use a Zapier Webhook action with:
- URL: `https://guard-rail.example.com/v1/execute/partner-webhook`
- Method: `POST`
- Headers:
  - `Authorization: Bearer <tenant_key>` for tenant-bound routes
  - `Content-Type: application/json`
- Body: JSON payload with `callback` field if policy checks include it

Expected behavior:
- valid payload → Zapier receives upstream response (proxy pass-through)
- blocked payload → `403` block payload with `policy` and `rule_field`

Example blocked response in a Zapier logs stream:

```json
{"status":"blocked","execution_id":"GR-EXE-...","policy":"callback-allowlist","rule_field":"$.callback","message":"domain_not_in condition triggered on field $.callback"}
```

## Make.com

Use HTTP module with:
- Method `POST`
- URL above (`/v1/execute/<route_id>`)
- Header `Authorization` set from Make data store
- Parse response status and payload in the module

Recommended pattern:
- use Make as a thin connector
- keep heavy payload transforms in Make or your service, not in route policy

## Custom webhooks

For in-house senders, keep the same contract:

    curl -X POST "https://guard-rail.example.com/v1/execute/partner-webhook" \
      -H "Authorization: Bearer $TENANT_KEY" \
      -H "Content-Type: application/json" \
      -d '{"callback":"https://api.safe.com/webhook","event":"invoice.paid","id_number":"8501015009087"}'

Expected results:
- route and methods match: runtime forwards when policy allows
- callback/domain check fails: `403` with block body
- oversized or malformed JSON: `400`

## Recommended payload model

Keep webhook payloads explicit and policy-friendly:

```json
{
  "callback": "https://api.safe.com/webhook",
  "event": "invoice.paid",
  "payload": {"...": "..."},
  "id_number": "8501015009087"
}
```

Then apply:
- `callback-allowlist` for `$.callback`
- `sa-id-pii-block` for `$.id_number` if sensitive data should be blocked
- `payload-size-limit` for total size control

## Pilot guardrails to keep

- Add one webhook endpoint per route until policy behavior is stable.
- Prefer tenant-bound routes for partner-supplied calls so key rotation can be done by tenant.
- Use audit endpoint (`GET /v1/audit/executions`) to investigate blocked/allowed webhook outcomes during pilot.
- Route auth mode and policy changes are deployable without code, via file reload.
