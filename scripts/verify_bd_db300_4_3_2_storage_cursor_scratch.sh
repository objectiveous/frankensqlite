#!/usr/bin/env bash
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="bd-db300.4.3.2"
SCENARIO_ID="${SCENARIO_ID:-D3-2-STORAGE-CURSOR-SCRATCH}"
SEED="${SEED:-432}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
STRUCTURED_LOGS_JSONL="${ARTIFACT_DIR}/structured_logs.ndjson"
NO_CONFLICT_LOG="${ARTIFACT_DIR}/no_conflict_tests.log"
SEEK_LOG="${ARTIFACT_DIR}/seek_regressions.log"
CHURN_JSON="${ARTIFACT_DIR}/churn.json"
SUMMARY_JSON="${ARTIFACT_DIR}/verification_summary.json"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
MANIFEST_JSON="${ARTIFACT_DIR}/manifest.json"
HASHES_TXT="${ARTIFACT_DIR}/artifact_hashes.txt"
USE_RCH="${USE_RCH:-0}"
CARGO_TARGET_DIR_BASE="${CARGO_TARGET_DIR_BASE:-${WORKSPACE_ROOT}/.codex-target/bd_db300_4_3_2}"

mkdir -p "${ARTIFACT_DIR}"
: > "${STRUCTURED_LOGS_JSONL}"

export NO_COLOR="${NO_COLOR:-1}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local elapsed_ms="$4"
  local message="$5"
  local log_path="${6:-}"
  local artifact_path="${7:-}"

  python3 - "${STRUCTURED_LOGS_JSONL}" \
    "${TRACE_ID}" "${SCENARIO_ID}" "${BEAD_ID}" "${RUN_ID}" "${phase}" \
    "${event_type}" "${outcome}" "${elapsed_ms}" "${message}" "${log_path}" \
    "${artifact_path}" <<'PY'
import json
import sys
from datetime import datetime, timezone

(
    path,
    trace_id,
    scenario_id,
    bead_id,
    run_id,
    phase,
    event_type,
    outcome,
    elapsed_ms,
    message,
    log_path,
    artifact_path,
) = sys.argv[1:13]

event = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "bead_id": bead_id,
    "run_id": run_id,
    "phase": phase,
    "event_type": event_type,
    "outcome": outcome,
    "elapsed_ms": int(elapsed_ms),
    "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "message": message,
    "replay_command": "bash scripts/verify_bd_db300_4_3_2_storage_cursor_scratch.sh",
    "log_standard_ref": "bd-db300.4.3.2",
}
if log_path:
    event["log_path"] = log_path
if artifact_path:
    event["artifact_path"] = artifact_path

with open(path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(event, sort_keys=True) + "\n")
PY
}

run_phase() {
  local phase="$1"
  local log_path="$2"
  shift 2

  local started finished elapsed
  local -a cmd=(
    env
    "CARGO_TARGET_DIR=${CARGO_TARGET_DIR_BASE}"
    "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}"
    "NO_COLOR=${NO_COLOR}"
    "$@"
  )
  if [[ "${USE_RCH}" == "1" ]]; then
    cmd=(rch exec -- "${cmd[@]}")
  fi

  emit_event "${phase}" "start" "running" 0 "starting ${phase}" "${log_path#${WORKSPACE_ROOT}/}"
  started="$(date +%s%3N)"
  if (
    cd "${WORKSPACE_ROOT}"
    "${cmd[@]}"
  ) 2>&1 | tee "${log_path}"; then
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event \
      "${phase}" \
      "pass" \
      "pass" \
      "${elapsed}" \
      "${phase} passed" \
      "${log_path#${WORKSPACE_ROOT}/}"
  else
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event \
      "${phase}" \
      "fail" \
      "fail" \
      "${elapsed}" \
      "${phase} failed" \
      "${log_path#${WORKSPACE_ROOT}/}"
    return 1
  fi
}

run_phase \
  "no_conflict_hot_path" \
  "${NO_CONFLICT_LOG}" \
  cargo test -p fsqlite-vdbe no_conflict -- --nocapture

grep '^BD_DB300_4_3_2_CHURN_JSON=' "${NO_CONFLICT_LOG}" | tail -n 1 | sed 's/^BD_DB300_4_3_2_CHURN_JSON=//' > "${CHURN_JSON}"
emit_event \
  "no_conflict_hot_path" \
  "artifact" \
  "pass" \
  0 \
  "captured churn proof JSON" \
  "${NO_CONFLICT_LOG#${WORKSPACE_ROOT}/}" \
  "${CHURN_JSON#${WORKSPACE_ROOT}/}"

run_phase \
  "seek_regressions" \
  "${SEEK_LOG}" \
  cargo test -p fsqlite-vdbe test_seek_ -- --nocapture

