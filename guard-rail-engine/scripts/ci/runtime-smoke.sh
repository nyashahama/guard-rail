#!/usr/bin/env bash
set -euo pipefail

: "${GUARDRAIL_DATABASE__URL:?GUARDRAIL_DATABASE__URL must be set}"
: "${GUARDRAIL_ADMIN__TOKEN:?GUARDRAIL_ADMIN__TOKEN must be set}"

export GUARDRAIL_ENVIRONMENT="${GUARDRAIL_ENVIRONMENT:-development}"
export GUARDRAIL_SERVER__HOST="${GUARDRAIL_SERVER__HOST:-127.0.0.1}"
export GUARDRAIL_SERVER__PORT="${GUARDRAIL_SERVER__PORT:-18080}"

log_file="$(mktemp)"
server_pid=""

cleanup() {
  status=$?

  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" 2>/dev/null; then
    kill "${server_pid}" 2>/dev/null || true
    wait "${server_pid}" 2>/dev/null || true
  fi

  if [[ ${status} -ne 0 ]]; then
    cat "${log_file}"
  fi

  rm -f "${log_file}"
  exit "${status}"
}

trap cleanup EXIT

cargo run -- migrate --config ./config/config.yaml

cargo run -- serve --config ./config/config.yaml >"${log_file}" 2>&1 &
server_pid=$!

health_url="http://${GUARDRAIL_SERVER__HOST}:${GUARDRAIL_SERVER__PORT}/health"
ready_url="http://${GUARDRAIL_SERVER__HOST}:${GUARDRAIL_SERVER__PORT}/ready"

for _ in $(seq 1 30); do
  if curl --fail --silent --show-error "${health_url}" >/dev/null; then
    break
  fi

  if ! kill -0 "${server_pid}" 2>/dev/null; then
    echo "guard-rail-engine exited before becoming healthy" >&2
    exit 1
  fi

  sleep 1
done

curl --fail --silent --show-error "${health_url}" >/dev/null
curl --fail --silent --show-error "${ready_url}" >/dev/null