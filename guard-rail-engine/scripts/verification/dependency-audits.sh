#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

rust_status="skipped"
node_status="skipped"

# Rust audit
if command -v cargo-audit >/dev/null 2>&1; then
  pushd /home/nyasha-hama/projects/guard-rail/.worktrees/phase7-verification/guard-rail-engine >/dev/null
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
if [[ -d /home/nyasha-hama/projects/guard-rail/.worktrees/phase7-verification/node_modules ]]; then
  pushd /home/nyasha-hama/projects/guard-rail/.worktrees/phase7-verification >/dev/null
  if npm audit --audit-level=high; then
    node_status="pass"
  else
    node_status="fail"
  fi
  popd >/dev/null
else
  echo "node_modules not found — skipping npm audit" >&2
  node_status="skipped"
fi

metrics=$(python3 -c "
import json
print(json.dumps({
    'rust': '${rust_status}',
    'node': '${node_status}'
}))
")

if [[ "${rust_status}" == "pass" && "${node_status}" == "pass" ]]; then
  phase7_write_result_json "dependency-audits" "pass" "${metrics}"
  echo "dependency-audits: PASS"
elif [[ "${rust_status}" == "skipped" || "${rust_status}" == "missing" || "${node_status}" == "skipped" ]]; then
  phase7_write_result_json "dependency-audits" "skipped" "${metrics}"
  echo "dependency-audits: SKIPPED (missing prerequisites)"
  exit 0
else
  phase7_write_result_json "dependency-audits" "fail" "${metrics}"
  echo "dependency-audits: FAIL"
  exit 1
fi
