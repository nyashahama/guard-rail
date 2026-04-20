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
