# Guard Rail Phase 4 CI Reality Design

**Date:** 2026-04-20

**Status:** Approved design for implementation planning

## Goal

Make CI reflect the real backend runtime surface by automatically provisioning Postgres, running the DB-backed test binaries, keeping smoke coverage in CI, and validating the shipped `migrate` and `serve` entrypoints against an ephemeral database.

## Scope

Phase 4 includes:
- updating GitHub Actions in `.github/workflows/ci.yml`
- keeping a fast backend lane for unit and non-DB integration coverage
- adding CI-managed Postgres for DB-backed backend tests
- running `audit_api_test`, `auth_integration_test`, and `replay_integration_test` in CI
- running `smoke_test` in CI
- adding a runtime startup-smoke path that executes `cargo run -- migrate --config ./config/config.yaml`
- adding a runtime startup-smoke path that executes `cargo run -- serve --config ./config/config.yaml`
- probing `/health` and `/ready` during CI startup smoke
- injecting CI-only runtime config via environment variables instead of repo-only manual setup
- ensuring backend failures are attributed to the lane that failed instead of being hidden behind a partial test command

Phase 4 excludes:
- changing backend runtime semantics
- changing the deployment artifact or container hardening strategy
- adding Kubernetes, Compose, or systemd orchestration to CI
- adding performance, load, fuzz, or resilience testing
- adding a full benchmark pipeline
- redesigning the test harness architecture beyond what is needed to make CI truthful
- frontend or marketing-site work

## Design Choice

Three approaches were considered:

1. Extend the current backend test job with Postgres and keep one large test lane.
   - Rejected because it slows diagnosis and makes it unclear whether a failure came from unit coverage, DB-backed coverage, or real process startup.

2. Split backend CI into fast, DB-backed, and runtime-smoke lanes.
   - Recommended because it preserves fast feedback while making the real backend surface explicit and debuggable.

3. Put every backend test binary in its own GitHub Actions job.
   - Rejected for now because it increases YAML and runner overhead more than this phase requires.

Phase 4 uses option 2.

## Architecture

Phase 4 keeps a single GitHub Actions workflow but changes the backend portion from partial coverage to explicit lanes:
- `engine-check`
- `engine-test-fast`
- `engine-test-db`
- `engine-runtime-smoke`
- `frontend-check`

The new backend lanes share the same source checkout and Rust toolchain setup pattern, but they do not share runtime state. Each lane must be independently trustworthy.

This phase does not introduce a second workflow file. The goal is to make the existing CI workflow honest, not to fragment it.

## CI Contract

Phase 4 is complete only when all of the following are true:
- CI fails on unit, non-DB integration, smoke, DB-backed API, auth, or replay regressions
- CI provisions Postgres automatically for DB-backed work
- CI does not rely on a manually created `TEST_DATABASE_URL`
- CI proves `cargo run -- migrate` works against a fresh ephemeral database
- CI proves `cargo run -- serve` can start successfully against that migrated database
- CI can reach `/health` and `/ready` on the started process
- backend failures are isolated enough that the broken surface is obvious from the GitHub Actions job view

## Workflow Model

### Fast Backend Lane

`engine-test-fast` should run only coverage that does not require a live Postgres instance:
- `cargo test --lib`
- `cargo test --test integration_test`
- `cargo test --test smoke_test`

This lane should remain the quickest backend signal and should not depend on service containers.

### DB-Backed Backend Lane

`engine-test-db` should provision a Postgres service in GitHub Actions and export:
- `TEST_DATABASE_URL`

The lane should then run:
- `cargo test --test audit_api_test`
- `cargo test --test auth_integration_test`
- `cargo test --test replay_integration_test`

The commands should run as separate steps so the failing backend surface is obvious in CI output.

This lane is the authoritative CI signal for audit, tenant auth, replay persistence, and database-backed API behavior.

### Runtime Smoke Lane

`engine-runtime-smoke` should provision the same Postgres service pattern and run the real binary entrypoints rather than an in-process test harness.

Required steps:
- run `cargo run -- migrate --config ./config/config.yaml`
- start `cargo run -- serve --config ./config/config.yaml` in the background
- wait for the server to bind
- probe `GET /health`
- probe `GET /ready`
- fail with logs if the process exits early or probes do not succeed

