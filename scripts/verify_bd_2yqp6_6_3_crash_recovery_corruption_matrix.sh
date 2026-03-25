#!/usr/bin/env bash
# Verification gate for bd-2yqp6.6.3:
# deterministic crash-recovery/corruption differential matrix with replayable artifacts.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="bd-2yqp6.6.3"
SCENARIO_ID="CRASH-RECOVERY-CORRUPTION-MATRIX"
REPLAY_COMMAND="cargo test -p fsqlite-e2e --test bd_2yqp6_6_3_crash_recovery_corruption_matrix -- --nocapture --test-threads=1"
DEFAULT_STRATEGY_IDS="bitflip_db_single,truncate_db_half,wal_truncate_0,wal_bitflip_frame0,wal_torn_write_frame1,header_zero"

JSON_OUTPUT=false
STRATEGY_FILTER="${FSQLITE_CRASH_MATRIX_ONLY:-}"
RUN_TAG="${BD_2YQP6_6_3_RUN_TAG:-verify}"
SEED="${BD_2YQP6_6_3_WRAPPER_SEED:-2}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      JSON_OUTPUT=true
      shift
      ;;
    --strategies)
      shift
      [[ $# -gt 0 ]] || {
        echo "ERROR: --strategies requires a value" >&2
        exit 2
      }
      STRATEGY_FILTER="$1"
      shift
      ;;
    --run-tag)
      shift
      [[ $# -gt 0 ]] || {
        echo "ERROR: --run-tag requires a value" >&2
        exit 2
      }
      RUN_TAG="$1"
      shift
      ;;
    --seed)
      shift
      [[ $# -gt 0 ]] || {
        echo "ERROR: --seed requires a value" >&2
        exit 2
      }
      SEED="$1"
      shift
      ;;
    *)
      echo "ERROR: unknown argument '$1'" >&2
      exit 2
      ;;
  esac
done

TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${BEAD_ID}-${RUN_TAG}-${TIMESTAMP_UTC}-${SEED}"
TRACE_ID="trace-${RUN_ID}"
ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
REPORT_JSON="${ARTIFACT_DIR}/report.json"
MANIFEST_JSON="${ARTIFACT_DIR}/manifest.json"
SCENARIO_SUMMARY_JSONL="${ARTIFACT_DIR}/scenario_summary.jsonl"
LOG_CONTRACT="${ARTIFACT_DIR}/contract-check.log"
LOG_TEST="${ARTIFACT_DIR}/cargo-test-bd_2yqp6_6_3_crash_recovery_corruption_matrix.log"
MATRIX_ARTIFACT_JSON="${ARTIFACT_DIR}/crash_matrix_artifact.json"

mkdir -p "${ARTIFACT_DIR}"
start_ns="$(date +%s%N)"

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local message="$4"
  local now_ns elapsed_ms
  now_ns="$(date +%s%N)"
  elapsed_ms="$(( (now_ns - start_ns) / 1000000 ))"
  printf '{"trace_id":"%s","run_id":"%s","scenario_id":"%s","seed":%s,"phase":"%s","event_type":"%s","outcome":"%s","elapsed_ms":%d,"timestamp":"%s","message":"%s"}\n' \
    "${TRACE_ID}" "${RUN_ID}" "${SCENARIO_ID}" "${SEED}" "${phase}" "${event_type}" "${outcome}" "${elapsed_ms}" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${message}" \
    >> "${EVENTS_JSONL}"
}

expected_row_count() {
  local selected="${1:-}"
  if [[ -z "${selected}" ]]; then
    selected="${DEFAULT_STRATEGY_IDS}"
  fi
  python3 - "${selected}" <<'PY'
import sys
items = [item.strip() for item in sys.argv[1].split(",") if item.strip()]
print(len(items))
PY
}

echo "=== ${BEAD_ID}: crash-recovery corruption matrix verification ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"
echo "replay=${REPLAY_COMMAND}"
if [[ -n "${STRATEGY_FILTER}" ]]; then
  echo "strategy_filter=${STRATEGY_FILTER}"
fi

emit_event "bootstrap" "start" "running" "verification started"

