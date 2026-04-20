# Guard Rail

`guard-rail-engine` is the Rust runtime in this repo. Stage 5 adds production-hardening support for readiness, metrics, trace-context-aware request logging, graceful drain on shutdown, and baseline deployment artifacts.

## Guard Rail Engine Operations

Run migrations:

```bash
cd guard-rail-engine
cargo run -- migrate --config ./config/config.yaml
```

Serve locally:

```bash
cd guard-rail-engine
cargo run -- serve --config ./config/config.yaml
```

Build the container image:

```bash
cd guard-rail-engine
docker build -t guard-rail-engine .
```

Install the systemd unit:

```bash
sudo cp guard-rail-engine/deploy/systemd/guard-rail-engine.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now guard-rail-engine
```

Operational endpoints:
- `GET /health`
- `GET /ready`
- `GET /metrics`

Container and service artifacts:
- `guard-rail-engine/Dockerfile`
- `guard-rail-engine/.dockerignore`
- `guard-rail-engine/deploy/systemd/guard-rail-engine.service`

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
