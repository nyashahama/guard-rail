# Policy Cookbook (Pilot)

Policy files are loaded from the configured `policies_dir` and hot-reloaded when valid YAML changes land.

Each file uses:

```yaml
policies:
  - name: <string>
    description: <optional string>
    rules:
      - field: <jsonpath>
        condition: <condition_name>
        ... condition values ...
        action: block
        severity: <string>
```

## Field format

`field` must be a JSONPath expression. `$.callback`, `$.customer.id_number`, and `$` are supported patterns in this pilot.

`$.` is a special-case rule for size checks on the full payload.

## Condition list

- `domain_not_in` (with `values`)
- `domain_in` (with `values`)
- `regex_match` / `regex_not_match` (with `pattern`)
- `size_exceeds` (with `max_bytes`)
- `equals` / `not_equals` (with `value`)
- `contains` / `not_contains` (with `value`)
- `field_exists` / `field_not_exists`

Rules evaluate in order, and the first triggered condition blocks the request.

## 1) callback-allowlist.yaml

Block callbacks that are not in your approved domain list.

```yaml
policies:
  - name: callback-allowlist
    description: Block callback URLs outside the partner allowlist.
    rules:
      - field: "$.callback"
        condition: domain_not_in
        values:
          - "*.safe.com"
          - "*.internal.bank.za"
        action: block
        severity: critical
```

Suggested use:
- outbound callback URLs in request payloads
- webhooks that can be influenced by downstream clients

Expected block message shape:

```json
{"status":"blocked","policy":"callback-allowlist",...}
```

## 2) sa-id-pii-block.yaml

Block obvious South African ID-number-like values.

```yaml
policies:
  - name: sa-id-pii-block
    description: Block common South African ID value patterns.
    rules:
      - field: "$.id_number"
        condition: regex_match
        pattern: "\\b\\d{2}(0[1-9]|1[0-2])\\d{6}\\b"
        action: block
        severity: critical
      - field: "$.customer.id_number"
        condition: regex_match
        pattern: "\\b\\d{2}(0[1-9]|1[0-2])\\d{6}\\b"
        action: block
        severity: critical
```

Suggested use:
- customer verification payloads
- partner payloads that should never include raw ID fields

## 3) payload-size-limit.yaml

Stop oversized payloads before they reach your upstream.

```yaml
policies:
  - name: payload-size-limit
    description: Block requests above negotiated payload size.
    rules:
      - field: "$"
        condition: size_exceeds
        max_bytes: 102400
        action: block
        severity: warning
```

Suggested use:
- reduce upstream parsing failures
- protect webhook receivers from oversized payload abuse

## Practical tips

- Keep policy names short and route-specific when possible.
- Put stricter, low-cost checks first.
- For trial runs, start with one policy per route.
- If you need to ship updates quickly, edit files and wait for logs: policy reload should be accepted/rejected at runtime.
- Use `payload-size-limit` for traffic-shaping and `callback-allowlist` for endpoint safety during pilot.

## Pilot scope note

This cookbook covers checks that are implemented and verifiable in this pilot today.
It does not promise a visual policy editor or enterprise policy DSL outside this file format.
