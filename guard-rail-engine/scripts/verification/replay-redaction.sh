#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

ENGINE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
export PHASE7_SUITE_ID="${PHASE7_SUITE_ID:-$(phase7_timestamp)}"
database_url_source="missing"
if [[ -n "${TEST_DATABASE_URL:-}" ]]; then
  database_url_source="TEST_DATABASE_URL"
fi
if [[ -z "${TEST_DATABASE_URL:-}" && -n "${GUARDRAIL_DATABASE__URL:-}" ]]; then
  export TEST_DATABASE_URL="${GUARDRAIL_DATABASE__URL}"
  database_url_source="GUARDRAIL_DATABASE__URL"
fi

scenario="replay-redaction"
log_file="$(mktemp)"
tests=(
  "test_replay_artifacts_redact_sensitive_request_json_before_persistence"
  "test_replay_artifacts_redact_sensitive_response_json_before_persistence"
)

cleanup() {
  rm -f "${log_file}"
}
trap cleanup EXIT

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

if [[ "${status:-}" != "fail" ]]; then
  exit_code=0
  status="pass"
fi

metrics="$(python3 - "${exit_code}" "${database_url_source}" "${log_file}" "${tests[@]}" <<'PY'
import json
import pathlib
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
    "matched_pass_lines": output.count("test result: ok"),
    "test_database_url_configured": database_url_source != "missing",
    "database_url_source": database_url_source,
}))
PY
)"

phase7_write_result_json "${scenario}" "${status}" "${metrics}"
printf '%s: %s\n' "${scenario}" "${status^^}"
test "${status}" = "pass"
