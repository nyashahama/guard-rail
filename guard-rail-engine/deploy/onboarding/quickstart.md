# Guard Rail Quickstart

This quickstart proves the pilot flow with one protected route:

- create a tenant
- issue a tenant API key
- bind the tenant to a route
- send one allowed request
- send one blocked request
- fetch audit evidence
- run replay

This quickstart uses the `guard-rail-engine/examples/pilot-demo/` config, payloads, and upstream server that are part of the Phase 3 onboarding examples.

## Prerequisites

- Docker
- PostgreSQL reachable from the runtime
- `curl`
- `python3`
- a built `guard-rail-engine` image or local Rust toolchain

## Environment

```bash
export GUARDRAIL_DATABASE__URL=postgres://guardrail:secret@127.0.0.1:5432/guardrail
export GUARDRAIL_ADMIN__TOKEN=quickstart-admin-token
export GUARDRAIL_ENVIRONMENT=development
export GUARDRAIL_SERVER__HOST=127.0.0.1
export GUARDRAIL_SERVER__PORT=18080
```

Use development mode for this local quickstart so the sample upstream can run on `http://127.0.0.1`.

From the repository root, enter the engine directory once. Run the remaining commands from this directory; start separate terminals in this directory too.

```bash
cd guard-rail-engine
```

## Migrate

```bash
cargo run -- migrate --config ./config/config.yaml
```

Expected output:

```text
Migrations applied successfully
```

## Start The Local Upstream

In a separate terminal that is also in `guard-rail-engine`, start the pilot demo upstream:

```bash
python3 examples/pilot-demo/upstream.py
```

The upstream listens on `127.0.0.1:19090`. The `pilot-webhook` route forwards allowed requests to this local service.

## Start The Runtime

```bash
cargo run -- serve --config ./examples/pilot-demo/config.yaml
```

Readiness check:

```bash
curl -i http://127.0.0.1:18080/ready
```

Expected:

```text
HTTP/1.1 200 OK
```

## Create Tenant, Key, And Route Binding

```bash
TENANT_ID=$(
  curl -sS -X POST http://127.0.0.1:18081/v1/admin/tenants \
    -H "authorization: Bearer ${GUARDRAIL_ADMIN__TOKEN}" \
    -H "content-type: application/json" \
    -d '{"name":"quickstart"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])'
)

TENANT_KEY=$(
  curl -sS -X POST "http://127.0.0.1:18081/v1/admin/tenants/${TENANT_ID}/keys" \
    -H "authorization: Bearer ${GUARDRAIL_ADMIN__TOKEN}" \
    -H "content-type: application/json" \
    -d '{"name":"primary"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["raw_key"])'
)

curl -sS -X POST "http://127.0.0.1:18081/v1/admin/tenants/${TENANT_ID}/routes" \
  -H "authorization: Bearer ${GUARDRAIL_ADMIN__TOKEN}" \
  -H "content-type: application/json" \
  -d '{"route_id":"pilot-webhook"}'
```

## Send An Allowed Request

```bash
ALLOWED_RESPONSE=$(
  curl -sS -D /tmp/guardrail-allowed.headers \
    -X POST http://127.0.0.1:18080/v1/execute/pilot-webhook \
    -H "authorization: Bearer ${TENANT_KEY}" \
    -H "content-type: application/json" \
    -d @examples/pilot-demo/payloads/allowed.json
)

cat /tmp/guardrail-allowed.headers
printf '%s\n' "$ALLOWED_RESPONSE"
```

Expected:
- HTTP status from the local upstream is returned
- `x-guardrail-execution-id` is present

## Send A Blocked Request

```bash
curl -i -X POST http://127.0.0.1:18080/v1/execute/pilot-webhook \
  -H "authorization: Bearer ${TENANT_KEY}" \
  -H "content-type: application/json" \
  -d @examples/pilot-demo/payloads/blocked-callback.json
```

Expected:
- HTTP `403 Forbidden`
- response body contains a blocked verdict
- local upstream does not receive the request

## Fetch Audit Evidence

```bash
EXECUTION_ID=$(awk 'tolower($1)=="x-guardrail-execution-id:" {print $2}' /tmp/guardrail-allowed.headers | tr -d '\r')

curl -sS "http://127.0.0.1:18080/v1/audit/executions/${EXECUTION_ID}" \
  -H "authorization: Bearer ${TENANT_KEY}" \
  | python3 -m json.tool
```

Expected:
- `execution_id` matches the header value
- `verdict` is present
- `replay_available` is present

## Replay

```bash
curl -sS -X POST "http://127.0.0.1:18080/v1/replay/executions/${EXECUTION_ID}" \
  -H "authorization: Bearer ${TENANT_KEY}" \
  -H "content-type: application/json" \
  -d '{"policy_source":"snapshot"}' \
  | python3 -m json.tool
```

Expected:
- replay returns the original verdict and replay verdict
- replay does not call the upstream service

## Next Steps

- Copy policies from [policy cookbook](policy-cookbook.md).
- Use the [Docker pilot guide](docker-pilot-guide.md) for a design-partner install.
- Use the [webhook integration guide](integrations/webhooks.md) when pointing Zapier, Make, or custom webhook clients at Guard Rail.
