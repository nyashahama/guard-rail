# Guard Rail

Guard Rail is a policy enforcement runtime for internal API traffic. It sits between trusted clients and upstream APIs, evaluates requests against route and policy definitions, and either forwards the request or blocks it with an auditable verdict.

The repo has two user-facing parts:
- `guard-rail-engine/` for the Rust runtime
- `app/` for the marketing site, which is separate from the runtime

## Blessed deployment path

The supported pilot deployment is:

`container image + external Postgres + reverse proxy/LB`

That path assumes:
- the runtime is run from the built container image
- Postgres is managed outside the container
- a reverse proxy or load balancer fronts the runtime
- runtime config is mounted from disk and secrets are injected through the environment

The runtime is not positioned as a standalone VM install, a Docker Compose production target, or a Kubernetes-first control plane.

## What the runtime does

The engine currently covers the pilot-grade request flow:
- route lookup and method checks
- request authentication and authorization surfaces
- policy evaluation before forwarding
- audit persistence for executed requests
- replay capture and related runtime artifacts
- readiness and metrics surfaces for operations

## Docs

Start here for pilot onboarding:
- Pilot onboarding: [guard-rail-engine/deploy/onboarding/README.md](guard-rail-engine/deploy/onboarding/README.md)
- Quickstart: [guard-rail-engine/deploy/onboarding/quickstart.md](guard-rail-engine/deploy/onboarding/quickstart.md)
- API reference: [guard-rail-engine/deploy/onboarding/api-reference.md](guard-rail-engine/deploy/onboarding/api-reference.md)
- Policy cookbook: [guard-rail-engine/deploy/onboarding/policy-cookbook.md](guard-rail-engine/deploy/onboarding/policy-cookbook.md)
- Docker pilot guide: [guard-rail-engine/deploy/onboarding/docker-pilot-guide.md](guard-rail-engine/deploy/onboarding/docker-pilot-guide.md)
- Webhook integrations: [guard-rail-engine/deploy/onboarding/integrations/webhooks.md](guard-rail-engine/deploy/onboarding/integrations/webhooks.md)

Use these repo-native docs for supported operator workflows:
- Data operations: [guard-rail-engine/deploy/container/DATA_OPERATIONS.md](guard-rail-engine/deploy/container/DATA_OPERATIONS.md)
- Observability: [guard-rail-engine/deploy/observability/](guard-rail-engine/deploy/observability/)
- Verification: [guard-rail-engine/deploy/verification/README.md](guard-rail-engine/deploy/verification/README.md)

## Current scope

Guard Rail is focused on the runtime and its pilot deployment story. It is intentionally not framed here as a full SaaS platform, management console, or multi-environment control plane.

For runtime surfaces and repo-local evaluation, start with the pilot onboarding docs above.
