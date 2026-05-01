#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

ENGINE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
export PHASE7_SUITE_ID="${PHASE7_SUITE_ID:-$(phase7_timestamp)}"

scenario="hard-audit-mode"
test_target="required_audit_mode_does_not_forward_without_pre_forward_intent"
log_file="$(mktemp)"

cleanup() {
  rm -f "${log_file}"
}
trap cleanup EXIT

pushd "${ENGINE_DIR}" >/dev/null
if cargo test --test smoke_test "${test_target}" -- --exact >"${log_file}" 2>&1; then
  status="pass"
  exit_code=0
else
  status="fail"
  exit_code=$?
fi
popd >/dev/null

metrics="$(python3 - "${test_target}" "${exit_code}" "${log_file}" <<'PY'
import json
import pathlib
import sys

test_name = sys.argv[1]
exit_code = int(sys.argv[2])
log_path = pathlib.Path(sys.argv[3])
output = log_path.read_text()

print(json.dumps({
    "command": f"cargo test --test smoke_test {test_name} -- --exact",
    "test_target": test_name,
    "exit_code": exit_code,
    "matched_pass_lines": output.count("test result: ok"),
}))
PY
)"

phase7_write_result_json "${scenario}" "${status}" "${metrics}"
printf '%s: %s\n' "${scenario}" "${status^^}"
test "${status}" = "pass"
