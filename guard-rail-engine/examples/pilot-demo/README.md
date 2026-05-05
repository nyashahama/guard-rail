# Pilot Demo

This demo runs a local upstream, starts Guard Rail, creates a tenant/key/route binding, sends one allowed request, sends one blocked request, fetches audit evidence, and runs replay.

## Prerequisites

- Rust toolchain
- `curl`
- `python3`
- PostgreSQL reachable through `GUARDRAIL_DATABASE__URL`

## Run

```bash
cd guard-rail-engine
export GUARDRAIL_DATABASE__URL=postgres://guardrail:secret@127.0.0.1:5432/guardrail
./examples/pilot-demo/run-demo.sh
```

The script creates a unique directory under the system temp directory and prints
the audit, replay, and upstream log paths. Set `GUARDRAIL_DEMO_TMP_DIR` to choose a
specific output directory.
