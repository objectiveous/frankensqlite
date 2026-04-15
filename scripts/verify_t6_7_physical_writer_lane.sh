#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BEAD_ID="bd-1dp9.6.7.9.1"
SCENARIO_ID="physical_writer_lane"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${BEAD_ID}-${TIMESTAMP_UTC}"
TRACE_ID="trace-${RUN_ID}"
ARTIFACT_DIR="${REPO_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
RUN_ROOT="${ARTIFACT_DIR}/runs"
TEST_LOG="${ARTIFACT_DIR}/cargo-test.log"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
LANE_REPORT_JSON="${ARTIFACT_DIR}/physical_writer_lane_report.json"
GATE_REPORT_JSON="${ARTIFACT_DIR}/gate_report.json"
RESULT="running"

mkdir -p "${RUN_ROOT}"
: > "${TEST_LOG}"
: > "${EVENTS_JSONL}"

emit_event() {
    local phase="$1"
    local outcome="$2"
    local message="$3"
    printf '{"trace_id":"%s","run_id":"%s","scenario_id":"%s","phase":"%s","outcome":"%s","timestamp":"%s","message":"%s"}\n' \
        "${TRACE_ID}" "${RUN_ID}" "${SCENARIO_ID}" "${phase}" "${outcome}" \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${message}" >> "${EVENTS_JSONL}"
}

hash_file() {
    local path="$1"
    if [[ -f "${path}" ]] && command -v sha256sum >/dev/null 2>&1; then
        sha256sum "${path}" | awk '{print $1}'
    else
        printf ''
    fi
}

finish() {
    local exit_code=$?
    local test_log_sha lane_report_sha events_sha
    test_log_sha="$(hash_file "${TEST_LOG}")"
    lane_report_sha="$(hash_file "${LANE_REPORT_JSON}")"
    events_sha="$(hash_file "${EVENTS_JSONL}")"

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
  "artifact_dir": "${ARTIFACT_DIR}",
  "test_log": "${TEST_LOG}",
  "lane_report_json": "${LANE_REPORT_JSON}",
  "events_jsonl": "${EVENTS_JSONL}",
  "artifact_hashes": {
    "test_log_sha256": "${test_log_sha}",
    "lane_report_sha256": "${lane_report_sha}",
    "events_sha256": "${events_sha}"
  },
  "replay_commands": [
    "cargo test -p fsqlite-pager arrival_wait_policy_ -- --nocapture",
    "cargo test -p fsqlite-pager physical_writer_ -- --nocapture",
    "cargo test -p fsqlite-e2e --test bd_3wop3_1_2_parallel_wal_staging -- --nocapture --test-threads=1"
  ],
  "bounded_wait_evidence_fields": [
    "queue_delay_ns",
    "target_wait_ns",
    "max_wait_ns"
  ],
  "result": "${RESULT}"
}
EOF

    emit_event "finalize" "${RESULT}" "gate report written to ${GATE_REPORT_JSON}"

    if [[ ${exit_code} -eq 0 ]]; then
        echo "[GATE PASS] ${BEAD_ID} physical writer lane verification passed"
    else
        echo "[GATE FAIL] ${BEAD_ID} physical writer lane verification failed"
        echo "artifact_dir=${ARTIFACT_DIR}"
    fi
}
trap finish EXIT

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

run_local_step() {
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

assert_lane_report_contract() {
    jq -e '
        (.runs | to_entries | length) > 0
        and all(.runs | to_entries[];
            .value.required_fields.run_id
            and .value.required_fields.batch_id
            and .value.required_fields.batch_membership
            and .value.required_fields.queue_delay_ns
            and .value.required_fields.target_wait_ns
            and .value.required_fields.max_wait_ns
            and .value.required_fields.fsync_boundary
            and .value.required_fields.ordering_phase
            and .value.required_fields.rollback_mode_active
        )
        and (.runs.conservative.fallback_reasons | index("operator_forced") != null)
    ' "${LANE_REPORT_JSON}" >/dev/null
}

echo "=== ${BEAD_ID}: physical writer lane verification ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"
echo "artifact_dir=${ARTIFACT_DIR}"

emit_event "bootstrap" "running" "verification started"

run_compile_step \
    "pager_arrival_wait" \
    "running arrival-wait policy unit coverage" \
    cargo test -p fsqlite-pager arrival_wait_policy_ -- --nocapture

run_compile_step \
    "pager_physical_writer_helpers" \
    "running physical-writer helper unit coverage" \
    cargo test -p fsqlite-pager physical_writer_ -- --nocapture

run_compile_step \
    "e2e_lane_contract" \
    "running file-backed lane staging e2e contract" \
    env \
    RUST_LOG=trace \
    FSQLITE_BD_3WOP3_1_2_RUN_ROOT="${RUN_ROOT}" \
    FSQLITE_BD_3WOP3_1_2_ARTIFACT="${LANE_REPORT_JSON}" \
    cargo test -p fsqlite-e2e --test bd_3wop3_1_2_parallel_wal_staging -- --nocapture --test-threads=1

if [[ ! -f "${LANE_REPORT_JSON}" ]]; then
    run_local_step \
        "e2e_lane_contract_local_artifact_replay" \
        "remote run did not materialize a local artifact; rerunning e2e locally for report capture" \
        env \
        RUST_LOG=trace \
        FSQLITE_BD_3WOP3_1_2_RUN_ROOT="${RUN_ROOT}" \
        FSQLITE_BD_3WOP3_1_2_ARTIFACT="${LANE_REPORT_JSON}" \
        cargo test -p fsqlite-e2e --test bd_3wop3_1_2_parallel_wal_staging -- --nocapture --test-threads=1
fi

if [[ ! -f "${LANE_REPORT_JSON}" ]]; then
    emit_event "artifact_check" "fail" "missing expected report ${LANE_REPORT_JSON}"
    echo "ERROR: missing expected report ${LANE_REPORT_JSON}" >&2
    exit 1
fi

assert_lane_report_contract
emit_event "artifact_check" "pass" "validated lane report contract ${LANE_REPORT_JSON}"
