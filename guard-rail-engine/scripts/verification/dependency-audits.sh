#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"
ENGINE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
REPO_ROOT="$(cd "${ENGINE_DIR}/.." && pwd)"
REQUIRE_DEPENDENCY_AUDIT="${REQUIRE_DEPENDENCY_AUDIT:-false}"
export PHASE7_SUITE_ID="${PHASE7_SUITE_ID:-$(phase7_timestamp)}"

rust_status="skipped"
node_status="skipped"

# Rust audit
if command -v cargo-audit >/dev/null 2>&1; then
  pushd "${ENGINE_DIR}" >/dev/null
  if cargo audit; then
    rust_status="pass"
  else
    rust_status="fail"
  fi
  popd >/dev/null
else
  echo "cargo-audit is required but not installed" >&2
  rust_status="missing"
fi

# Node audit
if [[ -f "${REPO_ROOT}/package-lock.json" ]]; then
  if command -v npm >/dev/null 2>&1; then
    pushd "${REPO_ROOT}" >/dev/null
    if npm audit --audit-level=high; then
      node_status="pass"
    else
      node_status="fail"
    fi
    popd >/dev/null
  else
    echo "npm is required for Node dependency audit but not installed" >&2
    node_status="missing"
  fi
else
  echo "package-lock.json not found; skipping npm audit" >&2
  node_status="skipped"
fi

metrics=$(python3 -c "
import json
print(json.dumps({
    'rust': '${rust_status}',
    'node': '${node_status}'
}))
")

if [[ "${rust_status}" == "fail" || "${node_status}" == "fail" ]]; then
  phase7_write_result_json "dependency-audits" "fail" "${metrics}"
  echo "dependency-audits: FAIL"
  exit 1
elif [[ "${rust_status}" == "pass" && "${node_status}" == "pass" ]]; then
  phase7_write_result_json "dependency-audits" "pass" "${metrics}"
  echo "dependency-audits: PASS"
  exit 0
elif [[ "${REQUIRE_DEPENDENCY_AUDIT}" == "true" ]]; then
  phase7_write_result_json "dependency-audits" "fail" "${metrics}"
  echo "dependency-audits: FAIL (missing required audit prerequisites)"
  exit 1
else
  phase7_write_result_json "dependency-audits" "skipped" "${metrics}"
  echo "dependency-audits: SKIPPED (missing prerequisites)"
  exit 0
fi
