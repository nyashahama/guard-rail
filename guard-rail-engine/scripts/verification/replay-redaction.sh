#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

ENGINE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
REPO_ROOT="$(cd "${ENGINE_DIR}/.." && pwd)"
export PHASE7_SUITE_ID="${PHASE7_SUITE_ID:-$(phase7_timestamp)}"
export PHASE7_RESULT_ROOT="${PHASE7_RESULT_ROOT:-${REPO_ROOT}/tmp/verification}"
database_url_source="missing"
if [[ -n "${TEST_DATABASE_URL:-}" ]]; then
  database_url_source="TEST_DATABASE_URL"
fi
if [[ -z "${TEST_DATABASE_URL:-}" && -n "${GUARDRAIL_DATABASE__URL:-}" ]]; then
  export TEST_DATABASE_URL="${GUARDRAIL_DATABASE__URL}"
  database_url_source="GUARDRAIL_DATABASE__URL"
fi

scenario="replay-redaction"
artifact_dir="$(cd "$(phase7_result_dir)" && pwd)"
log_file="${artifact_dir}/${scenario}.log"
tests=(
  "test_replay_artifacts_redact_sensitive_request_json_before_persistence"
  "test_replay_artifacts_redact_sensitive_response_json_before_persistence"
)

pushd "${ENGINE_DIR}" >/dev/null
set +e
>"${log_file}"
for test_target in "${tests[@]}"; do
  cargo test --test replay_integration_test "${test_target}" -- --exact >>"${log_file}" 2>&1
  cmd_status=$?
  if [[ ${cmd_status} -ne 0 ]]; then
    exit_code=${cmd_status}
    status="fail"
    break
  fi
done
set -e
popd >/dev/null

if python3 - "${log_file}" "${tests[@]}" <<'PY'
import pathlib
import re
import sys

log_path = pathlib.Path(sys.argv[1])
tests = sys.argv[2:]
output = log_path.read_text()

if output.count("running 1 test") != len(tests):
    sys.exit(1)

for test_name in tests:
    if re.search(rf"test\s+{re.escape(test_name)}\s+\.\.\.\s+ok\b", output) is None:
        sys.exit(1)

if output.count("test result: ok. 1 passed;") != len(tests):
    sys.exit(1)

sys.exit(0)
PY
then
  matched_expected_tests=true
else
  matched_expected_tests=false
fi

if [[ "${status:-}" != "fail" && "${matched_expected_tests}" == "true" ]]; then
  exit_code=0
  status="pass"
elif [[ "${status:-}" != "fail" ]]; then
  exit_code=1
  status="fail"
fi

metrics="$(python3 - "${exit_code}" "${database_url_source}" "${log_file}" "${tests[@]}" <<'PY'
import json
import pathlib
import re
import sys

exit_code = int(sys.argv[1])
database_url_source = sys.argv[2]
log_path = pathlib.Path(sys.argv[3])
tests = sys.argv[4:]
output = log_path.read_text()

print(json.dumps({
    "command": " && ".join(
        f"cargo test --test replay_integration_test {test_name} -- --exact"
        for test_name in tests
    ),
    "tests": tests,
    "exit_code": exit_code,
    "log_path": str(log_path),
    "running_1_test_count": output.count("running 1 test"),
    "matched_test_count": sum(
        1
        for test_name in tests
        if re.search(rf"test\s+{re.escape(test_name)}\s+\.\.\.\s+ok\b", output) is not None
    ),
    "matched_pass_summary_count": output.count("test result: ok. 1 passed;"),
    "test_database_url_configured": database_url_source != "missing",
    "database_url_source": database_url_source,
}))
PY
)"

phase7_write_result_json "${scenario}" "${status}" "${metrics}"
printf '%s: %s\n' "${scenario}" "${status^^}"
test "${status}" = "pass"
