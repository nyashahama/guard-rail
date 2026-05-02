# Docker Pilot Guide

This guide reflects the supported pilot path for this repo: container image, external PostgreSQL, and fronting load balancer/reverse proxy.

## Runtime topology

Required in pilot:
- One Guard Rail runtime container mounted with:
  - `config.yaml`
  - `routes.yaml`
  - `policies/` directory
- External PostgreSQL reachable as DSN in `database.url`
- Optional reverse proxy for TLS and external host/route controls

### Config expectations to match pilot posture

- Production-grade policy checks are active with `required_before_response` persistence mode.
- Route files + policy files are mounted read-only in the container.
- Changes are validated on startup and reload.

## Build image

From repo root:

    docker build -t guard-rail-engine:pilot -f guard-rail-engine/Dockerfile guard-rail-engine

## Run with PostgreSQL and health check endpoints

Example using `/tmp/guard-rail-onboarding` from the quickstart.

    docker network create guardrail-onboard >/dev/null 2>&1 || true

    docker run -d --name guardrail-postgres --network guardrail-onboard \
      -e POSTGRES_DB=guardrail \
      -e POSTGRES_USER=guardrail \
      -e POSTGRES_PASSWORD=guardrail \
      -p 5432:5432 \
      postgres:16

    docker run --rm --network guardrail-onboard \
      -v /tmp/guard-rail-onboarding:/etc/guard-rail-engine:ro \
      guard-rail-engine:pilot \
      migrate --config /etc/guard-rail-engine/config.yaml

    docker run -d --network guardrail-onboard \
      --name guardrail-engine \
      -p 8080:8080 \
      -p 8081:8081 \
      -v /tmp/guard-rail-onboarding:/etc/guard-rail-engine:ro \
      guard-rail-engine:pilot

Expected:
- `curl -i http://localhost:8080/health` -> `200 ok`
- `curl -i http://localhost:8080/ready` -> `200`

If you need admin API checks from this setup, use:
- `curl -i -H "Authorization: Bearer pilot-admin-token" "http://localhost:8081/v1/admin/tenants"`
- `curl -i -H "Authorization: Bearer pilot-admin-token" "http://localhost:8081/v1/audit/integrity?from_execution_id=<uuid>&to_execution_id=<uuid>"`

## Reverse proxy notes

In a pilot deployment, place TLS termination and trusted source controls in front of `8080`.

Typical checks:
- proxy health check to `/ready`
- pass all request methods for pilot routes to `/v1/execute/{route_id}`
- avoid exposing admin endpoints publicly; route admin listener separately where possible.

## Reload and rollout behavior

Guard Rail watches `routes.yaml` and `policies/` on disk. In pilot containers, this means:
- edit files in the host mount path
- wait for reload log event
- verify readiness remains green

Quick readiness check after change:

    curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/ready

## Graceful shutdown behavior

On SIGTERM, runtime enters draining, then exits after grace period in config.

    docker stop guard-rail-engine

Use this for deploy rollouts so in-flight requests are handled consistently.

## Pilot command snippets

Run migrations manually:

    docker run --rm --network guardrail-onboard \
      -v /tmp/guard-rail-onboarding:/etc/guard-rail-engine:ro \
      guard-rail-engine:pilot \
      migrate --config /etc/guard-rail-engine/config.yaml

Run cleanup preview:

    docker run --rm --network guardrail-onboard \
      -v /tmp/guard-rail-onboarding:/etc/guard-rail-engine:ro \
      guard-rail-engine:pilot \
      cleanup --config /etc/guard-rail-engine/config.yaml

Apply cleanup:

    docker run --rm --network guardrail-onboard \
      -v /tmp/guard-rail-onboarding:/etc/guard-rail-engine:ro \
      guard-rail-engine:pilot \
      cleanup --apply --config /etc/guard-rail-engine/config.yaml

## Pilot boundaries

This is still a pilot setup:
- no managed SaaS control plane is included
- no tenant UI
- no separate enterprise policy editor

Keep docs and automation aligned to the runtime-only contract above.
