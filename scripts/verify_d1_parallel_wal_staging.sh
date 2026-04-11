#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BEAD_ID="bd-3wop3.1.2"
SCENARIO_ID="parallel_wal_lane_staging"
COMPATIBILITY_SELECTOR="wal_invariant,integrity_check,row_level"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${BEAD_ID}-${TIMESTAMP_UTC}"
TRACE_ID="trace-${RUN_ID}"
ARTIFACT_DIR="${REPO_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
RUN_ROOT="${ARTIFACT_DIR}/runs"
TEST_LOG="${ARTIFACT_DIR}/cargo-test.log"
REPORT_JSON="${ARTIFACT_DIR}/parallel_wal_staging_report.json"
GATE_REPORT_JSON="${ARTIFACT_DIR}/gate_report.json"
RESULT="running"

mkdir -p "${RUN_ROOT}"
: > "${TEST_LOG}"

emit_event() {
    local phase="$1"
    local outcome="$2"
    local message="$3"
    printf '{"trace_id":"%s","run_id":"%s","scenario_id":"%s","phase":"%s","outcome":"%s","compatibility_selector":"%s","timestamp":"%s","message":"%s"}\n' \
        "${TRACE_ID}" "${RUN_ID}" "${SCENARIO_ID}" "${phase}" "${outcome}" "${COMPATIBILITY_SELECTOR}" \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${message}" >> "${ARTIFACT_DIR}/events.jsonl"
}

finish() {
    local exit_code=$?
    if [[ ${exit_code} -eq 0 ]]; then
        RESULT="pass"
    else
        RESULT="fail"
    fi

    cat > "${GATE_REPORT_JSON}" <<EOF
{
  "bead_id": "${BEAD_ID}",
  "run_id": "${RUN_ID}",
  "trace_id": "${TRACE_ID}",
  "scenario_id": "${SCENARIO_ID}",
  "compatibility_selector": "${COMPATIBILITY_SELECTOR}",
  "artifact_dir": "${ARTIFACT_DIR}",
  "test_log": "${TEST_LOG}",
  "report_json": "${REPORT_JSON}",
  "result": "${RESULT}"
}
EOF

    emit_event "finalize" "${RESULT}" "gate report written to ${GATE_REPORT_JSON}"

    if [[ ${exit_code} -eq 0 ]]; then
        echo "[GATE PASS] ${BEAD_ID} parallel WAL staging verification passed"
    else
        echo "[GATE FAIL] ${BEAD_ID} parallel WAL staging verification failed"
        echo "artifact_dir=${ARTIFACT_DIR}"
    fi
}
trap finish EXIT

run_step() {
    local phase="$1"
    local description="$2"
    shift 2

    emit_event "${phase}" "running" "${description}"
    if "$@" 2>&1 | tee -a "${TEST_LOG}"; then
        emit_event "${phase}" "pass" "${description}"
    else
        emit_event "${phase}" "fail" "${description}"
        return 1
    fi
}

run_compile_step() {
    local phase="$1"
    local description="$2"
    shift 2

    if command -v rch >/dev/null 2>&1; then
        emit_event "${phase}" "running" "${description} via rch"
        if rch exec -- "$@" 2>&1 | tee -a "${TEST_LOG}"; then
            emit_event "${phase}" "pass" "${description} via rch"
            return 0
        fi
        emit_event "${phase}" "running" "${description} falling back to local execution"
    fi

    if "$@" 2>&1 | tee -a "${TEST_LOG}"; then
        emit_event "${phase}" "pass" "${description} via local fallback"
    else
        emit_event "${phase}" "fail" "${description} via local fallback"
        return 1
    fi
}

echo "=== ${BEAD_ID}: per-core append lanes and local frame staging verification ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"
echo "compatibility_selector=${COMPATIBILITY_SELECTOR}"
echo "artifact_dir=${ARTIFACT_DIR}"

emit_event "bootstrap" "running" "verification started"

run_compile_step \
    "wal_unit" \
    "running bead-scoped WAL lane-stager unit coverage" \
    cargo test -p fsqlite-wal lane_stager -- --nocapture

run_compile_step \
    "pager_unit" \
    "running bead-scoped pager unit coverage" \
    cargo test -p fsqlite-pager bd_3wop3_1_2 -- --nocapture

run_compile_step \
    "e2e" \
    "running bead-scoped e2e lane staging matrix" \
    env \
    RUST_LOG=trace \
    FSQLITE_BD_3WOP3_1_2_RUN_ROOT="${RUN_ROOT}" \
    FSQLITE_BD_3WOP3_1_2_ARTIFACT="${REPORT_JSON}" \
    cargo test -p fsqlite-e2e --test bd_3wop3_1_2_parallel_wal_staging -- --nocapture --test-threads=1

if [[ ! -f "${REPORT_JSON}" ]]; then
    emit_event "artifact_check" "fail" "missing expected report ${REPORT_JSON}"
    echo "ERROR: missing expected report ${REPORT_JSON}" >&2
    exit 1
fi

emit_event "artifact_check" "pass" "found report ${REPORT_JSON}"
