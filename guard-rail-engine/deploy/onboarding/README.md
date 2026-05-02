# Guard Rail Pilot Onboarding

This folder is the pilot onboarding entrypoint for runtime operators and integrators.

Guard Rail is currently positioned as a **pilot runtime** in this repository, not as a full control-plane product. The pilot surface is a containerized deployment path with:

- route + policy configuration from files
- tenant-bound and public routes
- JSON policy enforcement before upstream forwarding
- audit and replay persistence for traceability

### Pilot Documentation Index

- [Quickstart](./quickstart.md) for bringing up a minimal pilot runtime.
- [API Reference](./api-reference.md) for public, tenant, admin, audit, and replay endpoints.
- [Policy Cookbook](./policy-cookbook.md) for practical policy examples.
- [Docker Pilot Guide](./docker-pilot-guide.md) for container deployment in the supported path.
- [Webhook Guide](./webhooks-guide.md) for Zapier, Make, and custom webhook setups.
- [Scripted Demo](./scripted-demo.md) for end-to-end API-driven validation.

Use these docs as the canonical pilot onboarding sequence. They intentionally describe what is implemented and verified today rather than promising broader platform features.
