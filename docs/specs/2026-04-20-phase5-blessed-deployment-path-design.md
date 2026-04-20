# Guard Rail Phase 5 Blessed Deployment Path Design

**Date:** 2026-04-20

**Status:** Approved design for implementation planning

## Goal

Bless one production deployment path for the pilot by hardening the container runtime, removing secret-unsafe defaults from the image contract, and documenting a single supported operational model: `container image + external Postgres + reverse proxy/LB`.

## Scope

Phase 5 includes:
- treating the container image as the only supported pilot deployment artifact
- hardening `guard-rail-engine/Dockerfile`
- running the container as a non-root user
- removing the expectation that production config or placeholder secrets are baked into the image
- defining an explicit runtime contract for env-injected secrets and mounted config
- separating schema migration execution from normal app startup
- adding a container healthcheck contract
- documenting the canonical deployment shape with external Postgres and reverse proxy/LB
- providing a production-oriented env/config example without fake production defaults
- explicitly deferring `systemd` as a non-canonical path

Phase 5 excludes:
- hardening Kubernetes manifests or adding a Kubernetes deployment target
- supporting docker-compose as a production deployment path
- blessing bare-metal or `systemd` installs for production use
- changing core runtime semantics unrelated to deployment packaging
- adding HA Postgres, failover orchestration, or backup automation
- adding full dashboards, alerts, or runbooks beyond deployment-path documentation
- frontend or marketing-site work

## Design Choice

Three approaches were considered:

1. Bless both container and `systemd` as supported pilot deployment paths.
   - Rejected because it splits hardening, testing, and documentation across two different operational models before either one is fully trustworthy.

2. Bless only `container + external Postgres + reverse proxy/LB` and defer `systemd`.
   - Recommended because it gives the project one reproducible artifact, one startup contract, and one documentation path.

3. Jump directly to a broader orchestrated target such as Kubernetes.
   - Rejected because it adds platform complexity before the image and runtime contract are fully settled.

Phase 5 uses option 2.

## Current State

The repo already has the beginnings of a deployment story:
- `guard-rail-engine/Dockerfile`
- `guard-rail-engine/.dockerignore`
- `guard-rail-engine/deploy/systemd/guard-rail-engine.service`
- `guard-rail-engine/config/config.yaml`

That deployment story is not yet safe enough to bless:
- the image currently runs as root
- the image copies repo-managed config into the runtime filesystem
- the copied config contains placeholder production-like values such as `admin.token: "change-me"`
- the image does not define a healthcheck
- the repo docs describe both container and `systemd` paths without identifying one canonical production target

Phase 5 fixes that ambiguity by making the container path the only supported pilot contract.

## Canonical Deployment Contract

The blessed Phase 5 deployment shape is:
- Guard Rail runs as a container image
- Postgres runs externally to the container
- a reverse proxy or load balancer terminates TLS and forwards traffic to the main listener
- the admin listener is separately bound and exposed only on private infrastructure when enabled

Required pilot assumptions:
- single region
- one canonical image build
- one canonical runtime invocation
- environment variables and mounted files are the only runtime configuration inputs
- no mutable application state is stored in the container filesystem

Phase 5 is complete only when the project docs, image, and startup contract all match this model.

## Container Image Rules

The Docker image must be safe and minimal enough to bless as the pilot artifact.

Required behavior:
- build the `guard-rail-engine` binary in a builder stage
- copy only the release binary and strictly necessary runtime assets into the final stage
- run as a dedicated non-root user
- avoid writable application paths unless they are intentionally created
- expose the main listener port only
- define a healthcheck that exercises the container’s health/readiness contract

The image should not:
- bake placeholder production secrets
- require editing tracked config files inside the image to become deployable
- assume Postgres is local to the container

Phase 5 does not require distroless packaging if that slows the work down, but it does require a materially safer image contract than the current root-running baseline.

## Config And Secret Model

The blessed deployment path must separate repo examples from production secrets.

Required behavior:
- secrets such as database credentials and admin token are injected at runtime
- production deployments do not rely on the repo’s sample `config/config.yaml` as-is
- the deployment docs state which settings are required in the environment
- the image can start with a mounted config file plus env overrides, or with a production-oriented config file provided outside the repo artifact

Required production-sensitive settings:
- `GUARDRAIL_DATABASE__URL`
- `GUARDRAIL_ADMIN__TOKEN`
- `GUARDRAIL_ENVIRONMENT=production`
- listener host/port values when they differ from the defaults

Phase 5 should provide a production env example or deployment env file example, but it must not contain fake secrets presented as usable defaults.

## Migration Contract

Schema migration execution must be separate from normal service startup.

Required behavior:
- `guard-rail-engine migrate --config ...` is the canonical migration command
- the deployment docs explicitly run migrations as a separate operational step
- the normal service start path assumes the schema is already ready
- the container deployment path does not hide migrations inside the default app startup command

This separation keeps rollback and failure behavior understandable:
- a migration failure should fail the migration step
- an app startup failure should fail the app container

Phase 5 does not need a full migration job runner or orchestration system, but it does need a documented and testable split between migrate and serve.

## Reverse Proxy And Listener Model

The canonical deployment path assumes a reverse proxy or load balancer in front of the runtime.

Required behavior:
- public traffic reaches the main listener through the proxy/LB
- TLS termination happens at the proxy/LB, not inside Guard Rail
- the admin listener is not publicly exposed by default
- `/health`, `/ready`, and `/metrics` are documented relative to the canonical deployment path

The deployment docs must be explicit that:
- the main listener is the public runtime surface
- the admin listener, when enabled, belongs on private network paths only
- the container image does not attempt to manage certificates or edge proxy concerns

## Systemd Position

`systemd` is intentionally not a blessed Phase 5 production path.

Required repo posture:
- keep the existing unit file only as an example or fallback artifact
- do not document it as the canonical pilot deployment path
- do not spend Phase 5 scope hardening it to parity with the container path

This is a scope decision, not a claim that `systemd` is impossible. It is simply deferred so one deployment contract can be made trustworthy first.

## Documentation And Artifact Expectations

Phase 5 must update repo documentation so it stops implying multiple equally supported production stories.

Required documentation updates:
- `README.md` identifies the canonical deployment path as container-only
- `README.md` explains separate migration and service start steps
- `README.md` documents runtime-secret injection and external Postgres
- `OVERVIEW.md` aligns deployment claims with the single blessed path
- any production env example avoids fake-but-production-looking secrets

Required artifact updates:
- `guard-rail-engine/Dockerfile`
- `guard-rail-engine/.dockerignore` if needed to keep the build context clean
- a production-oriented env example file or documented env contract

## Testing Strategy

Phase 5 requires verification at the artifact and startup-contract level.

Required verification:
- build the container image locally
- run the migration command through the blessed container/runtime path against Postgres
- run the serve command through the blessed container/runtime path against migrated Postgres
- validate the healthcheck/startup behavior of the running container
- ensure the image runs as non-root

Acceptable verification surfaces:
- local `docker build`
- local `docker run` against disposable Postgres
- CI expansion only if it stays tightly scoped to the canonical path

Phase 5 is not complete until the container path is documented, reproducible, and materially safer than the current baseline.

## File Impact

The expected primary implementation surface for Phase 5 is:
- `guard-rail-engine/Dockerfile`
- `guard-rail-engine/.dockerignore`
- `guard-rail-engine/config/config.yaml`
- `README.md`
- `OVERVIEW.md`

Additional files may be touched only if a small deployment helper or env example is needed to make the container path explicit and reproducible.
