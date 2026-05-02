# Quickstart

This guide is the fastest practical path to a running pilot runtime.

## 1) Prerequisites

- Docker running on your workstation or test host.
- PostgreSQL reachable from the runtime container.
- `curl` and `jq` available for API calls.

## 2) Create a pilot config

In this repository, use the existing container layout as the base and replace values only where needed.

    mkdir -p /tmp/guard-rail-onboarding/policies

    cat > /tmp/guard-rail-onboarding/routes.yaml <<'EOF'
    routes:
      - id: pilot-webhook
        auth_mode: tenant_bound
        upstream: https://postman-echo.com/post
        methods: [POST]
        policies: [callback-allowlist, payload-size-limit]
        timeout_ms: 4000
    EOF

    cp guard-rail-engine/deploy/onboarding/policies/*.yaml /tmp/guard-rail-onboarding/policies/

    cat > /tmp/guard-rail-onboarding/config.yaml <<'EOF'
    environment: development
    server:
      host: "0.0.0.0"
      port: 8080
      request_body_limit_bytes: 1048576

    routes_file: "/etc/guard-rail-engine/routes.yaml"
    policies_dir: "/etc/guard-rail-engine/policies/"

    forwarding:
      default_timeout_ms: 5000
      user_agent: "GuardRail/0.1.0"

    logging:
      level: "info"
      format: "json"

    database:
      url: "postgres://guardrail:guardrail@postgres:5432/guardrail"
      max_connections: 10

    audit:
      write_timeout_ms: 250
      persistence_mode: required_before_response

    admin:
      token: "pilot-admin-token"

    admin_server:
      host: "0.0.0.0"
      port: 8081

    rate_limit:
      requests_per_minute: 120
      burst: 30

    replay:
      enabled: true

    observability:
      metrics_enabled: true
      metrics_path: "/metrics"

    shutdown:
      grace_period_ms: 15000
      drain_poll_interval_ms: 50

    data_ops:
      audit_retention_days: 180
      artifact_retention_days: 30
      replay_run_retention_days: 30
      orphan_snapshot_retention_days: 30
      cleanup_batch_size: 1000
    EOF

## 3) Start supporting infrastructure

    docker network create guard-rail-onboard >/dev/null 2>&1 || true

    docker run -d --name guardrail-pg --network guard-rail-onboard \
      -e POSTGRES_DB=guardrail \
      -e POSTGRES_USER=guardrail \
      -e POSTGRES_PASSWORD=guardrail \
      postgres:16

## 4) Build and run Guard Rail

From repo root:

    docker build -t guard-rail-engine:pilot -f guard-rail-engine/Dockerfile guard-rail-engine

    docker run --rm --network guard-rail-onboard \
      -v /tmp/guard-rail-onboarding:/etc/guard-rail-engine:ro \
      --name guardrail-migrations guard-rail-engine:pilot \
      migrate --config /etc/guard-rail-engine/config.yaml

    docker run -d --network guard-rail-onboard \
      --name guardrail-engine \
      -p 8080:8080 -p 8081:8081 \
      -v /tmp/guard-rail-onboarding:/etc/guard-rail-engine:ro \
      guard-rail-engine:pilot \
      serve --config /etc/guard-rail-engine/config.yaml

Expected startup checks:

- `curl -i http://localhost:8080/ready` returns `200`.
- `curl -i http://localhost:8080/health` returns `200 ok`.

## 5) Smoke test the pilot path

Create a tenant, issue an API key, and bind the route.

    export GUARDRAIL_ADMIN_URL="http://localhost:8081"
    export GUARDRAIL_RUN_URL="http://localhost:8080"

    TENANT_ID=$(curl -s -X POST "$GUARDRAIL_ADMIN_URL/v1/admin/tenants" \
      -H "Authorization: Bearer pilot-admin-token" \
      -H "Content-Type: application/json" \
      -d '{"name":"acme"}' | jq -r '.id')

    API_KEY=$(curl -s -X POST "$GUARDRAIL_ADMIN_URL/v1/admin/tenants/$TENANT_ID/keys" \
      -H "Authorization: Bearer pilot-admin-token" \
      -H "Content-Type: application/json" \
      -d '{"name":"service-client"}' | jq -r '.raw_key')

    curl -s -X POST "$GUARDRAIL_ADMIN_URL/v1/admin/tenants/$TENANT_ID/routes" \
      -H "Authorization: Bearer pilot-admin-token" \
      -H "Content-Type: application/json" \
      -d '{"route_id":"pilot-webhook"}'

Allowed request:

    curl -i -X POST "$GUARDRAIL_RUN_URL/v1/execute/pilot-webhook" \
      -H "Authorization: Bearer $API_KEY" \
      -H "Content-Type: application/json" \
      -d '{"callback":"https://api.safe.com/hook","notes":"onboarding test"}'

Expected:

- `200 OK`
- Upstream response body from `postman-echo.com` includes your posted JSON in a success shape.

Blocked request:

    curl -i -X POST "$GUARDRAIL_RUN_URL/v1/execute/pilot-webhook" \
      -H "Authorization: Bearer $API_KEY" \
      -H "Content-Type: application/json" \
      -d '{"callback":"https://evil.partner.example/callback","amount":100}'

Expected:

```json
HTTP/1.1 403 Forbidden
{
  "status":"blocked",
  "execution_id":"GR-EXE-...",
  "policy":"callback-allowlist",
  "rule_field":"$.callback",
  "message":"domain_not_in condition triggered on field $.callback"
}
```

## 6) Where to go next

- Review the [Policy Cookbook](./policy-cookbook.md) before adding production rules.
- Read the [Docker Pilot Guide](./docker-pilot-guide.md) for hardened deploy posture and operational checks.
- Follow the [API Reference](./api-reference.md) for endpoint behavior.
