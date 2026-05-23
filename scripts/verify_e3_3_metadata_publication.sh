#!/usr/bin/env bash
# Verification gate for bd-db300.5.3.3 metadata publication contracts.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${WORKSPACE_ROOT}"
BEAD_ID="bd-db300.5.3.3"
SCENARIO_ID="E3_3_METADATA_PUBLICATION"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${BEAD_ID}-${TIMESTAMP_UTC}"
TRACE_ID="trace-${RUN_ID}"
ARTIFACT_DIR="${ARTIFACT_DIR:-${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
REPORT_JSON="${ARTIFACT_DIR}/report.json"

mkdir -p "${ARTIFACT_DIR}"
: >"${EVENTS_JSONL}"

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local elapsed_ms="$4"
  local message="$5"
  printf '{"trace_id":"%s","run_id":"%s","scenario_id":"%s","phase":"%s","event_type":"%s","outcome":"%s","elapsed_ms":%s,"timestamp":"%s","message":"%s"}\n' \
    "${TRACE_ID}" "${RUN_ID}" "${SCENARIO_ID}" "${phase}" "${event_type}" "${outcome}" "${elapsed_ms}" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${message}" \
    >>"${EVENTS_JSONL}"
}

run_phase() {
  local phase="$1"
  local logfile="$2"
  shift 2

  emit_event "${phase}" "start" "running" 0 "running: $*"
  local started finished elapsed
  started="$(date +%s%3N)"
  if "$@" 2>&1 | tee "${logfile}"; then
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event "${phase}" "pass" "pass" "${elapsed}" "completed: $*"
  else
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event "${phase}" "fail" "fail" "${elapsed}" "failed: $*"
    return 1
  fi
}

echo "=== ${BEAD_ID}: metadata publication verification ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"
echo "artifacts=${ARTIFACT_DIR}"

emit_event "bootstrap" "start" "running" 0 "verification started"

run_phase \
  "committed_snapshot_unit_matrix" \
  "${ARTIFACT_DIR}/committed_snapshot_unit_matrix.log" \
  rch exec -- cargo test -p fsqlite-pager committed_snapshot -- --nocapture --test-threads=1

run_phase \
  "committed_snapshot_reclamation_stress" \
  "${ARTIFACT_DIR}/committed_snapshot_reclamation_stress.log" \
  rch exec -- cargo test -p fsqlite-pager \
    test_committed_snapshot_reclamation_stress_bounds_stale_arcs -- --nocapture

run_phase \
  "committed_snapshot_writer_progress" \
  "${ARTIFACT_DIR}/committed_snapshot_writer_progress.log" \
  rch exec -- cargo test -p fsqlite-pager \
    test_committed_snapshot_writer_not_starved_by_reader_pressure -- --nocapture

run_phase \
  "pager_check" \
  "${ARTIFACT_DIR}/pager_check.log" \
  rch exec -- cargo check -p fsqlite-pager --all-targets

run_phase \
  "concurrent_writer_smoke_bench" \
  "${ARTIFACT_DIR}/mt_mvcc_bench.log" \
  rch exec -- cargo run -p fsqlite-e2e --bin mt-mvcc-bench -- \
    --rows-per-thread=25 \
    --threads=1,2 \
    --iters=1 \
    --json-output="${ARTIFACT_DIR}/mt_mvcc_bench.json" \
    --summary-md="${ARTIFACT_DIR}/mt_mvcc_bench.md" \
    --history-json="${ARTIFACT_DIR}/mt_mvcc_bench.history.json"

{
  printf '{\n'
  printf '  "trace_id": "%s",\n' "${TRACE_ID}"
  printf '  "run_id": "%s",\n' "${RUN_ID}"
  printf '  "scenario_id": "%s",\n' "${SCENARIO_ID}"
  printf '  "bead_id": "%s",\n' "${BEAD_ID}"
  printf '  "events_jsonl": "%s",\n' "${EVENTS_JSONL}"
  printf '  "mt_mvcc_bench_json": "%s",\n' "${ARTIFACT_DIR}/mt_mvcc_bench.json"
  printf '  "result": "pass"\n'
  printf '}\n'
} >"${REPORT_JSON}"

emit_event "finalize" "pass" "pass" 0 "report written to ${REPORT_JSON}"
echo "[GATE PASS] ${BEAD_ID} metadata publication verification passed"
