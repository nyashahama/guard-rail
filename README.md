# Guard Rail

`guard-rail-engine` is the Rust runtime in this repo. Stage 5 adds production-hardening support for readiness, metrics, trace-context-aware request logging, graceful drain on shutdown, and baseline deployment artifacts.

## Guard Rail Engine Operations

### Canonical Pilot Deployment

The only supported pilot deployment path is:

`container image + external Postgres + reverse proxy/LB`

The container expects:
- a mounted config directory at `/etc/guard-rail-engine`
- env overrides for secrets such as `GUARDRAIL_DATABASE__URL` and `GUARDRAIL_ADMIN__TOKEN`
- migrations to be run separately from the normal service start path

Build the container image:

```bash
cd guard-rail-engine
docker build -t guard-rail-engine .
```

Run migrations:

```bash
docker run --rm \
  -v "$(pwd)/deploy/container:/etc/guard-rail-engine:ro" \
  --env-file ./deploy/container/guard-rail-engine.env.example \
  guard-rail-engine \
  migrate --config /etc/guard-rail-engine/config.yaml
```

Start the runtime:

```bash
docker run --rm \
  -p 8080:8080 \
  -v "$(pwd)/deploy/container:/etc/guard-rail-engine:ro" \
  --env-file ./deploy/container/guard-rail-engine.env.example \
  guard-rail-engine
```

`systemd` remains in the repo only as a deferred fallback example. It is not a supported Phase 5 production path.

## Data Operations

Retention cleanup is an explicit operator command, not part of normal startup:

```bash
cd guard-rail-engine
GUARDRAIL_DATABASE__URL="$TEST_DATABASE_URL" \
GUARDRAIL_ADMIN__TOKEN=phase3-admin-token \
cargo run -- cleanup --config ./deploy/container/config.yaml
```

See `guard-rail-engine/deploy/container/DATA_OPERATIONS.md` for cleanup, backup, restore, and rollback guidance.

## Local DB-Backed Testing

DB-backed backend tests require `TEST_DATABASE_URL` locally. CI provisions Postgres automatically for these commands:

```bash
cd guard-rail-engine
export TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:5432/guardrail

cargo test --test audit_api_test
cargo test --test auth_integration_test
cargo test --test replay_integration_test
```

The CI runtime-smoke lane runs the shipped entrypoints through the helper script:

```bash
cd guard-rail-engine
export GUARDRAIL_DATABASE__URL="$TEST_DATABASE_URL"
export GUARDRAIL_ADMIN__TOKEN=ci-admin-token
export GUARDRAIL_ENVIRONMENT=development
export GUARDRAIL_SERVER__HOST=127.0.0.1
export GUARDRAIL_SERVER__PORT=18080

./scripts/ci/runtime-smoke.sh
```

## Observability

The runtime exposes Prometheus metrics at `/metrics`. See `guard-rail-engine/deploy/observability/` for:
- Alert rules (`prometheus-alerts.yml`)
- Grafana dashboard (`grafana-dashboard.json`)
- Operator runbooks (`RUNBOOKS.md`)

## Verification

See `guard-rail-engine/deploy/verification/README.md` for the Phase 7 verification package covering load, resilience, and security verification.