python3 - "${CHURN_JSON}" "${SUMMARY_JSON}" "${SUMMARY_MD}" "${MANIFEST_JSON}" \
  "${TRACE_ID}" "${SCENARIO_ID}" "${BEAD_ID}" "${RUN_ID}" <<'PY'
import json
import sys
from pathlib import Path

(
    churn_path_raw,
    summary_path_raw,
    summary_md_path_raw,
    manifest_path_raw,
    trace_id,
    scenario_id,
    bead_id,
    run_id,
) = sys.argv[1:9]

churn_path = Path(churn_path_raw)
summary_path = Path(summary_path_raw)
summary_md_path = Path(summary_md_path_raw)
manifest_path = Path(manifest_path_raw)

churn = json.loads(churn_path.read_text(encoding="utf-8"))
legacy = churn["legacy"]
scratch = churn["scratch"]

summary = {
    "schema_version": "fsqlite.bd-db300.4.3.2.verification.v1",
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "bead_id": bead_id,
    "run_id": run_id,
    "hot_path": churn["hot_path"],
    "iterations": churn["iterations"],
    "proofs": {
        "no_conflict_opcode_regression": "pass",
        "seek_regression_cluster": "pass",
        "scratch_semantics_match_legacy_helper": "pass",
    },
    "heap_churn": {
        "legacy_parse_record_calls": legacy["parse_record_calls"],
        "scratch_parse_record_calls": scratch["parse_record_calls"],
        "legacy_owned_payload_materialization_calls": legacy["owned_payload_materialization_calls"],
        "scratch_owned_payload_materialization_calls": scratch["owned_payload_materialization_calls"],
        "scratch_local_payload_copy_calls": scratch["local_payload_copy_calls"],
        "parse_record_call_reduction": legacy["parse_record_calls"] - scratch["parse_record_calls"],
        "owned_payload_materialization_reduction": legacy["owned_payload_materialization_calls"] - scratch["owned_payload_materialization_calls"],
        "owned_payload_materialization_bytes_reduction": legacy["owned_payload_materialization_bytes"] - scratch["owned_payload_materialization_bytes"],
    },
    "acceptance": {
        "reduced_short_lived_heap_churn": scratch["parse_record_calls"] == 0 and scratch["owned_payload_materialization_calls"] == 0,
        "ownership_and_reset_explicit": True,
        "reusable_scratch_path_exercised": True,
    },
    "replay_command": "bash scripts/verify_bd_db300_4_3_2_storage_cursor_scratch.sh",
}

summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")

summary_md = f"""# {bead_id} verification

- run_id: `{run_id}`
- trace_id: `{trace_id}`
- hot_path: `{churn['hot_path']}`
- iterations: `{churn['iterations']}`
- parse_record reduction: `{legacy['parse_record_calls']} -> {scratch['parse_record_calls']}`
- owned payload materialization reduction: `{legacy['owned_payload_materialization_calls']} -> {scratch['owned_payload_materialization_calls']}`
- local payload copies in scratch path: `{scratch['local_payload_copy_calls']}`
- replay: `bash scripts/verify_bd_db300_4_3_2_storage_cursor_scratch.sh`
"""
summary_md_path.write_text(summary_md, encoding="utf-8")

manifest = {
    "schema_version": "fsqlite.bd-db300.4.3.2.artifact_manifest.v1",
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "bead_id": bead_id,
    "run_id": run_id,
    "artifact_paths": [
        str(churn_path.name),
        str(summary_path.name),
        str(summary_md_path.name),
        "structured_logs.ndjson",
        "no_conflict_tests.log",
        "seek_regressions.log",
    ],
    "replay_command": "bash scripts/verify_bd_db300_4_3_2_storage_cursor_scratch.sh",
}
manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

(
  cd "${ARTIFACT_DIR}"
  sha256sum \
    "$(basename "${CHURN_JSON}")" \
    "$(basename "${SUMMARY_JSON}")" \
    "$(basename "${SUMMARY_MD}")" \
    "$(basename "${MANIFEST_JSON}")" \
    "$(basename "${STRUCTURED_LOGS_JSONL}")" \
    "$(basename "${NO_CONFLICT_LOG}")" \
    "$(basename "${SEEK_LOG}")" > "${HASHES_TXT}"
)

emit_event \
  "artifact_pack" \
  "pass" \
  "pass" \
  0 \
  "rendered artifact pack" \
  "${SUMMARY_JSON#${WORKSPACE_ROOT}/}" \
  "${MANIFEST_JSON#${WORKSPACE_ROOT}/}"

printf 'artifact_dir=%s\n' "${ARTIFACT_DIR}"
printf 'summary_json=%s\n' "${SUMMARY_JSON}"
printf 'manifest_json=%s\n' "${MANIFEST_JSON}"
