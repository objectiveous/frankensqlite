#!/usr/bin/env bash
# Verification gate for bd-2yqp6.7.5:
# user-facing parity status report JSON + Markdown publication contract.

set -euo pipefail

BEAD_ID="bd-2yqp6.7.5"
SCENARIO_ID="PARITY-STATUS-G5"
SEED=7525
GENERATED_UNIX_MS=1700000000000
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="${BEAD_ID}-$(date -u +%Y%m%dT%H%M%SZ)-${SEED}"
TRACE_ID="trace-${RUN_ID}"
ARTIFACT_ROOT="${FSQLITE_PARITY_STATUS_ARTIFACT_ROOT:-${REPO_ROOT}/artifacts/${BEAD_ID}}"
ARTIFACT_DIR="${ARTIFACT_ROOT}/${RUN_ID}"
PREFLIGHT_JSON="${ARTIFACT_DIR}/oracle_preflight.json"
DIFFERENTIAL_JSON="${ARTIFACT_DIR}/differential_manifest.json"
REPORT_JSON="${ARTIFACT_DIR}/parity_status_report.json"
REPORT_MD="${ARTIFACT_DIR}/parity_status_report.md"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
TEST_LOG="${ARTIFACT_DIR}/test.log"
SUMMARY_JSON="${ARTIFACT_DIR}/verification_summary.json"
RUNNER_TARGET_DIR="${REPO_ROOT}/.rch-target-parity-status-${RUN_ID}"
RUNNER_BIN="${RUNNER_TARGET_DIR}/debug/parity_status_report_runner"
JSON_OUTPUT=false
INPUT_MODE="${FSQLITE_PARITY_STATUS_INPUT_MODE:-live-differential-ci}"
DIFF_CI_LANE="${FSQLITE_PARITY_STATUS_DIFF_CI_LANE:-smoke}"
DIFF_CI_REPORT_JSON=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      JSON_OUTPUT=true
      shift
      ;;
    --synthetic-inputs)
      INPUT_MODE="synthetic"
      shift
      ;;
    --live-differential-ci)
      INPUT_MODE="live-differential-ci"
      shift
      ;;
    --differential-ci-lane)
      if [[ $# -lt 2 ]]; then
        echo "ERROR: --differential-ci-lane requires a value" >&2
        exit 2
      fi
      DIFF_CI_LANE="$2"
      shift 2
      ;;
    *)
      echo "ERROR: unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

case "${INPUT_MODE}" in
  live-differential-ci | synthetic)
    ;;
  *)
    echo "ERROR: input mode must be live-differential-ci or synthetic: ${INPUT_MODE}" >&2
    exit 2
    ;;
esac

case "${DIFF_CI_LANE}" in
  smoke | expanded)
    ;;
  *)
    echo "ERROR: --differential-ci-lane must be smoke or expanded: ${DIFF_CI_LANE}" >&2
    exit 2
    ;;
esac

mkdir -p "${ARTIFACT_DIR}"

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local message="$4"
  printf '{"trace_id":"%s","run_id":"%s","scenario_id":"%s","seed":%d,"phase":"%s","event_type":"%s","outcome":"%s","timestamp":"%s","message":"%s"}\n' \
    "${TRACE_ID}" "${RUN_ID}" "${SCENARIO_ID}" "${SEED}" "${phase}" "${event_type}" "${outcome}" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${message}" \
    >> "${EVENTS_JSONL}"
}

run_cargo() {
  if command -v rch >/dev/null 2>&1 && [[ "${FSQLITE_DISABLE_RCH:-0}" != "1" ]]; then
    rch exec -- "$@"
  else
    "$@"
  fi
}

run_cargo_with_target() {
  local target_dir="$1"
  shift
  if command -v rch >/dev/null 2>&1 && [[ "${FSQLITE_DISABLE_RCH:-0}" != "1" ]]; then
    CARGO_TARGET_DIR="${target_dir}" rch exec -- "$@"
  else
    CARGO_TARGET_DIR="${target_dir}" "$@"
  fi
}

run_gate() {
  local phase="$1"
  shift
  emit_event "${phase}" "start" "running" "running: $*"
  if [[ "${JSON_OUTPUT}" == "true" ]]; then
    if "$@" >> "${TEST_LOG}" 2>&1; then
      emit_event "${phase}" "pass" "pass" "command passed"
      return 0
    fi
  else
    if "$@" 2>&1 | tee -a "${TEST_LOG}"; then
      emit_event "${phase}" "pass" "pass" "command passed"
      return 0
    fi
  fi
  emit_event "${phase}" "fail" "fail" "command failed"
  return 1
}

