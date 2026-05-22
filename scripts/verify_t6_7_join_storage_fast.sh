#!/usr/bin/env bash
set -euo pipefail

# bd-1dp9.6.7.4.1.1: fast join-cutover verification path.
#
# This avoids compiling the 160k-line fsqlite-core test binary for the
# connection.rs join cutover. It verifies the storage-cursor join proof at the
# VDBE layer and records the concurrent-writer default guard. The default target
# dir is the repo's warm shared target for speed; use --target-dir to force an
# isolated target when shared Cargo locking is the thing being investigated.
#
# Usage:
#   ./scripts/verify_t6_7_join_storage_fast.sh [--target-dir DIR]
#
# Artifacts:
#   artifacts/bd-1dp9.6.7.4.1.1/<run_id>/events.jsonl
#   artifacts/bd-1dp9.6.7.4.1.1/<run_id>/vdbe-storage-join.log
#   artifacts/bd-1dp9.6.7.4.1.1/<run_id>/report.json

BEAD_ID="bd-1dp9.6.7.4.1.1"
SCENARIO_ID="t6_7_join_storage_fast"
SEED=67411

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
ARTIFACT_ROOT="${ARTIFACT_ROOT:-artifacts/${BEAD_ID}}"
ARTIFACT_DIR="${ARTIFACT_DIR:-${ARTIFACT_ROOT}/${RUN_ID}}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
VDBE_LOG="${ARTIFACT_DIR}/vdbe-storage-join.log"
CONCURRENT_GUARD="${ARTIFACT_DIR}/concurrent-mode-default.txt"
SOURCE_GUARD="${ARTIFACT_DIR}/vdbe-storage-join-source.txt"
HASHES_TXT="${ARTIFACT_DIR}/sha256.txt"
REPORT_JSON="${ARTIFACT_DIR}/report.json"
RUST_LOG_VALUE="${FSQLITE_VERIFY_RUST_LOG:-error}"
TARGET_DIR="${FSQLITE_VERIFY_CARGO_TARGET_DIR:-/data/tmp/cargo-target}"
CARGO_BUILD_JOBS="${FSQLITE_VERIFY_CARGO_BUILD_JOBS:-4}"
CARGO_INCREMENTAL_VALUE="${FSQLITE_VERIFY_CARGO_INCREMENTAL:-0}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target-dir)
      TARGET_DIR="$2"
      shift 2
      ;;
    *)
      echo "ERROR: unknown argument '$1'" >&2
      exit 2
      ;;
  esac
done

mkdir -p "${ARTIFACT_DIR}"
: > "${EVENTS_JSONL}"

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local message="$4"
  printf '{"trace_id":"%s","run_id":"%s","scenario_id":"%s","seed":%d,"phase":"%s","event_type":"%s","outcome":"%s","timestamp":"%s","message":"%s"}\n' \
    "${TRACE_ID}" "${RUN_ID}" "${SCENARIO_ID}" "${SEED}" "${phase}" "${event_type}" "${outcome}" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${message}" \
    >> "${EVENTS_JSONL}"
}

sha256_file() {
  sha256sum "$1" | awk '{print $1}'
}

cd "${REPO_ROOT}"

vdbe_cmd=(
  rch exec -- env
  "CARGO_TARGET_DIR=${TARGET_DIR}"
  "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}"
  "CARGO_INCREMENTAL=${CARGO_INCREMENTAL_VALUE}"
  "RUST_LOG=${RUST_LOG_VALUE}"
  cargo test -p fsqlite-vdbe --lib test_storage_only_ -- --nocapture
)
REPLAY_COMMAND="RUN_ID=${RUN_ID} TRACE_ID=${TRACE_ID} FSQLITE_VERIFY_CARGO_TARGET_DIR=${TARGET_DIR} ${0}"

echo "=== ${BEAD_ID}: fast storage-join verification ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"
echo "target_dir=${TARGET_DIR}"
echo "artifact_dir=${ARTIFACT_DIR}"
echo "replay=${REPLAY_COMMAND}"

emit_event "bootstrap" "start" "running" "fast storage-join verification started"