This lane validates the repo’s actual startup contract, not just library-level behavior.

## Postgres Service Model

Phase 4 should use a GitHub Actions service container, not Docker-in-Docker and not a repo-managed Compose file.

Recommended service shape:
- image: `postgres:16`
- fixed user, password, and database for CI only
- healthcheck using `pg_isready`

Recommended connection style:
- `postgres://guardrail:secret@127.0.0.1:5432/guardrail`

The exact database name may differ between lanes, but each job should use a single explicit CI-owned URL instead of hidden setup.

The workflow should wait for Postgres health before test execution. It should not assume the database is ready immediately after container startup.

## Runtime Configuration In CI

Phase 4 should prefer environment overrides over editing repo config files in CI.

Required override behavior:
- set `GUARDRAIL_DATABASE__URL` to the CI Postgres URL for runtime smoke
- set `GUARDRAIL_ADMIN__TOKEN` to a non-default CI token
- keep `GUARDRAIL_ENVIRONMENT=development` for startup smoke unless the sample config is changed to a production-safe fixture first

Optional override behavior:
- set `GUARDRAIL_SERVER__HOST=127.0.0.1`
- set `GUARDRAIL_SERVER__PORT` to an explicit CI-safe port

Phase 4 should not mutate tracked config files during the workflow just to make startup succeed.

## Startup Smoke Rules

The runtime smoke lane should be intentionally small and stable.

Required behavior:
- validate that migrations apply cleanly to the CI database
- validate that the binary starts after migrations
- validate that `/health` returns success
- validate that `/ready` returns success after startup
- collect process logs when startup or probing fails

Important limits:
- startup smoke should not execute sample upstream calls
- startup smoke should not depend on external internet access
- startup smoke should not try to validate full forwarding behavior because that is already covered elsewhere

This keeps the lane focused on process start, schema readiness, and listener health.

## Integrity Coverage Rule

Earlier phase planning required one integrity-oriented CI check. The current repo does not yet have a dedicated GitHub Actions lane for that, and it does not need one in this phase.

Phase 4 should satisfy that requirement by ensuring the DB-backed audit surface includes at least one integrity-oriented regression path in the suite executed by `engine-test-db`.

Acceptable outcomes for this phase:
- an existing audit integrity regression test is run in CI, or
- a focused audit integrity regression test is added to `audit_api_test.rs` and then run in CI

Phase 4 does not need a separate integrity workflow unless the coverage proves too slow or too noisy inside the DB lane.

## Failure And Observability Rules

CI should optimize for actionable failures, not minimum YAML length.

Required behavior:
- separate failing commands into separate steps where practical
- show which binary failed without requiring log archaeology
- preserve server logs for runtime-smoke failure diagnosis
- avoid a single `cargo test` command that hides which DB-backed surface broke

The workflow may keep shared setup via repeated YAML blocks or anchors if that improves clarity, but readability is more important than deduplicating every line.

## Documentation Expectations

Phase 4 should keep documentation changes small and practical.

Required documentation updates:
- update `.github/workflows/ci.yml` comments or step names so the lane purpose is obvious
- add a short README note describing that DB-backed backend tests require `TEST_DATABASE_URL` locally while CI provisions it automatically

This phase does not need a large CI operations guide.

## Testing Strategy

Phase 4 requires verification at three levels:

1. Workflow structure verification
   - confirm the workflow defines the new backend lanes and Postgres service usage where required

2. Local command verification
   - run the non-DB commands locally
   - compile-check or run the DB-backed binaries locally when `TEST_DATABASE_URL` is available

3. CI runtime verification
   - confirm the workflow runs the real binary startup path and fails when startup breaks

The implementation is not complete until CI tests the same backend surfaces that engineers rely on locally for merge confidence.

## File Impact

The expected primary implementation surface for Phase 4 is:
- `.github/workflows/ci.yml`
- `README.md`
- `guard-rail-engine/tests/audit_api_test.rs`

Additional files should stay limited to a small helper or focused regression test surface only when required to keep the CI lanes truthful and stable.