resolve_runner_binary() {
  if [[ -x "${RUNNER_BIN}" ]]; then
    printf '%s\n' "${RUNNER_BIN}"
    return 0
  fi
  find "${REPO_ROOT}" -maxdepth 3 \
    -path "${REPO_ROOT}/.rch-target-*/debug/parity_status_report_runner" \
    -type f \
    -perm -111 \
    -printf '%T@ %p\n' \
    | sort -nr \
    | head -n 1 \
    | cut -d' ' -f2-
}

write_synthetic_inputs() {
  cat > "${PREFLIGHT_JSON}" <<EOF_PREFLIGHT
{
  "schema_version": "1.0.0",
  "bead_id": "bd-2yqp6.2.5",
  "run_id": "${RUN_ID}-doctor",
  "trace_id": "${TRACE_ID}-doctor",
  "scenario_id": "${SCENARIO_ID}-DOCTOR",
  "seed": ${SEED},
  "generated_unix_ms": ${GENERATED_UNIX_MS},
  "outcome": "green",
  "certifying": true,
  "timing_ms": 11,
  "first_failure": null,
  "findings": [],
  "checks": {
    "expected_subject_identity": "frankensqlite",
    "expected_reference_identity": "csqlite-oracle",
    "expected_sqlite_version_prefix": "3.52.0",
    "fixtures_dir": "crates/fsqlite-harness/conformance",
    "fixture_manifest_path": "docs/contracts/corpus_manifest.toml",
    "oracle_binary_path": "sqlite3",
    "oracle_version": "3.52.0-test",
    "fixture_json_files_seen": 1,
    "fixture_entries_ingested": 1,
    "fixture_sql_statements_ingested": 2,
    "skipped_fixture_files": 0,
    "fixture_manifest_mtime_unix_ms": ${GENERATED_UNIX_MS},
    "fixture_manifest_sha256": "$(printf '%064d' 1)",
    "latest_fixture_mtime_unix_ms": ${GENERATED_UNIX_MS}
  },
  "replay_command": "cargo run -p fsqlite-harness --bin oracle_preflight_doctor_runner"
}
EOF_PREFLIGHT

  cat > "${DIFFERENTIAL_JSON}" <<EOF_DIFF
{
  "schema_version": 1,
  "bead_id": "bd-mblr.7.1.2",
  "run_id": "${RUN_ID}-diff",
  "trace_id": "${TRACE_ID}-diff",
  "scenario_id": "${SCENARIO_ID}-DIFF",
  "generated_unix_ms": ${GENERATED_UNIX_MS},
  "commit_sha": "synthetic",
  "root_seed": ${SEED},
  "overall_pass": false,
  "run_report": {
    "total_cases": 3,
    "passed": 2,
    "diverged": 1,
    "data_hash": "synthetic-data-hash"
  },
  "first_failure": {
    "root_cause_domain": "planner",
    "diagnostic_json_pointer": "/run_report/divergent_cases/0",
    "replay_command": "cargo run -p fsqlite-harness --bin differential_manifest_runner",
    "minimal_reproduction_json_pointer": null,
    "artifact_entries": ["differential_manifest_json", "differential_manifest_summary"],
    "remediation_playbook": {
      "summary": "Inspect planner mismatch frontier before release certification.",
      "owner_hint": "planner owners: inspect crates/fsqlite-planner first",
      "next_commands": ["cargo test -p fsqlite-planner", "cargo test -p fsqlite-harness --bin differential_manifest_runner"]
    }
  },
  "sampled_passing_replays": [
    {
      "case_id": "synthetic-pass",
      "transform_name": "identity",
      "seed": ${SEED},
      "replay_command": "cargo run -p fsqlite-harness --bin differential_manifest_runner",
      "diagnostic_json_pointer": "/run_report/sampled_passing_cases/0",
      "artifact_entries": ["differential_manifest_json"]
    }
  ],
  "replay": {
    "command": "cargo run -p fsqlite-harness --bin differential_manifest_runner"
  }
}
EOF_DIFF
}

