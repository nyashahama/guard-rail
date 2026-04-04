# Guard Rail Backend — Stage 1: Core Proxy + Policy Engine

## Overview

An HTTP API gateway in Rust that receives webhook/API payloads, evaluates them against YAML-defined policies using JSONPath field inspection, and either forwards allowed requests to a configured upstream or blocks them. Structured JSON logging to stdout.

This is Stage 1 of 5. Later stages add: PostgreSQL persistence + audit trail (Stage 2), multi-tenancy + API keys (Stage 3), replay engine (Stage 4), observability + production hardening (Stage 5).

## Architecture

Guard Rail runs as a single async Rust binary. It exposes one primary endpoint pattern: `POST /v1/execute/{route_id}`. Each route maps to an upstream URL and a set of policies.

### Request Flow

```
Client (webhook source, partner API, AI agent)
  → POST /v1/execute/{route_id}
  → Validate: is body valid JSON? → 400 if not
  → Match route: does route_id exist? correct HTTP method? → 404/405 if not
  → Policy evaluation: load policies bound to this route
    → For each policy, for each rule:
      → Extract field value via JSONPath
      → Evaluate condition (domain check, regex, equals, etc.)
      → If any rule triggers BLOCK → short-circuit, return 403
  → If all policies pass: forward request to upstream URL
    → Strip Authorization header, add X-GuardRail-* headers
    → Return upstream response to caller transparently
  → Log execution as one structured JSON line to stdout
```

Every request gets a unique execution ID: `GR-EXE-{uuid}`.

### Deployment Model

API gateway mode. Customers change their webhook URLs to point at Guard Rail instead of directly at their internal APIs. Example: Zapier sends to `https://gw.guardrail.co.za/v1/execute/transfer-api` instead of `https://internal-bank.za/api/v1/transfer`.

## Rust Stack

| Crate | Purpose |
|-------|---------|
| `axum` | HTTP server framework |
| `tokio` | Async runtime |
| `reqwest` | HTTP client for upstream forwarding |
| `serde` + `serde_yaml` + `serde_json` | Serialization, config/policy/payload parsing |
| `jsonpath-rust` | JSONPath extraction from payloads |
| `regex` | Pattern matching for regex conditions |
| `notify` | File watcher for hot-reload of routes/policies |
| `tracing` + `tracing-subscriber` | Structured logging |
| `uuid` | Execution ID generation |
| `tower-http` | Body size limits, request ID middleware |
| `clap` | CLI argument parsing (config file path) |
| `chrono` | Timestamps |
| `url` | URL/domain parsing for domain conditions |

## Configuration

### config.yaml — Server Settings

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  request_body_limit_bytes: 1048576  # 1MB

routes_file: "./routes.yaml"
policies_dir: "./policies/"

forwarding:
  default_timeout_ms: 5000
  user_agent: "GuardRail/0.1.0"

logging:
  level: "info"     # debug, info, warn, error
  format: "json"    # json or pretty (for local dev)
```

Environment variables override any config value using double-underscore nesting: `GUARDRAIL_SERVER__PORT=9090`.

Server settings (`host`, `port`, `request_body_limit_bytes`) require a restart. Routes and policies hot-reload on file change.

### routes.yaml — Route Definitions

```yaml
routes:
  - id: transfer-api
    path: /v1/execute/transfer-api
    upstream: https://internal-bank.za/api/v1/transfer
    methods: [POST, PUT]
    policies: [block-external-callbacks, pii-detection]
    timeout_ms: 5000

  - id: partner-webhook
    path: /v1/execute/partner-webhook
    upstream: https://erp.internal/webhooks/partner
    methods: [POST]
    policies: [payload-size-limit]
    timeout_ms: 3000