if rg -n 'concurrent_mode_default: RefCell::new\(true\)' \
  crates/fsqlite-core/src/connection.rs > "${CONCURRENT_GUARD}"; then
  emit_event "concurrent_default_guard" "pass" "pass" "concurrent_mode_default remains true"
else
  emit_event "concurrent_default_guard" "fail" "fail" "concurrent_mode_default true guard missing"
  exit 1
fi

if rg -n 'fn test_storage_only_nested_loop_join_executes_without_attached_memdb' \
  crates/fsqlite-vdbe/src/engine.rs > "${SOURCE_GUARD}"; then
  emit_event "source_guard" "pass" "pass" "VDBE storage-only nested-loop join proof exists"
else
  emit_event "source_guard" "fail" "fail" "VDBE storage-only nested-loop join proof missing"
  exit 1
fi

emit_event "vdbe_storage_join" "start" "running" "running targeted fsqlite-vdbe storage-only join tests"
printf '+'
printf ' %q' "${vdbe_cmd[@]}"
printf '\n'

if "${vdbe_cmd[@]}" > "${VDBE_LOG}" 2>&1; then
  RESULT="pass"
  emit_event "vdbe_storage_join" "pass" "pass" "targeted fsqlite-vdbe storage-only join tests passed"
else
  RESULT="fail"
  emit_event "vdbe_storage_join" "fail" "fail" "targeted fsqlite-vdbe storage-only join tests failed"
fi

if [[ "${RESULT}" == "pass" ]]; then
  grep -q 'test_storage_only_table_program_executes_without_attached_memdb' "${VDBE_LOG}"
  grep -q 'test_storage_only_nested_loop_join_executes_without_attached_memdb' "${VDBE_LOG}"
  grep -q 'test result: ok' "${VDBE_LOG}"
fi

sha256sum "${EVENTS_JSONL}" "${VDBE_LOG}" "${CONCURRENT_GUARD}" "${SOURCE_GUARD}" \
  > "${HASHES_TXT}"

EVENTS_SHA256="$(sha256_file "${EVENTS_JSONL}")"
VDBE_SHA256="$(sha256_file "${VDBE_LOG}")"
CONCURRENT_SHA256="$(sha256_file "${CONCURRENT_GUARD}")"
SOURCE_SHA256="$(sha256_file "${SOURCE_GUARD}")"

cat > "${REPORT_JSON}" <<EOF
{
  "schema_version": "fsqlite.verify_t6_7_join_storage_fast.v1",
  "bead_id": "${BEAD_ID}",
  "trace_id": "${TRACE_ID}",
  "run_id": "${RUN_ID}",
  "scenario_id": "${SCENARIO_ID}",
  "seed": ${SEED},
  "result": "${RESULT}",
  "target_dir": "${TARGET_DIR}",
  "replay_command": "${REPLAY_COMMAND}",
  "cargo_command": "${vdbe_cmd[*]}",
  "verification_scope": [
    "fsqlite-vdbe storage-only table program without attached MemDatabase",
    "fsqlite-vdbe storage-only nested-loop join without attached MemDatabase",
    "source guard for concurrent_mode_default=true"
  ],
  "core_compile_policy": "does not compile fsqlite-core test binary; connection.rs guard is source-checked only",
  "artifacts": {
    "events_jsonl": {
      "path": "${EVENTS_JSONL}",
      "sha256": "${EVENTS_SHA256}"
    },
    "vdbe_storage_join_log": {
      "path": "${VDBE_LOG}",
      "sha256": "${VDBE_SHA256}"
    },
    "concurrent_guard": {
      "path": "${CONCURRENT_GUARD}",
      "sha256": "${CONCURRENT_SHA256}"
    },
    "source_guard": {
      "path": "${SOURCE_GUARD}",
      "sha256": "${SOURCE_SHA256}"
    }
  }
}
EOF

echo "result=${RESULT}"
echo "events=${EVENTS_JSONL}"
echo "log=${VDBE_LOG}"
echo "report=${REPORT_JSON}"

if [[ "${RESULT}" != "pass" ]]; then
  tail -80 "${VDBE_LOG}" || true
  exit 1
fi
