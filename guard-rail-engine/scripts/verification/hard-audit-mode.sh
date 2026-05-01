#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

ENGINE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
export PHASE7_SUITE_ID="${PHASE7_SUITE_ID:-$(phase7_timestamp)}"

scenario="hard-audit-mode"
test_target="required_audit_mode_does_not_forward_without_pre_forward_intent"
artifact_dir="$(cd "$(phase7_result_dir)" && pwd)"
log_file="${artifact_dir}/${scenario}.log"

: >"${log_file}"

pushd "${ENGINE_DIR}" >/dev/null
if cargo test --test smoke_test "${test_target}" -- --exact >"${log_file}" 2>&1; then
  exit_code=0
else
  exit_code=$?
fi
popd >/dev/null

if python3 - "${test_target}" "${log_file}" <<'PY'
import pathlib
import re
import sys

test_name = sys.argv[1]
output = pathlib.Path(sys.argv[2]).read_text()

checks = [
    f"running 1 test" in output,
    re.search(rf"test\s+{re.escape(test_name)}\s+\.\.\.\s+ok\b", output) is not None,
    "test result: ok. 1 passed;" in output,
]
sys.exit(0 if all(checks) else 1)
PY
then
  matched_expected_test=true
else
  matched_expected_test=false
fi

if [[ ${exit_code} -eq 0 && "${matched_expected_test}" == "true" ]]; then
  status="pass"
else
  status="fail"
fi

metrics="$(python3 - "${test_target}" "${exit_code}" "${log_file}" <<'PY'
import json
import pathlib
import re
import sys

test_name = sys.argv[1]
exit_code = int(sys.argv[2])
log_path = pathlib.Path(sys.argv[3])
output = log_path.read_text()

print(json.dumps({
    "command": f"cargo test --test smoke_test {test_name} -- --exact",
    "test_target": test_name,
    "exit_code": exit_code,
    "log_path": str(log_path),
    "running_1_test": "running 1 test" in output,
    "matched_test_line": re.search(rf"test\s+{re.escape(test_name)}\s+\.\.\.\s+ok\b", output) is not None,
    "matched_pass_summary": "test result: ok. 1 passed;" in output,
}))
PY
)"

phase7_write_result_json "${scenario}" "${status}" "${metrics}"
printf '%s: %s\n' "${scenario}" "${status^^}"
test "${status}" = "pass"
