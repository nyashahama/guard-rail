# Guard Rail Verification

This directory contains the Phase 7 pilot verification package for the blessed deployment path.

## Prerequisites

- Docker
- PostgreSQL reachable through `GUARDRAIL_DATABASE__URL`
- `TEST_DATABASE_URL` for test-backed replay verification, or allow `replay-redaction.sh` to derive it from `GUARDRAIL_DATABASE__URL`
- `GUARDRAIL_ADMIN__TOKEN`
- Rust toolchain installed
- `cargo-audit` installed for Rust dependency audits
- Node dependencies installed for `npm audit` (optional)
- `GUARDRAIL_SERVER__HOST` and `GUARDRAIL_SERVER__PORT` (defaults to `127.0.0.1:18080`)

## Environment

```bash
export TEST_DATABASE_URL=postgres://guardrail:secret@127.0.0.1:5432/guardrail
export GUARDRAIL_DATABASE__URL="$TEST_DATABASE_URL"
export GUARDRAIL_ADMIN__TOKEN=phase7-admin-token
export GUARDRAIL_ENVIRONMENT=development
export GUARDRAIL_SERVER__HOST=127.0.0.1
export GUARDRAIL_SERVER__PORT=18080
```

For the database-degradation drill, the Postgres container name defaults to `guardrail-phase7-pg`. Override with:

```bash
export PHASE7_PG_CONTAINER=guardrail-postgres
```

## Result Artifacts

Verification results are written under `<repo-root>/tmp/verification/<timestamp>/`, regardless of the caller's current working directory.

Each scenario writes a JSON summary with:
- `scenario`
- `status`
- `timestamp`
- `metrics`

Wrapper-backed checks also retain the captured `cargo test` output beside the JSON artifact as `<scenario>.log` in the same timestamped result directory.

## Scenarios

### Allowed And Blocked Load

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
bash scripts/verification/load-allowed-and-blocked.sh
```

Expected:
- writes `load-allowed-and-blocked.json`
- exits `0`
- reports allowed and blocked request counts plus latency summaries

### Reload Under Traffic

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
bash scripts/verification/reload-under-traffic.sh
```

Expected:
- writes `reload-under-traffic.json`
- exits `0`
- background traffic continues across reload attempts
- valid candidate changes behavior as expected
- rejected candidate does not change behavior

### Drain Under Load

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
bash scripts/verification/drain-under-load.sh
```

Expected:
- writes `drain-under-load.json`
- exits `0`
- in-flight request returns successfully
- `/ready` transitions away from ready during drain

### Database Degradation

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
bash scripts/verification/db-degradation.sh
```

Expected:
- writes `db-degradation.json`
- exits `0`
- `/ready` returns `200` while DB is healthy
- `/ready` returns `503` after DB becomes unavailable
- restarts the DB container on cleanup

### Upstream Degradation

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
bash scripts/verification/upstream-degradation.sh
```

Expected:
- writes `upstream-degradation.json`
- exits `0`
- timeout drill returns `502`
- error drill returns `500`

### Hard Audit Mode

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
bash scripts/verification/hard-audit-mode.sh
```

Expected:
- writes `hard-audit-mode.json`
- writes `hard-audit-mode.log`
- exits `0`
- runs the focused smoke test for strict fail-closed audit behavior before forwarding upstream traffic
- requires concrete proof in captured cargo output that the exact smoke test ran and passed
- fails if `required_audit_mode_does_not_forward_without_pre_forward_intent` regresses

### Replay Redaction

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
bash scripts/verification/replay-redaction.sh
```

Expected:
- writes `replay-redaction.json`
- writes `replay-redaction.log`
- exits `0`
- runs the focused replay artifact redaction tests already present in the repo
- requires concrete proof in captured cargo output that both focused replay tests ran and passed
- verifies both request and response JSON persistence stay redacted

### Dependency Audits

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
bash scripts/verification/dependency-audits.sh
```

Expected:
- writes `dependency-audits.json`
- exits `0` when Rust and Node dependency audits pass
- exits `0` with status `skipped` in default mode when audit prerequisites are missing or skipped
- exits non-zero when `cargo audit` reports Rust dependency vulnerabilities
- exits non-zero when `npm audit --audit-level=high` reports high-or-above Node dependency vulnerabilities
- exits non-zero when `REQUIRE_DEPENDENCY_AUDIT=true` and audit prerequisites are missing or skipped
- `cargo audit` covers Rust dependencies
- `RUSTSEC-2023-0071` is explicitly ignored because it is pulled through SQLx's optional MySQL package in `Cargo.lock`; Guard Rail enables only Postgres SQLx features and `sqlx-mysql` is not in the active dependency tree
- `npm audit --audit-level=high` covers Node dependencies

## Running The Full Suite

```bash
cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine
export PHASE7_SUITE_ID=$(date -u +"%Y%m%dT%H%M%SZ")
bash scripts/verification/run-phase7-suite.sh
```

The suite orchestrates all scenarios in order and exits non-zero if any scenario fails. Results are aggregated under `<repo-root>/tmp/verification/${PHASE7_SUITE_ID}/`.

## Interpreting Results

Each scenario JSON contains:
- `status`: `pass`, `fail`, or `skipped`
- `timestamp`: UTC ISO8601-like string
- `metrics`: scenario-specific measurements

A failing suite means at least one scenario exited non-zero. Investigate the corresponding JSON file and any console output for the specific failure.

For wrapper-backed failures, inspect the sibling `.log` artifact in the same `<repo-root>/tmp/verification/<timestamp>/` directory for the preserved `cargo test` output.
