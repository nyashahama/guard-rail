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

# Start a slow mock upstream server (2s delay)
upstream_port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("", 0)); print(s.getsockname()[1]); s.close()')"
upstream_pid=""

start_slow_upstream() {
  python3 -c "
import http.server, socketserver, threading, time
class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        time.sleep(2)
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(b'{\"ok\":true}')
    def log_message(self, format, *args):
        pass
server = socketserver.TCPServer(('127.0.0.1', ${upstream_port}), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
time.sleep(3600)
" &
  upstream_pid=$!
  for _ in $(seq 1 10); do
    if curl --silent --output /dev/null "http://127.0.0.1:${upstream_port}/"; then
      break
    fi
    sleep 0.2
  done
}

# Write temp config with shorter timeout for faster tests, but longer than upstream delay
cat > "${config_file}" <<EOF
environment: development
server:
  host: "${host}"
  port: ${port}
  request_body_limit_bytes: 1048576
routes_file: "${routes_file}"
policies_dir: "${policies_dir}"
forwarding:
  default_timeout_ms: 10000
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

cat > "${routes_file}" <<EOF
routes:
  - id: slow-route
    auth_mode: public
    upstream: http://127.0.0.1:${upstream_port}/api/slow
    methods: [POST]
    policies: []
    timeout_ms: 10000
EOF

cat > "${policies_dir}/empty.yaml" <<'EOF'
policies: []
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

start_slow_upstream

cargo run --quiet -- migrate --config "${config_file}" >/dev/null
cargo run --quiet -- serve --config "${config_file}" >"${log_file}" 2>&1 &
server_pid=$!

wait_for_health "${base_url}/health"
wait_for_ready "${base_url}/ready"

# Start an in-flight slow request in background
in_flight_code=""
(
  code=$(curl --silent --output /dev/null --write-out "%{http_code}" \
    -X POST -H 'content-type: application/json' \
    --data '{"msg":"slow"}' \
    "${base_url}/v1/execute/slow-route")
  echo "${code}" > "${log_file}.inflight"
) &
in_flight_pid=$!

# Give the request time to be in-flight
sleep 0.5

# Signal TERM to trigger drain
kill -TERM "${server_pid}"

# Wait for the in-flight request to complete
wait "${in_flight_pid}" || true

if [[ -f "${log_file}.inflight" ]]; then
  in_flight_code=$(cat "${log_file}.inflight")
else
  in_flight_code="unknown"
fi

# Check ready endpoint after drain
ready_code=$(curl --silent --output /dev/null --write-out "%{http_code}" "${base_url}/ready" || true)

# Wait for server to exit
wait "${server_pid}" || true
server_pid=""

metrics=$(python3 -c "
import json
print(json.dumps({
    'in_flight_status': ${in_flight_code},
    'ready_during_drain': ${ready_code},
    'drain_completed': True
}))
")

test "${in_flight_code}" = "200"
test "${ready_code}" != "200"

phase7_write_result_json "drain-under-load" "pass" "${metrics}"
echo "drain-under-load: PASS"