prepare_live_differential_ci_inputs() {
  local lane_root="${REPO_ROOT}/artifacts/differential-ci/${DIFF_CI_LANE}"
  local lane_env=()

  PREFLIGHT_JSON="${lane_root}/doctor/oracle_preflight_doctor.json"
  DIFFERENTIAL_JSON="${lane_root}/run-a/differential_manifest.json"
  DIFF_CI_REPORT_JSON="${lane_root}/differential_ci_lane_report.json"

  if command -v rch >/dev/null 2>&1 && [[ "${FSQLITE_DISABLE_RCH:-0}" != "1" ]]; then
    lane_env=(DIFF_LANE_FORCE_RCH=1)
  fi

  if ! run_gate "differential_ci_${DIFF_CI_LANE}_lane" \
    env "${lane_env[@]}" bash "${REPO_ROOT}/scripts/verify_differential_ci_lane.sh" \
      --lane "${DIFF_CI_LANE}" \
      --json \
      --seed "${SEED}" \
      --generated-unix-ms "${GENERATED_UNIX_MS}"; then
    return 1
  fi

  for required_artifact in "${PREFLIGHT_JSON}" "${DIFFERENTIAL_JSON}" "${DIFF_CI_REPORT_JSON}"; do
    if [[ ! -s "${required_artifact}" ]]; then
      echo "ERROR: live differential CI artifact missing: ${required_artifact}" >&2
      return 1
    fi
  done
}

RESULT="pass"
emit_event "bootstrap" "start" "running" "verification started"

case "${INPUT_MODE}" in
  live-differential-ci)
    if ! prepare_live_differential_ci_inputs; then
      RESULT="fail"
    fi
    ;;
  synthetic)
    write_synthetic_inputs
    ;;
esac

if ! run_gate "parity_status_unit_tests" \
  run_cargo cargo test --manifest-path crates/fsqlite-harness/Cargo.toml --lib parity_status_report -- --nocapture; then
  RESULT="fail"
fi

if ! run_gate "parity_status_runner_build" \
  run_cargo_with_target "${RUNNER_TARGET_DIR}" cargo build -p fsqlite-harness --bin parity_status_report_runner; then
  RESULT="fail"
fi

RESOLVED_RUNNER_BIN="$(resolve_runner_binary)"
if [[ -z "${RESOLVED_RUNNER_BIN}" || ! -x "${RESOLVED_RUNNER_BIN}" ]]; then
  emit_event "parity_status_runner" "fail" "fail" "runner binary missing after build"
  RESULT="fail"
elif ! run_gate "parity_status_runner" \
  "${RESOLVED_RUNNER_BIN}" \
    --workspace-root "${REPO_ROOT}" \
    --oracle-preflight-json "${PREFLIGHT_JSON}" \
    --differential-manifest-json "${DIFFERENTIAL_JSON}" \
    --generated-unix-ms "${GENERATED_UNIX_MS}" \
    --output-json "${REPORT_JSON}" \
    --output-human "${REPORT_MD}"; then
  RESULT="fail"
fi

# shellcheck disable=SC2016 # jq program, not shell interpolation.
if ! run_gate "parity_status_json_contract" \
  jq -e \
    --arg schema_version "fsqlite.parity_status_report.v1" \
    --arg bead_id "${BEAD_ID}" \
    --arg input_mode "${INPUT_MODE}" \
    --arg differential_json "${DIFFERENTIAL_JSON}" \
    '
      .schema_version == $schema_version and
      .bead_id == $bead_id and
      .report_complete == true and
      (.features | type == "array" and length > 0) and
      .evidence_freshness.overall_fresh == true and
      .oracle_preflight.present == true and
      (.oracle_preflight.outcome | IN("green", "yellow")) and
      (.oracle_preflight.certifying | type == "boolean") and
      .current_frontier.present == true and
      .current_frontier.total_cases >= .current_frontier.passed_cases and
      .current_frontier.divergent_cases >= 0 and
      .current_frontier.sampled_passing_replay_count >= 0 and
      (
        (
          .current_frontier.divergent_cases == 0 and
          .current_frontier.first_failure == null
        ) or (
          .current_frontier.divergent_cases > 0 and
          (.current_frontier.first_failure | type == "object") and
          (.current_frontier.first_failure.root_cause_domain | IN("parser", "planner", "vdbe", "storage", "harness", "fixture")) and
          (.current_frontier.first_failure.remediation_playbook.next_commands | length) > 0
        )
      ) and
      (.divergence_ledger | length) > 0 and
      (
        $input_mode != "live-differential-ci" or
        (.evidence_freshness.sources | any(.source_id == "differential_manifest" and .artifact_path == $differential_json))
      ) and
      (.validation_violations | length) == 0
    ' "${REPORT_JSON}"; then
  RESULT="fail"
fi