```

Each route specifies:
- `id` — unique identifier, used in the URL path
- `path` — the full path Guard Rail listens on (always `/v1/execute/{id}`)
- `upstream` — the destination URL to forward allowed requests to
- `methods` — allowed HTTP methods
- `policies` — list of policy names to evaluate (must exist in policies dir)
- `timeout_ms` — forwarding timeout (overrides `forwarding.default_timeout_ms`)

### policies/*.yaml — Policy Rules

```yaml
policies:
  - name: block-external-callbacks
    description: Block payloads with callback URLs outside allowlist
    rules:
      - field: "$.callback"
        condition: domain_not_in
        values: ["*.guardrail.co.za", "*.internal.bank.za"]
        action: block
        severity: critical

  - name: pii-detection
    description: Block payloads containing South African ID numbers
    rules:
      - field: "$..*.value"
        condition: regex_match
        pattern: "\\b\\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\\d|3[01])\\d{4}[01]\\d{2}\\b"
        action: block
        severity: critical

  - name: payload-size-limit
    description: Reject oversized payloads
    rules:
      - field: "$"
        condition: size_exceeds
        max_bytes: 102400
        action: block
        severity: warning
```

Multiple YAML files can exist in the policies directory. All are loaded and merged by policy name.

## Policy Engine

### Field Matching

Uses JSONPath to extract values from the JSON payload. A single JSONPath expression can match multiple values (e.g., `$..*.value` matches all nested `value` fields). If any matched value triggers a condition, the rule fires.

### Conditions (v1)

| Condition | Parameters | Behavior |
|-----------|-----------|----------|
| `domain_not_in` | `values: [glob patterns]` | Extract URL from field, check domain against glob allowlist. Blocks if domain does NOT match any pattern. |
| `domain_in` | `values: [glob patterns]` | Blocks if domain DOES match any pattern (blocklist). |
| `regex_match` | `pattern: string` | Blocks if field value matches the regex. |
| `regex_not_match` | `pattern: string` | Blocks if field value does NOT match the regex. |
| `equals` | `value: any` | Blocks if field value equals the given value. |
| `not_equals` | `value: any` | Blocks if field value does NOT equal the given value. |
| `contains` | `value: string` | Blocks if field value contains the substring. |
| `not_contains` | `value: string` | Blocks if field value does NOT contain the substring. |
| `size_exceeds` | `max_bytes: number` | Applied to `$` (root). Blocks if raw payload byte size exceeds limit. |
| `field_exists` | (none) | Blocks if the JSONPath matches any value. |
| `field_not_exists` | (none) | Blocks if the JSONPath matches nothing. |

### Evaluation Logic

1. For a given route, gather all referenced policies by name.
2. For each policy, evaluate all rules. Rules within a policy are AND'd — all must pass for the policy to pass.
3. Across policies, any single BLOCK verdict blocks the request. The engine short-circuits on the first block — it does not continue evaluating remaining policies.
4. The engine returns either `ALLOW` (all policies passed) or `BLOCK` (with the violating policy name, rule field, and matched value).

### Action

v1 supports `block` only. Future stages add `transform` (PII masking, field stripping).

## Request Forwarding

### What Gets Forwarded (on ALLOW)

- Original request body, untouched
- Original headers, with modifications:
  - **Stripped:** `Authorization` header (Guard Rail's API key must not leak to upstream)
  - **Added:** `X-GuardRail-Execution-Id: GR-EXE-{uuid}`
  - **Added:** `X-GuardRail-Verdict: ALLOW`
  - **Preserved:** `Content-Type`, `Accept`, and all other original headers

### What Gets Returned to Caller

**On ALLOW + successful forward:**
- Upstream's response status code, headers, and body — passed through transparently
- Added response header: `X-GuardRail-Execution-Id`

**On BLOCK:**
- HTTP `403 Forbidden`
- JSON body:
```json
{
  "status": "blocked",
  "execution_id": "GR-EXE-a1b2c3d4",
  "policy": "block-external-callbacks",
  "rule_field": "$.callback",
  "message": "Domain not in allowlist"
}
```

**On upstream failure (timeout, connection refused, 5xx):**
- HTTP `502 Bad Gateway`
- JSON body with execution ID and error description
- Logged as execution with verdict `ALLOW`, forward status `FAILED`

## Logging

Every execution produces one structured JSON log line to stdout:

```json
{
  "execution_id": "GR-EXE-a1b2c3d4",
  "timestamp": "2026-04-04T14:32:01.004Z",
  "route_id": "transfer-api",
  "method": "POST",
  "source_ip": "41.13.22.91",
  "verdict": "BLOCKED",
  "policy": "block-external-callbacks",
  "rule_field": "$.callback",
  "violation_value": "https://evil.sh/exfil",
  "upstream": null,
  "upstream_status": null,
  "latency_inspect_us": 142,
  "latency_forward_ms": null,
  "latency_total_ms": 1
}
```

For ALLOWED + forwarded requests: `verdict` is `"ALLOWED"`, `upstream` is the URL, `upstream_status` is the HTTP status code, `latency_forward_ms` is the round-trip to upstream. Inspection latency is in microseconds to capture sub-millisecond performance.

Non-execution events (startup, config reload, errors) use standard `tracing` structured logs.

## Error Handling

| Condition | Response | Logged? |
|-----------|----------|---------|
| Body is not valid JSON | `400 Bad Request` | Yes, verdict `REJECTED` |
| Route not found | `404 Not Found` | No (no route = no Guard Rail involvement) |
| Method not allowed for route | `405 Method Not Allowed` | Yes, verdict `REJECTED` |
| Policy blocks request | `403 Forbidden` | Yes, verdict `BLOCKED` |
| Upstream timeout/connection error | `502 Bad Gateway` | Yes, verdict `ALLOWED`, forward `FAILED` |
| Upstream response exceeds size limit | `502 Bad Gateway` | Yes, verdict `ALLOWED`, forward `FAILED` |

### Hot-Reload Safety

- If a policy file has YAML syntax errors on reload: log warning, keep previous valid policy set. Never run with zero policies.
- If initial policy load fails at startup: server refuses to start.
- If routes reference a policy name that doesn't exist: reject the reload (or refuse to start on initial load). Caught at load time, never at request time.

## Testing Strategy

### Unit Tests

- **Policy engine conditions:** Every condition type gets test cases: valid input (should pass), triggering input (should block), edge cases (empty strings, null fields, missing fields, nested objects, arrays).
- **JSONPath extraction:** Verify correct values are extracted from complex nested payloads.
- **Route matching:** Path + method → correct route config, or 404/405.
- **Config loading:** Valid YAML parses correctly. Invalid YAML produces clear errors. Missing required fields caught.
- **Header manipulation:** Authorization stripped, X-GuardRail-* headers added correctly.

### Integration Tests

- Spin up Guard Rail with test config, routes, and policies using `axum::serve` in-process.
- Spin up a mock upstream HTTP server (also `axum`) that records received requests and returns configurable responses.
- Full request cycle tests:
  - ALLOW path: send valid payload → policies pass → upstream receives forwarded request → caller gets upstream response
  - BLOCK path: send payload that violates a policy → 403 returned → upstream never called
  - REJECT path: send malformed JSON → 400 returned
  - Upstream failure: mock upstream returns 500 or times out → 502 returned
  - Method not allowed: send GET to POST-only route → 405 returned

### No External Dependencies

All tests run with `cargo test`. No Docker, no Postgres, no external services. Mock upstream is in-process.

## Project Structure

```
guard-rail-engine/
  Cargo.toml
  src/
    main.rs              — entry point, CLI parsing, server startup
    config.rs            — config.yaml loading + env var overrides
    routes.rs            — routes.yaml loading, route matching
    policy/
      mod.rs             — policy loading, hot-reload, policy set management
      engine.rs          — evaluation logic (match policies to payload, return verdict)
      conditions.rs      — condition implementations (domain_check, regex, equals, etc.)
    proxy/
      mod.rs             — request handling, forwarding orchestration
      forward.rs         — upstream HTTP forwarding, header manipulation
      response.rs        — response construction (block, reject, bad gateway)
    logging.rs           — execution log struct, JSON serialization to stdout
    reload.rs            — file watcher setup, hot-reload coordination
  config/
    config.yaml          — example server config
    routes.yaml          — example routes
    policies/
      security.yaml      — example security policies
  tests/
    integration/
      mod.rs
      helpers.rs         — test server setup, mock upstream builder
      allow_test.rs      — full ALLOW path tests
      block_test.rs      — full BLOCK path tests
      error_test.rs      — 400, 404, 405, 502 tests
      reload_test.rs     — hot-reload behavior tests
```
