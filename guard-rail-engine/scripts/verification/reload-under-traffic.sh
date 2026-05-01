#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"
ENGINE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

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

# Initial routes
cat > "${routes_file}" <<EOF
routes:
  - id: open-route
    auth_mode: public
    upstream: http://127.0.0.1:${upstream_port}/api/open
    methods: [POST]
    policies: []
    timeout_ms: 5000
EOF

# Empty policies
write_policies() {
  cat > "${policies_dir}/empty.yaml" <<'EOF'
policies: []
EOF
}
write_policies

server_pid=""
log_file="$(mktemp)"
bg_traffic_log="$(mktemp)"

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
  rm -f "${log_file}" "${bg_traffic_log}"
}
trap cleanup EXIT

start_upstream

pushd "${ENGINE_DIR}" >/dev/null
cargo run --quiet -- migrate --config "${config_file}" >/dev/null
cargo run --quiet -- serve --config "${config_file}" >"${log_file}" 2>&1 &
server_pid=$!
popd >/dev/null

wait_for_health "${base_url}/health"
wait_for_ready "${base_url}/ready"

# Start background traffic in a subshell
background_requests=0
background_successes=0
(
  while true; do
    if curl --silent --output /dev/null --write-out "%{http_code}" \
      -X POST -H 'content-type: application/json' \
      --data '{"msg":"bg"}' "${base_url}/v1/execute/open-route" | grep -q '^200$'; then
      echo "success" >> "${bg_traffic_log}"
    else
      echo "fail" >> "${bg_traffic_log}"
    fi
    sleep 0.1
  done
) &
bg_pid=$!

# Let background traffic run for a moment
sleep 1

# Capture baseline
baseline_count=$(wc -l < "${bg_traffic_log}" | tr -d ' ')
baseline_successes=$(grep -c "^success$" "${bg_traffic_log}" || true)

# VALID RELOAD: add a new route
cat > "${routes_file}" <<EOF
routes:
  - id: open-route
    auth_mode: public
    upstream: http://127.0.0.1:${upstream_port}/api/open
    methods: [POST]
    policies: []
    timeout_ms: 5000

  - id: new-route
    auth_mode: public
    upstream: http://127.0.0.1:${upstream_port}/api/new
    methods: [POST]
    policies: []
    timeout_ms: 5000
EOF

# Wait for file watcher to pick up the change
sleep 2

# Verify new route works
new_route_code=$(curl --silent --output /dev/null --write-out "%{http_code}" \
  -X POST -H 'content-type: application/json' \
  --data '{"msg":"new"}' "${base_url}/v1/execute/new-route")
test "${new_route_code}" = "200"

# Capture post-valid-reload
valid_count=$(wc -l < "${bg_traffic_log}" | tr -d ' ')
valid_successes=$(grep -c "^success$" "${bg_traffic_log}" || true)

# INVALID RELOAD: reference a missing policy
cat > "${routes_file}" <<EOF
routes:
  - id: open-route
    auth_mode: public
    upstream: http://127.0.0.1:${upstream_port}/api/open
    methods: [POST]
    policies: [missing-policy]
    timeout_ms: 5000
EOF

# Wait for file watcher
sleep 2

# Verify old route still works (rejected reload preserved last good config)
old_route_code=$(curl --silent --output /dev/null --write-out "%{http_code}" \
  -X POST -H 'content-type: application/json' \
  --data '{"msg":"old"}' "${base_url}/v1/execute/open-route")
test "${old_route_code}" = "200"

# Verify new route still works (from valid reload)
new_route_code2=$(curl --silent --output /dev/null --write-out "%{http_code}" \
  -X POST -H 'content-type: application/json' \
  --data '{"msg":"new2"}' "${base_url}/v1/execute/new-route")
test "${new_route_code2}" = "200"

# Capture post-invalid-reload
invalid_count=$(wc -l < "${bg_traffic_log}" | tr -d ' ')
invalid_successes=$(grep -c "^success$" "${bg_traffic_log}" || true)

# Stop background traffic
kill "${bg_pid}" 2>/dev/null || true
wait "${bg_pid}" 2>/dev/null || true

metrics=$(python3 -c "
import json
print(json.dumps({
    'total_background_requests': ${invalid_count},
    'total_background_successes': ${invalid_successes},
    'valid_reload_new_route_works': True,
    'invalid_reload_preserved_config': True,
    'baseline_requests': ${baseline_count},
    'post_valid_requests': ${valid_count},
    'post_invalid_requests': ${invalid_count}
}))
")

phase7_write_result_json "reload-under-traffic" "pass" "${metrics}"
echo "reload-under-traffic: PASS"
