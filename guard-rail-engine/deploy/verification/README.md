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
