# Policy Cookbook

Policies are YAML files loaded from `policies_dir`. Routes reference policies by name.

## Route Example

```yaml
routes:
  - id: pilot-webhook
    auth_mode: tenant_bound
    upstream: https://upstream.internal.example/webhook
    methods: [POST]
    policies: [callback-allowlist, sa-id-pii-block, payload-size-limit]
    timeout_ms: 3000
```

## Callback Allowlist

Use this when payloads include a callback URL and only approved callback domains should be allowed.

```yaml
policies:
  - name: callback-allowlist
    description: Block callback URLs outside approved domains
    rules:
      - field: "$.callback"
        condition: domain_not_in
        values: ["*.safe.example", "hooks.internal.example"]
        action: block
        severity: critical
```

Allowed payload:

```json
{
  "callback": "https://api.safe.example/hook",
  "value": "ship"
}
```

Blocked payload:

```json
{
  "callback": "https://evil.example/exfiltrate",
  "value": "ship"
}
```

Expected behavior: Guard Rail returns `403` before forwarding when the callback domain is outside the allowlist.

## South African ID / PII Block

Use this when workflow payloads contain free-form nested fields and South African ID numbers should not be forwarded.

```yaml
policies:
  - name: sa-id-pii-block
    description: Block South African ID numbers in nested value fields
    rules:
      - field: "$..*.value"
        condition: regex_match
        pattern: "\\b\\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\\d|3[01])\\d{4}[01]\\d{2}\\b"
        action: block
        severity: critical
```

Allowed payload:

```json
{
  "fields": [
    { "name": "reference", "value": "INV-2048" }
  ]
}
```

Blocked payload:

```json
{
  "fields": [
    { "name": "customer_id", "value": "8501015009087" }
  ]
}
```

Expected behavior: Guard Rail returns `403` when a nested `value` field matches the configured ID-number pattern.

## Payload Size Limit

Use this when a route should reject oversized payloads before upstream work starts.

```yaml
policies:
  - name: payload-size-limit
    description: Block payloads larger than 100 KiB
    rules:
      - field: "$"
        condition: size_exceeds
        max_bytes: 102400
        action: block
        severity: warning
```

Expected behavior: Guard Rail returns `403` when the raw request body exceeds `max_bytes`.

## Notes

- `domain_not_in` only evaluates string values that parse as URLs.
- `regex_match` only evaluates string values.
- `size_exceeds` uses raw request body size, not parsed JSON size.
- A missing JSONPath field does not trigger a block for `domain_not_in` or `regex_match`.
- Policies short-circuit on the first blocking rule.
