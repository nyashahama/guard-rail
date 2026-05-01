#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

ENGINE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
REPO_ROOT="$(cd "${ENGINE_DIR}/.." && pwd)"
export PHASE7_SUITE_ID="${PHASE7_SUITE_ID:-$(phase7_timestamp)}"
export PHASE7_RESULT_ROOT="${PHASE7_RESULT_ROOT:-${REPO_ROOT}/tmp/verification}"

scripts=(
  "load-allowed-and-blocked.sh"
  "reload-under-traffic.sh"
  "drain-under-load.sh"
  "db-degradation.sh"
  "upstream-degradation.sh"
  "hard-audit-mode.sh"
  "replay-redaction.sh"
  "dependency-audits.sh"
)

failures=0
for script in "${scripts[@]}"; do
  if ! "${SCRIPT_DIR}/${script}"; then
    failures=$((failures + 1))
  fi
done

printf 'Phase 7 result directory: %s\n' "$(phase7_result_dir)"
test "${failures}" -eq 0