emit_event "contract_scan" "start" "running" "checking matrix contract markers"
if {
  rg -n "const REPLAY_COMMAND" crates/fsqlite-e2e/tests/bd_2yqp6_6_3_crash_recovery_corruption_matrix.rs
  rg -n "const ARTIFACT_ENV_PATH" crates/fsqlite-e2e/tests/bd_2yqp6_6_3_crash_recovery_corruption_matrix.rs
  rg -n "SCENARIO_OUTCOME:" crates/fsqlite-e2e/tests/bd_2yqp6_6_3_crash_recovery_corruption_matrix.rs
  rg -n "maybe_write_artifact" crates/fsqlite-e2e/tests/bd_2yqp6_6_3_crash_recovery_corruption_matrix.rs
} >"${LOG_CONTRACT}" 2>&1; then
  emit_event "contract_scan" "pass" "pass" "matrix contract markers found"
else
  emit_event "contract_scan" "fail" "fail" "matrix contract markers missing"
  echo "[GATE FAIL] contract scan failed; see ${LOG_CONTRACT}" >&2
  exit 1
fi

emit_event "matrix_tests" "start" "running" "running crash/corruption matrix test"
set +e
(
  cd "${WORKSPACE_ROOT}"
  if [[ -n "${STRATEGY_FILTER}" ]]; then
    FSQLITE_CRASH_MATRIX_ONLY="${STRATEGY_FILTER}" \
    FSQLITE_CRASH_MATRIX_ARTIFACT="${MATRIX_ARTIFACT_JSON}" \
    cargo test -p fsqlite-e2e --test bd_2yqp6_6_3_crash_recovery_corruption_matrix -- --nocapture --test-threads=1
  else
    FSQLITE_CRASH_MATRIX_ARTIFACT="${MATRIX_ARTIFACT_JSON}" \
    cargo test -p fsqlite-e2e --test bd_2yqp6_6_3_crash_recovery_corruption_matrix -- --nocapture --test-threads=1
  fi
) >"${LOG_TEST}" 2>&1
TEST_STATUS=$?
set -e
if [[ ${TEST_STATUS} -eq 0 ]]; then
  emit_event "matrix_tests" "pass" "pass" "matrix integration test passed"
  TEST_RESULT="pass"
else
  emit_event "matrix_tests" "fail" "fail" "matrix integration test failed"
  TEST_RESULT="fail"
fi

emit_event "extract_outcomes" "start" "running" "extracting SCENARIO_OUTCOME rows"
if grep -E 'SCENARIO_OUTCOME:' "${LOG_TEST}" >/dev/null; then
  sed -n 's/.*SCENARIO_OUTCOME://p' "${LOG_TEST}" > "${SCENARIO_SUMMARY_JSONL}"
else
  : > "${SCENARIO_SUMMARY_JSONL}"
fi
SCENARIO_ROWS="$(wc -l < "${SCENARIO_SUMMARY_JSONL}" | tr -d ' ')"
EXPECTED_ROWS="$(expected_row_count "${STRATEGY_FILTER}")"
if [[ "${SCENARIO_ROWS}" -lt "${EXPECTED_ROWS}" ]]; then
  emit_event "extract_outcomes" "fail" "fail" "expected >=${EXPECTED_ROWS} scenario summaries, got ${SCENARIO_ROWS}"
  SUMMARY_RESULT="fail"
else
  emit_event "extract_outcomes" "pass" "pass" "scenario_summary_rows=${SCENARIO_ROWS}"
  SUMMARY_RESULT="pass"
fi

emit_event "artifact_validation" "start" "running" "validating crash-matrix artifact payload"
if [[ -f "${MATRIX_ARTIFACT_JSON}" ]] && jq -e \
  --arg bead_id "${BEAD_ID}" \
  --arg replay_command "${REPLAY_COMMAND}" \
  --argjson expected_rows "${EXPECTED_ROWS}" \
  '.bead_id == $bead_id
   and (.run_count >= $expected_rows)
   and (.replay_command == $replay_command)
   and ((.runs | length) >= $expected_rows)' \
  "${MATRIX_ARTIFACT_JSON}" >/dev/null 2>&1; then
  emit_event "artifact_validation" "pass" "pass" "artifact payload looks complete"
  ARTIFACT_RESULT="pass"
else
  emit_event "artifact_validation" "fail" "fail" "artifact payload missing or incomplete"
  ARTIFACT_RESULT="fail"
fi

if [[ "${TEST_RESULT}" == "pass" && "${SUMMARY_RESULT}" == "pass" && "${ARTIFACT_RESULT}" == "pass" ]]; then
  RESULT="pass"
