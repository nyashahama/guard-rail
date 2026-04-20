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

# Start a mock upstream
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

cat > "${routes_file}" <<EOF
routes:
  - id: open-route
    auth_mode: public
    upstream: http://127.0.0.1:${upstream_port}/api/open
    methods: [POST]
    policies: []
    timeout_ms: 5000
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
  # Always restart the DB container if we stopped it
  local pg_container="${PHASE7_PG_CONTAINER:-guardrail-phase7-pg}"
  if docker ps -a --format '{{.Names}}' | grep -q "^${pg_container}$"; then
    docker start "${pg_container}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

start_upstream

cargo run --quiet -- migrate --config "${config_file}" >/dev/null
cargo run --quiet -- serve --config "${config_file}" >"${log_file}" 2>&1 &
server_pid=$!

wait_for_health "${base_url}/health"
wait_for_ready "${base_url}/ready"

# Probe while DB is healthy
healthy_start=$(date +%s%N)
healthy_code=$(curl --silent --output /dev/null --write-out "%{http_code}" "${base_url}/ready")
healthy_end=$(date +%s%N)
healthy_ms=$(( (healthy_end - healthy_start) / 1000000 ))

pg_container="${PHASE7_PG_CONTAINER:-guardrail-phase7-pg}"

# Stop the DB container
docker stop "${pg_container}" >/dev/null
sleep 3

# Probe while DB is unavailable
degraded_start=$(date +%s%N)
degraded_code=$(curl --silent --output /dev/null --write-out "%{http_code}" "${base_url}/ready")
degraded_end=$(date +%s%N)
degraded_ms=$(( (degraded_end - degraded_start) / 1000000 ))

# Restart the DB container for other tests
docker start "${pg_container}" >/dev/null
sleep 2

test "${healthy_code}" = "200"
test "${degraded_code}" = "503"

metrics=$(python3 -c "
import json
print(json.dumps({
    'healthy_code': ${healthy_code},
    'healthy_probe_ms': ${healthy_ms},
    'degraded_code': ${degraded_code},
    'degraded_probe_ms': ${degraded_ms}
}))
")

phase7_write_result_json "db-degradation" "pass" "${metrics}"
echo "db-degradation: PASS"
