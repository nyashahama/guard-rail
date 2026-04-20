# Guard Rail Verification

This directory contains the Phase 7 pilot verification package for the blessed deployment path.

## Prerequisites

- Docker
- PostgreSQL reachable through `GUARDRAIL_DATABASE__URL`
- `GUARDRAIL_ADMIN__TOKEN`
- Rust toolchain installed
- Node dependencies installed for `npm audit`

## Result Artifacts

Verification results are written under `guard-rail-engine/tmp/verification/<timestamp>/`.

Each scenario writes a JSON summary with:
- `scenario`
- `status`
- `timestamp`
- `metrics`

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
