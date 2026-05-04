# Docker Pilot Guide

The supported pilot deployment path is:

```text
container image + external Postgres + reverse proxy/LB
```

This guide assumes Postgres is managed outside the Guard Rail container and secrets are injected through environment variables.

## Required Files

Mount a config directory like this:

```text
/etc/guard-rail-engine/
  config.yaml
  routes.yaml
  policies/
    security.yaml
```

Use the repo samples in `guard-rail-engine/deploy/container/` as the starting point.

## Required Environment

Do not run with the placeholder environment example as-is. Replace placeholders with real values and pass those values explicitly into the container.

```bash
export GUARDRAIL_DATABASE__URL=postgres://guardrail:<password>@postgres.internal:5432/guardrail
export GUARDRAIL_ADMIN__TOKEN=<set-a-real-admin-token>
export GUARDRAIL_ENVIRONMENT=production
export GUARDRAIL_SERVER__HOST=0.0.0.0
export GUARDRAIL_SERVER__PORT=8080
```

Production validation requires:

- non-empty `database.url`
- `audit.persistence_mode: required_before_response`
- non-zero `audit.write_timeout_ms`
- non-zero request body limit
- non-empty replay redaction policy when replay is enabled
- admin listener not bound to `0.0.0.0`
- non-empty, non-default admin token when admin listener is enabled

## Migrate

```bash
docker run --rm \
  -v "$PWD/guard-rail-engine/deploy/container:/etc/guard-rail-engine:ro" \
  --env GUARDRAIL_DATABASE__URL \
  --env GUARDRAIL_ADMIN__TOKEN \
  --env GUARDRAIL_ENVIRONMENT \
  --env GUARDRAIL_SERVER__HOST \
  --env GUARDRAIL_SERVER__PORT \
  guard-rail-engine \
  migrate --config /etc/guard-rail-engine/config.yaml
```

## Run

```bash
docker run --rm \
  --name guard-rail-engine \
  -p 8080:8080 \
  -v "$PWD/guard-rail-engine/deploy/container:/etc/guard-rail-engine:ro" \
  --env GUARDRAIL_DATABASE__URL \
  --env GUARDRAIL_ADMIN__TOKEN \
  --env GUARDRAIL_ENVIRONMENT \
  --env GUARDRAIL_SERVER__HOST \
  --env GUARDRAIL_SERVER__PORT \
  guard-rail-engine \
  serve --config /etc/guard-rail-engine/config.yaml
```

Place a reverse proxy or load balancer in front of the main listener. Keep the admin listener on loopback or a private operator network. With the sample config, the admin listener binds to `127.0.0.1` inside the container, so it is intentionally not published to the host.

## Verify

```bash
curl -i http://127.0.0.1:8080/ready
curl -sS http://127.0.0.1:8080/metrics
docker exec guard-rail-engine curl -sS http://127.0.0.1:8081/v1/admin/tenants \
  -H "authorization: Bearer ${GUARDRAIL_ADMIN__TOKEN}"
```

Run the Phase 7 verification suite before pilot traffic:

```bash
cd guard-rail-engine
export PHASE7_SUITE_ID=$(date -u +"%Y%m%dT%H%M%SZ")
bash scripts/verification/run-phase7-suite.sh
```

## Pilot Install Checklist

- External Postgres is reachable from the runtime.
- Migrations have been applied.
- Config and policies are mounted read-only.
- Admin token is injected as a secret.
- Main listener is behind the customer reverse proxy or load balancer.
- Admin listener is not publicly exposed.
- `/ready` returns `200`.
- `/metrics` is scraped when observability is enabled.
- Backup and cleanup procedures are agreed before pilot traffic.
- At least one tenant API key has been issued and stored securely.
- Each tenant-bound route is bound to the expected tenant.

## Data Operations

Use `guard-rail-engine/deploy/container/DATA_OPERATIONS.md` for cleanup, backup, restore, and rollback notes.

## Not The Phase 3 Blessed Path

- Docker Compose as a production target.
- Standalone VM install without container image discipline.
- Kubernetes-first control-plane automation.
- Self-serve tenant provisioning.
