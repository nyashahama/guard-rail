#!/usr/bin/env bash
set -euo pipefail

phase7_require_env() {
  local name="$1"
  : "${!name:?${name} must be set}"
}

phase7_timestamp() {
  date -u +"%Y%m%dT%H%M%SZ"
}

phase7_result_root() {
  local root="${PHASE7_RESULT_ROOT:-./tmp/verification}"
  mkdir -p "${root}"
  printf '%s\n' "${root}"
}

phase7_result_dir() {
  local suite_id="${PHASE7_SUITE_ID:?PHASE7_SUITE_ID must be set}"
  local dir
  dir="$(phase7_result_root)/${suite_id}"
  mkdir -p "${dir}"
  printf '%s\n' "${dir}"
}

phase7_write_result_json() {
  local scenario="$1"
  local status="$2"
  local metrics_json="$3"
  local out
  out="$(phase7_result_dir)/${scenario}.json"
  printf '{\n  "scenario": "%s",\n  "status": "%s",\n  "timestamp": "%s",\n  "metrics": %s\n}\n' \
    "${scenario}" "${status}" "$(phase7_timestamp)" "${metrics_json}" > "${out}"
}

wait_for_health() {
  local url="$1"
  local max_attempts="${2:-30}"
  local attempt=0
  while [[ ${attempt} -lt ${max_attempts} ]]; do
    if curl --fail --silent --show-error "${url}" >/dev/null 2>&1; then
      return 0
    fi
    attempt=$((attempt + 1))
    sleep 1
  done
  echo "Health endpoint ${url} did not become healthy within ${max_attempts}s" >&2
  return 1
}

wait_for_ready() {
  local url="$1"
  local max_attempts="${2:-30}"
  local attempt=0
  while [[ ${attempt} -lt ${max_attempts} ]]; do
    local code
    code=$(curl --silent --output /dev/null --write-out "%{http_code}" "${url}" 2>/dev/null || true)
    if [[ "${code}" == "200" ]]; then
      return 0
    fi
    attempt=$((attempt + 1))
    sleep 1
  done
  echo "Ready endpoint ${url} did not become ready within ${max_attempts}s" >&2
  return 1
}