if ! run_gate "parity_status_markdown_contract" \
  bash -c "grep -F '## Oracle Preflight' '${REPORT_MD}' >/dev/null && grep -F '## Current Frontier' '${REPORT_MD}' >/dev/null && grep -F '## Divergence Ledger' '${REPORT_MD}' >/dev/null"; then
  RESULT="fail"
fi

EVENTS_SHA256="$(sha256sum "${EVENTS_JSONL}" | awk '{print $1}')"
TEST_LOG_SHA256="$(sha256sum "${TEST_LOG}" | awk '{print $1}')"
REPORT_JSON_SHA256="$(sha256sum "${REPORT_JSON}" | awk '{print $1}')"
REPORT_MD_SHA256="$(sha256sum "${REPORT_MD}" | awk '{print $1}')"
PREFLIGHT_JSON_SHA256="$(sha256sum "${PREFLIGHT_JSON}" | awk '{print $1}')"
DIFFERENTIAL_JSON_SHA256="$(sha256sum "${DIFFERENTIAL_JSON}" | awk '{print $1}')"
DIFF_CI_REPORT_JSON_SHA256=""
if [[ -s "${DIFF_CI_REPORT_JSON}" ]]; then
  DIFF_CI_REPORT_JSON_SHA256="$(sha256sum "${DIFF_CI_REPORT_JSON}" | awk '{print $1}')"
fi

cat > "${SUMMARY_JSON}" <<EOF_SUMMARY
{
  "trace_id": "${TRACE_ID}",
  "run_id": "${RUN_ID}",
  "scenario_id": "${SCENARIO_ID}",
  "seed": ${SEED},
  "bead_id": "${BEAD_ID}",
  "input_mode": "${INPUT_MODE}",
  "differential_ci_lane": "${DIFF_CI_LANE}",
  "commands": [
    "DIFF_LANE_FORCE_RCH=1 bash scripts/verify_differential_ci_lane.sh --lane ${DIFF_CI_LANE} --json --seed ${SEED} --generated-unix-ms ${GENERATED_UNIX_MS}",
    "rch exec -- cargo test --manifest-path crates/fsqlite-harness/Cargo.toml --lib parity_status_report -- --nocapture",
    "rch exec -- cargo build -p fsqlite-harness --bin parity_status_report_runner",
    "${RESOLVED_RUNNER_BIN} --workspace-root ${REPO_ROOT} --oracle-preflight-json <preflight> --differential-manifest-json <manifest> --output-json <report> --output-human <markdown>",
    "jq -e <parity status schema contract>"
  ],
  "artifacts": {
    "events_jsonl": "${EVENTS_JSONL}",
    "events_sha256": "${EVENTS_SHA256}",
    "test_log": "${TEST_LOG}",
    "test_log_sha256": "${TEST_LOG_SHA256}",
    "oracle_preflight_json": "${PREFLIGHT_JSON}",
    "oracle_preflight_json_sha256": "${PREFLIGHT_JSON_SHA256}",
    "differential_manifest_json": "${DIFFERENTIAL_JSON}",
    "differential_manifest_json_sha256": "${DIFFERENTIAL_JSON_SHA256}",
    "differential_ci_report_json": "${DIFF_CI_REPORT_JSON}",
    "differential_ci_report_json_sha256": "${DIFF_CI_REPORT_JSON_SHA256}",
    "report_json": "${REPORT_JSON}",
    "report_json_sha256": "${REPORT_JSON_SHA256}",
    "report_markdown": "${REPORT_MD}",
    "report_markdown_sha256": "${REPORT_MD_SHA256}"
  },
  "result": "${RESULT}"
}
EOF_SUMMARY

emit_event "finalize" "info" "${RESULT}" "summary written to ${SUMMARY_JSON}"

if [[ "${JSON_OUTPUT}" == "true" ]]; then
  cat "${SUMMARY_JSON}"
else
  echo "=== ${BEAD_ID}: parity status report verification ==="
  echo "run_id=${RUN_ID}"
  echo "trace_id=${TRACE_ID}"
  echo "report_json=${REPORT_JSON}"
  echo "report_json_sha256=${REPORT_JSON_SHA256}"
  echo "report_markdown=${REPORT_MD}"
  echo "report_markdown_sha256=${REPORT_MD_SHA256}"
  echo "result=${RESULT}"
fi

if [[ "${RESULT}" != "pass" ]]; then
  echo "[GATE FAIL] ${BEAD_ID} parity status report verification failed" >&2
  exit 1
fi

if [[ "${JSON_OUTPUT}" != "true" ]]; then
  echo "[GATE PASS] ${BEAD_ID} parity status report verification passed"
fi
