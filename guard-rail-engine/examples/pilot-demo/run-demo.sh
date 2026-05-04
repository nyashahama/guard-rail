#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

command -v cargo >/dev/null || { echo "cargo is required" >&2; exit 1; }
command -v curl >/dev/null || { echo "curl is required" >&2; exit 1; }
command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 1; }

export GUARDRAIL_DATABASE__URL="${GUARDRAIL_DATABASE__URL:-postgres://guardrail:secret@127.0.0.1:5432/guardrail}"
export GUARDRAIL_ADMIN__TOKEN="${GUARDRAIL_ADMIN__TOKEN:-demo-admin-token}"
export GUARDRAIL_ENVIRONMENT=development

TMP_DIR="${GUARDRAIL_DEMO_TMP_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/guardrail-pilot-demo.XXXXXX")}"
mkdir -p "$TMP_DIR"

cleanup() {
  if [[ -n "${ENGINE_PID:-}" ]]; then kill "$ENGINE_PID" 2>/dev/null || true; fi
  if [[ -n "${UPSTREAM_PID:-}" ]]; then kill "$UPSTREAM_PID" 2>/dev/null || true; fi
}

on_exit() {
  local status=$?
  if [[ "$status" -ne 0 ]]; then
    echo "pilot demo failed with status ${status}" >&2
    if [[ -f "$TMP_DIR/engine.log" ]]; then
      echo "--- engine.log ---" >&2
      tail -n 120 "$TMP_DIR/engine.log" >&2
    fi
    if [[ -f "$TMP_DIR/upstream.log" ]]; then
      echo "--- upstream.log ---" >&2
      tail -n 120 "$TMP_DIR/upstream.log" >&2
    fi
  fi
  cleanup
}
trap on_exit EXIT

python3 examples/pilot-demo/upstream.py > "$TMP_DIR/upstream.log" 2>&1 &
UPSTREAM_PID=$!

cargo run -- migrate --config ./examples/pilot-demo/config.yaml
cargo run -- serve --config ./examples/pilot-demo/config.yaml > "$TMP_DIR/engine.log" 2>&1 &
ENGINE_PID=$!

ready=0
for _ in $(seq 1 40); do
  if curl -fsS http://127.0.0.1:18080/ready >/dev/null 2>&1; then
    ready=1
    break
  fi
  if ! kill -0 "$ENGINE_PID" 2>/dev/null; then
    echo "guard-rail engine exited before becoming ready" >&2
    exit 1
  fi
  sleep 0.25
done

if [[ "$ready" -ne 1 ]]; then
  echo "guard-rail engine did not become ready within 10s" >&2
  exit 1
fi

DEMO_TENANT_NAME="pilot-demo-$(date -u +%Y%m%dT%H%M%SZ)"

TENANT_ID=$(
  curl -sS -X POST http://127.0.0.1:18081/v1/admin/tenants \
    -H "authorization: Bearer ${GUARDRAIL_ADMIN__TOKEN}" \
    -H "content-type: application/json" \
    -d "{\"name\":\"${DEMO_TENANT_NAME}\"}" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])'
)

TENANT_KEY=$(
  curl -sS -X POST "http://127.0.0.1:18081/v1/admin/tenants/${TENANT_ID}/keys" \
    -H "authorization: Bearer ${GUARDRAIL_ADMIN__TOKEN}" \
    -H "content-type: application/json" \
    -d '{"name":"primary"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["raw_key"])'
)

curl -fsS -X POST "http://127.0.0.1:18081/v1/admin/tenants/${TENANT_ID}/routes" \
  -H "authorization: Bearer ${GUARDRAIL_ADMIN__TOKEN}" \
  -H "content-type: application/json" \
  -d '{"route_id":"pilot-webhook"}' >/dev/null

ALLOWED_STATUS=$(
  curl -sS -D "$TMP_DIR/allowed.headers" \
    -o "$TMP_DIR/allowed.response.json" \
    -w "%{http_code}" \
    -X POST http://127.0.0.1:18080/v1/execute/pilot-webhook \
    -H "authorization: Bearer ${TENANT_KEY}" \
    -H "content-type: application/json" \
    -d @examples/pilot-demo/payloads/allowed.json
)

test "$ALLOWED_STATUS" = "200"

ALLOWED_ID=$(awk 'tolower($1)=="x-guardrail-execution-id:" {print $2}' "$TMP_DIR/allowed.headers" | tr -d '\r')
test -n "$ALLOWED_ID"

BLOCKED_STATUS=$(
  curl -sS -o "$TMP_DIR/blocked.response.json" -w "%{http_code}" \
    -X POST http://127.0.0.1:18080/v1/execute/pilot-webhook \
    -H "authorization: Bearer ${TENANT_KEY}" \
    -H "content-type: application/json" \
    -d @examples/pilot-demo/payloads/blocked-callback.json
)

test "$BLOCKED_STATUS" = "403"

curl -fsS "http://127.0.0.1:18080/v1/audit/executions/${ALLOWED_ID}" \
  -H "authorization: Bearer ${TENANT_KEY}" \
  | python3 -m json.tool > "$TMP_DIR/audit.json"

curl -fsS -X POST "http://127.0.0.1:18080/v1/replay/executions/${ALLOWED_ID}" \
  -H "authorization: Bearer ${TENANT_KEY}" \
  -H "content-type: application/json" \
  -d '{"policy_source":"snapshot"}' \
  | python3 -m json.tool > "$TMP_DIR/replay.json"

ALLOWED_UPSTREAM_COUNT=$(grep -c '"callback": "https://api.safe.example/hooks/invoice"' "$TMP_DIR/upstream.log" || true)
BLOCKED_UPSTREAM_COUNT=$(grep -c '"callback": "https://evil.example/exfiltrate"' "$TMP_DIR/upstream.log" || true)
test "$ALLOWED_UPSTREAM_COUNT" = "1"
test "$BLOCKED_UPSTREAM_COUNT" = "0"

echo "Allowed execution: ${ALLOWED_ID}"
echo "Blocked callback status: ${BLOCKED_STATUS}"
echo "Audit evidence: ${TMP_DIR}/audit.json"
echo "Replay output: ${TMP_DIR}/replay.json"
echo "Upstream log: ${TMP_DIR}/upstream.log"
