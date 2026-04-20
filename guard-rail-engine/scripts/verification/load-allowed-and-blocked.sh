#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

phase7_require_env GUARDRAIL_DATABASE__URL
phase7_require_env GUARDRAIL_ADMIN__TOKEN

host="${GUARDRAIL_SERVER__HOST:-127.0.0.1}"
port="${GUARDRAIL_SERVER__PORT:-18080}"
base_url="http://${host}:${port}"

# Create temp config directory
tmp_dir="$(mktemp -d)"
config_file="${tmp_dir}/config.yaml"
routes_file="${tmp_dir}/routes.yaml"
policies_dir="${tmp_dir}/policies"
mkdir -p "${policies_dir}"

# Start a mock upstream server
upstream_port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("", 0)); print(s.getsockname()[1]); s.close()')"
upstream_pid=""

start_upstream() {
  python3 -c "
import http.server, socketserver, threading
class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(b'{\"ok\":true}')
    def log_message(self, format, *args):
        pass
server = socketserver.TCPServer(('127.0.0.1', ${upstream_port}), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
import time; time.sleep(3600)
" &
  upstream_pid=$!
  # Wait for upstream to be ready
  for _ in $(seq 1 10); do
    if curl --silent --output /dev/null "http://127.0.0.1:${upstream_port}/"; then
      break
    fi
    sleep 0.2
  done
}

# Write temp config
cat > "${config_file}" <<EOF
environment: development
server:
  host: "${host}"
  port: ${port}
  request_body_limit_bytes: 1048576
routes_file: "${routes_file}"
policies_dir: "${policies_dir}"
forwarding:
  default_timeout_ms: 5000
  user_agent: "GuardRail/0.1.0"
logging:
  level: "info"
  format: "json"
database:
  url: "${GUARDRAIL_DATABASE__URL}"
  max_connections: 10
audit:
  write_timeout_ms: 250
admin:
  token: "${GUARDRAIL_ADMIN__TOKEN}"
rate_limit:
  requests_per_minute: 120
  burst: 30
replay:
  enabled: false
observability:
  service_name: "guard-rail-engine"
  metrics_enabled: true
  metrics_path: "/metrics"
  trace_header_name: "traceparent"
  readiness_probe_timeout_ms: 250
shutdown:
  grace_period_ms: 15000
  drain_poll_interval_ms: 50
EOF

# Write routes with open-route and blocked-route
cat > "${routes_file}" <<EOF
routes:
  - id: open-route
    auth_mode: public
    upstream: http://127.0.0.1:${upstream_port}/api/open
    methods: [POST]
    policies: []
    timeout_ms: 5000

  - id: blocked-route
    auth_mode: public
    upstream: http://127.0.0.1:${upstream_port}/api/blocked
    methods: [POST]
    policies: [pii-detection]
    timeout_ms: 5000
EOF

# Write blocking policy
cat > "${policies_dir}/security.yaml" <<'EOF'
policies:
  - name: pii-detection
    description: Block South African ID numbers
    rules:
      - field: "$..value"
        condition: regex_match
        pattern: "\\b\\d{2}(0[1-9]|1[0-2])\\d{6}\\b"
        action: block
        severity: critical
EOF

server_pid=""
log_file="$(mktemp)"

cleanup() {
  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" 2>/dev/null; then
    kill "${server_pid}" 2>/dev/null || true
    wait "${server_pid}" 2>/dev/null || true
  fi
  if [[ -n "${upstream_pid}" ]] && kill -0 "${upstream_pid}" 2>/dev/null; then
    kill "${upstream_pid}" 2>/dev/null || true
    wait "${upstream_pid}" 2>/dev/null || true
  fi
  rm -rf "${tmp_dir}"
  rm -f "${log_file}"
}
trap cleanup EXIT

start_upstream

cargo run --quiet -- migrate --config "${config_file}" >/dev/null
cargo run --quiet -- serve --config "${config_file}" >"${log_file}" 2>&1 &
server_pid=$!

wait_for_health "${base_url}/health"
wait_for_ready "${base_url}/ready"

allowed_latencies=()
blocked_latencies=()
allowed_count=0
blocked_count=0

# Allowed path loop
for _ in $(seq 1 25); do
  start_ns=$(date +%s%N)
  code=$(curl --silent --output /dev/null --write-out "%{http_code}" \
    -X POST \
    -H 'content-type: application/json' \
    --data '{"message":"ok"}' \
    "${base_url}/v1/execute/open-route")
  end_ns=$(date +%s%N)
  test "${code}" = "200"
  allowed_latencies+=( $(( (end_ns - start_ns) / 1000000 )) )
  allowed_count=$((allowed_count + 1))
done

# Blocked path loop - SA ID number triggers pii-detection
for _ in $(seq 1 25); do
  start_ns=$(date +%s%N)
  code=$(curl --silent --output /dev/null --write-out "%{http_code}" \
    -X POST \
    -H 'content-type: application/json' \
    --data '{"value":"9001015009"}' \
    "${base_url}/v1/execute/blocked-route")
  end_ns=$(date +%s%N)
  test "${code}" = "403"
  blocked_latencies+=( $(( (end_ns - start_ns) / 1000000 )) )
  blocked_count=$((blocked_count + 1))
done

# Format arrays as comma-separated strings for Python
allowed_csv="$(printf '%s,' "${allowed_latencies[@]}" | sed 's/,$//')"
blocked_csv="$(printf '%s,' "${blocked_latencies[@]}" | sed 's/,$//')"

# Compute percentiles using Python
percentiles=$(python3 -c "
import json
allowed = sorted([${allowed_csv}])
blocked = sorted([${blocked_csv}])
def pct(arr, p):
    if not arr:
        return 0
    k = (len(arr) - 1) * p / 100.0
    f = int(k)
    c = f + 1 if f + 1 < len(arr) else f
    if f == c:
        return arr[f]
    return arr[f] + (k - f) * (arr[c] - arr[f])
print(json.dumps({
    'allowed': {'count': len(allowed), 'p50': pct(allowed, 50), 'p95': pct(allowed, 95), 'p99': pct(allowed, 99)},
    'blocked': {'count': len(blocked), 'p50': pct(blocked, 50), 'p95': pct(blocked, 95), 'p99': pct(blocked, 99)}
}))
")

phase7_write_result_json "load-allowed-and-blocked" "pass" "${percentiles}"
echo "load-allowed-and-blocked: PASS"