else
  RESULT="fail"
fi

CONTRACT_SHA="$(sha256sum "${LOG_CONTRACT}" | awk '{print $1}')"
TEST_SHA="$(sha256sum "${LOG_TEST}" | awk '{print $1}')"
SUMMARY_SHA="$(sha256sum "${SCENARIO_SUMMARY_JSONL}" | awk '{print $1}')"
ARTIFACT_SHA=""
RUN_COUNT="-1"
OVERALL_STATUS=""
if [[ -f "${MATRIX_ARTIFACT_JSON}" ]]; then
  ARTIFACT_SHA="$(sha256sum "${MATRIX_ARTIFACT_JSON}" | awk '{print $1}')"
  RUN_COUNT="$(jq -r '.run_count // -1' "${MATRIX_ARTIFACT_JSON}" 2>/dev/null || echo -1)"
  OVERALL_STATUS="$(jq -r '.overall_status // ""' "${MATRIX_ARTIFACT_JSON}" 2>/dev/null || true)"
fi

cat > "${MANIFEST_JSON}" <<EOF_MANIFEST
{
  "trace_id": "${TRACE_ID}",
  "run_id": "${RUN_ID}",
  "scenario_id": "${SCENARIO_ID}",
  "seed": ${SEED},
  "bead_id": "${BEAD_ID}",
  "files": [
    {"path":"${EVENTS_JSONL}"},
    {"path":"${LOG_CONTRACT}","sha256":"${CONTRACT_SHA}"},
    {"path":"${LOG_TEST}","sha256":"${TEST_SHA}"},
    {"path":"${SCENARIO_SUMMARY_JSONL}","sha256":"${SUMMARY_SHA}"},
    {"path":"${MATRIX_ARTIFACT_JSON}","sha256":"${ARTIFACT_SHA}"}
  ]
}
EOF_MANIFEST

MANIFEST_SHA="$(sha256sum "${MANIFEST_JSON}" | awk '{print $1}')"

cat > "${REPORT_JSON}" <<EOF_REPORT
{
  "trace_id": "${TRACE_ID}",
  "run_id": "${RUN_ID}",
  "scenario_id": "${SCENARIO_ID}",
  "seed": ${SEED},
  "bead_id": "${BEAD_ID}",
  "result": "${RESULT}",
  "strategy_filter": "${STRATEGY_FILTER}",
  "expected_rows": ${EXPECTED_ROWS},
  "scenario_summary_rows": ${SCENARIO_ROWS},
  "artifact_run_count": ${RUN_COUNT},
  "artifact_overall_status": "${OVERALL_STATUS}",
  "events_jsonl": "${EVENTS_JSONL}",
  "scenario_summary_jsonl": "${SCENARIO_SUMMARY_JSONL}",
  "contract_log": "${LOG_CONTRACT}",
  "test_log": "${LOG_TEST}",
  "matrix_artifact_json": "${MATRIX_ARTIFACT_JSON}",
  "manifest_json": "${MANIFEST_JSON}",
  "manifest_sha256": "${MANIFEST_SHA}",
  "replay_command": "${REPLAY_COMMAND}",
  "artifact_bundle": {
    "report_path": "${REPORT_JSON}",
    "manifest_path": "${MANIFEST_JSON}",
    "events_path": "${EVENTS_JSONL}",
    "scenario_summary_path": "${SCENARIO_SUMMARY_JSONL}",
    "test_log_path": "${LOG_TEST}",
    "matrix_artifact_path": "${MATRIX_ARTIFACT_JSON}"
  }
}
EOF_REPORT

emit_event "finalize" "info" "${RESULT}" "report written to ${REPORT_JSON}"

if [[ "${JSON_OUTPUT}" == "true" ]]; then
  cat "${REPORT_JSON}"
else
  echo "Result:             ${RESULT}"
  echo "Strategy filter:    ${STRATEGY_FILTER:-<default>}"
  echo "Scenario rows:      ${SCENARIO_ROWS}"
  echo "Expected rows:      ${EXPECTED_ROWS}"
  echo "Artifact path:      ${MATRIX_ARTIFACT_JSON}"
  echo "Report:             ${REPORT_JSON}"
  echo "Manifest:           ${MANIFEST_JSON}"
fi

[[ "${RESULT}" == "pass" ]]
