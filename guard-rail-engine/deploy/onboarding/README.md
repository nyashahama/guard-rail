# Guard Rail Pilot Onboarding

Guard Rail is a pilot runtime for enforcing policies in front of internal API and webhook traffic. This onboarding path is for design partners running the supported deployment model:

```text
container image + external Postgres + reverse proxy/LB
```

## Start Here

1. Read the [quickstart](quickstart.md) to run one allowed request, one blocked request, one audit lookup, and one replay.
2. Use the [API reference](api-reference.md) when wiring tenants, route bindings, audit lookup, and replay calls.
3. Copy policy templates from the [policy cookbook](policy-cookbook.md).
4. Follow the [Docker pilot guide](docker-pilot-guide.md) for the blessed deployment path.
5. Use the [webhook integration guide](integrations/webhooks.md) for Zapier, Make, and custom webhook clients.

## What This Is

- A runtime that receives requests at `/v1/execute/{route_id}`.
- A policy gate that allows or blocks requests before forwarding to configured upstreams.
- An audit and replay surface for pilot operations.
- A tenant API-key model for controlled route access.

## What This Is Not

- A hosted SaaS control plane.
- A self-serve customer dashboard.
- A billing or procurement portal.
- A Zapier or Make marketplace app.
- A general-purpose API gateway replacement.

## Pilot Success Criteria

A first pilot should prove:

- time to first policy is under one working session
- one or two real routes can be protected without upstream application code changes
- risky payloads are blocked before they reach the upstream service
- audit evidence is available for allowed, blocked, and rejected requests
- replay output is understandable to the operator
