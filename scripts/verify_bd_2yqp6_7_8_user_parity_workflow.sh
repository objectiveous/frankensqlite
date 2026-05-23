#!/usr/bin/env bash
# One-command parity verification workflow and artifact navigator for bd-2yqp6.7.8.

set -euo pipefail

BEAD_ID="bd-2yqp6.7.8"
SCENARIO_ID="PARITY-WORKFLOW-G8"
SEED=7258
GENERATED_UNIX_MS=1700000000000
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="${BEAD_ID}-$(date -u +%Y%m%dT%H%M%SZ)-${SEED}"
TRACE_ID="trace-${RUN_ID}"
ARTIFACT_ROOT="${FSQLITE_PARITY_WORKFLOW_ARTIFACT_ROOT:-${REPO_ROOT}/artifacts/${BEAD_ID}}"
SCENARIO="synthetic-success"
JSON_OUTPUT=false
ALL_SYNTHETIC=false
DIFF_CI_LANE="${FSQLITE_PARITY_WORKFLOW_DIFF_CI_LANE:-smoke}"
RUNNER_TARGET_DIR="${REPO_ROOT}/.rch-target-parity-workflow-${RUN_ID}"
WORKFLOW_RUNNER_BIN="${RUNNER_TARGET_DIR}/debug/parity_verification_workflow_runner"
EVIDENCE_GATE_BIN="${RUNNER_TARGET_DIR}/debug/parity_evidence_matrix_gate"
STATUS_RUNNER_BIN="${RUNNER_TARGET_DIR}/debug/parity_status_report_runner"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      JSON_OUTPUT=true
      shift
      ;;
    --scenario)
      if [[ $# -lt 2 ]]; then
        echo "ERROR: --scenario requires a value" >&2
        exit 2
      fi
      SCENARIO="$2"
      shift 2
      ;;
    --all-synthetic-scenarios)
      ALL_SYNTHETIC=true
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

case "${SCENARIO}" in
  synthetic-success | synthetic-preflight-fail | synthetic-gate-fail | synthetic-stale-evidence | live-smoke)
    ;;
  *)
    echo "ERROR: unsupported scenario: ${SCENARIO}" >&2
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

resolve_runner_binary() {
  local bin_name="$1"
  local expected_path="$2"
  if [[ -x "${expected_path}" ]]; then
    printf '%s\n' "${expected_path}"
    return 0
  fi
  find "${REPO_ROOT}" -maxdepth 3 \
    -path "${REPO_ROOT}/.rch-target-*/debug/${bin_name}" \
    -type f \
    -perm -111 \
    -printf '%T@ %p\n' \
    | sort -nr \
    | head -n 1 \
    | cut -d' ' -f2-
}

ensure_runner_binary() {
  local bin_name="$1"
  local expected_path="$2"
  if [[ -x "${expected_path}" ]]; then
    return 0
  fi
  run_cargo_with_target "${RUNNER_TARGET_DIR}" cargo build -p fsqlite-harness --bin "${bin_name}" >&2
  local resolved
  resolved="$(resolve_runner_binary "${bin_name}" "${expected_path}")"
  if [[ -z "${resolved}" || ! -x "${resolved}" ]]; then
    echo "ERROR: could not locate ${bin_name} after cargo build" >&2
    return 1
  fi
}

runner_binary() {
  local bin_name="$1"
  local expected_path="$2"
  ensure_runner_binary "${bin_name}" "${expected_path}" >&2
  resolve_runner_binary "${bin_name}" "${expected_path}"
}

write_artifact() {
  local path="$1"
  local payload="$2"
  mkdir -p "$(dirname "${path}")"
  printf '%s\n' "${payload}" > "${path}"
}

hash_file() {
  sha256sum "$1" | awk '{print $1}'
}

phase_json() {
  local phase="$1"
  local command="$2"
  local outcome="$3"
  local exit_code="$4"
  local started_ms="$5"
  local finished_ms="$6"
  local artifact_json="$7"
  local failure_json="$8"
  jq -n \
    --arg phase "${phase}" \
    --arg command "${command}" \
    --arg outcome "${outcome}" \
    --argjson exit_code "${exit_code}" \
    --argjson started_unix_ms "${started_ms}" \
    --argjson finished_unix_ms "${finished_ms}" \
    --argjson artifact_ids "${artifact_json}" \
    --argjson first_failure "${failure_json}" \
    '{
      phase: $phase,
      command: $command,
      outcome: $outcome,
      exit_code: $exit_code,
      started_unix_ms: $started_unix_ms,
      finished_unix_ms: $finished_unix_ms,
      artifact_ids: $artifact_ids,
      first_failure: $first_failure
    }'
}

failure_json() {
  local phase="$1"
  local summary="$2"
  local pointer="$3"
  local artifact_json="$4"
  local commands_json="$5"
  jq -n \
    --arg phase "${phase}" \
    --arg summary "${summary}" \
    --arg pointer "${pointer}" \
    --argjson artifact_ids "${artifact_json}" \
    --argjson remediation_commands "${commands_json}" \
    '{
      phase: $phase,
      summary: $summary,
      diagnostic_json_pointer: $pointer,
      artifact_ids: $artifact_ids,
      remediation_commands: $remediation_commands
    }'
}

artifact_json() {
  local artifact_id="$1"
  local role="$2"
  local path="$3"
  local replay_command="$4"
  local observed_ms="$5"
  local required="$6"
  local sha256
  sha256="$(hash_file "${path}")"
  jq -n \
    --arg artifact_id "${artifact_id}" \
    --arg role "${role}" \
    --arg path "${path}" \
    --arg sha256 "${sha256}" \
    --arg replay_command "${replay_command}" \
    --argjson observed_unix_ms "${observed_ms}" \
    --argjson required "${required}" \
    '{
      artifact_id: $artifact_id,
      role: $role,
      path: $path,
      sha256: $sha256,
      replay_command: $replay_command,
      observed_unix_ms: $observed_unix_ms,
      required: $required
    }'
}

build_synthetic_input() {
  local scenario="$1"
  local artifact_dir="$2"
  local input_json="$3"
  local preflight_json="${artifact_dir}/oracle_preflight.json"
  local differential_json="${artifact_dir}/differential_manifest.json"
  local evidence_json="${artifact_dir}/parity_evidence_matrix.json"
  local status_json="${artifact_dir}/parity_status_report.json"
  local status_md="${artifact_dir}/parity_status_report.md"
  local events_jsonl="${artifact_dir}/events.jsonl"
  local observed_ms="${GENERATED_UNIX_MS}"

  if [[ "${scenario}" == "synthetic-stale-evidence" ]]; then
    observed_ms=$((GENERATED_UNIX_MS - 90000001))
  fi

  write_artifact "${preflight_json}" '{"outcome":"green","certifying":true}'
  write_artifact "${differential_json}" '{"run_report":{"total_cases":3,"passed":3,"diverged":0}}'
  write_artifact "${evidence_json}" '{"final_gate_passed":true,"evidence_classes":4}'
  write_artifact "${status_json}" '{"report_complete":true,"evidence_freshness":{"overall_fresh":true}}'
  write_artifact "${status_md}" '# Synthetic parity status report'
  write_artifact "${events_jsonl}" '{"event":"synthetic"}'

  local preflight_outcome="pass"
  local preflight_exit=0
  local evidence_outcome="pass"
  local evidence_exit=0
  local preflight_failure="null"
  local evidence_failure="null"

  if [[ "${scenario}" == "synthetic-preflight-fail" ]]; then
    preflight_outcome="fail"
    preflight_exit=1
    preflight_failure="$(failure_json \
      "preflight_doctor" \
      "oracle preflight doctor failed" \
      "/oracle_preflight/findings/0" \
      '["oracle_preflight"]' \
      '["cargo run -p fsqlite-harness --bin oracle_preflight_doctor_runner"]')"
  fi

  if [[ "${scenario}" == "synthetic-gate-fail" ]]; then
    evidence_outcome="fail"
    evidence_exit=1
    evidence_failure="$(failure_json \
      "parity_evidence_matrix" \
      "parity evidence matrix gate failed" \
      "/parity_evidence_matrix/violations/0" \
      '["parity_evidence_matrix"]' \
      '["cargo run -p fsqlite-harness --bin parity_evidence_matrix_gate"]')"
  fi

  local steps_json
  steps_json="$(
    jq -s '.' < <(
      phase_json "preflight_doctor" "cargo run -p fsqlite-harness --bin oracle_preflight_doctor_runner" "${preflight_outcome}" "${preflight_exit}" "${GENERATED_UNIX_MS}" "$((GENERATED_UNIX_MS + 1))" '["oracle_preflight"]' "${preflight_failure}"
      phase_json "differential_ci" "bash scripts/verify_differential_ci_lane.sh --lane smoke --json" "pass" 0 "$((GENERATED_UNIX_MS + 2))" "$((GENERATED_UNIX_MS + 3))" '["differential_manifest"]' "null"
      phase_json "parity_evidence_matrix" "cargo run -p fsqlite-harness --bin parity_evidence_matrix_gate" "${evidence_outcome}" "${evidence_exit}" "$((GENERATED_UNIX_MS + 4))" "$((GENERATED_UNIX_MS + 5))" '["parity_evidence_matrix"]' "${evidence_failure}"
      phase_json "parity_status_report" "cargo run -p fsqlite-harness --bin parity_status_report_runner" "pass" 0 "$((GENERATED_UNIX_MS + 6))" "$((GENERATED_UNIX_MS + 7))" '["parity_status_json","parity_status_markdown"]' "null"
      phase_json "certificate_readiness" "cargo run -p fsqlite-harness --bin parity_verification_workflow_runner" "pass" 0 "$((GENERATED_UNIX_MS + 8))" "$((GENERATED_UNIX_MS + 9))" '["oracle_preflight","differential_manifest","parity_evidence_matrix","parity_status_json","parity_status_markdown"]' "null"
    )
  )"

  local artifacts_json
  artifacts_json="$(
    jq -s '.' < <(
      artifact_json "oracle_preflight" "oracle_preflight_json" "${preflight_json}" "cargo run -p fsqlite-harness --bin oracle_preflight_doctor_runner" "${observed_ms}" true
      artifact_json "differential_manifest" "differential_manifest_json" "${differential_json}" "bash scripts/verify_differential_ci_lane.sh --lane smoke --json" "${observed_ms}" true
      artifact_json "parity_evidence_matrix" "parity_evidence_matrix_json" "${evidence_json}" "cargo run -p fsqlite-harness --bin parity_evidence_matrix_gate" "${observed_ms}" true
      artifact_json "parity_status_json" "parity_status_json" "${status_json}" "cargo run -p fsqlite-harness --bin parity_status_report_runner" "${observed_ms}" true
      artifact_json "parity_status_markdown" "parity_status_markdown" "${status_md}" "cargo run -p fsqlite-harness --bin parity_status_report_runner" "${observed_ms}" true
      artifact_json "workflow_events" "workflow_events_jsonl" "${events_jsonl}" "bash scripts/verify_bd_2yqp6_7_8_user_parity_workflow.sh --scenario ${scenario}" "${GENERATED_UNIX_MS}" false
    )
  )"

  jq -n \
    --arg run_id "${RUN_ID}-${scenario}" \
    --arg trace_id "${TRACE_ID}-${scenario}" \
    --arg scenario_id "${SCENARIO_ID}-${scenario}" \
    --argjson seed "${SEED}" \
    --argjson generated_unix_ms "${GENERATED_UNIX_MS}" \
    --argjson freshness_budget_ms "86400000" \
    --argjson steps "${steps_json}" \
    --argjson artifacts "${artifacts_json}" \
    '{
      run_id: $run_id,
      trace_id: $trace_id,
      scenario_id: $scenario_id,
      seed: $seed,
      generated_unix_ms: $generated_unix_ms,
      freshness_budget_ms: $freshness_budget_ms,
      steps: $steps,
      artifacts: $artifacts
    }' > "${input_json}"
}

build_live_input() {
  local artifact_dir="$1"
  local input_json="$2"
  local differential_root="${REPO_ROOT}/artifacts/differential-ci/${DIFF_CI_LANE}"
  local preflight_json="${differential_root}/doctor/oracle_preflight_doctor.json"
  local differential_json="${differential_root}/run-a/differential_manifest.json"
  local evidence_json="${artifact_dir}/parity_evidence_matrix.json"
  local status_json="${artifact_dir}/parity_status_report.json"
  local status_md="${artifact_dir}/parity_status_report.md"
  local events_jsonl="${artifact_dir}/events.jsonl"
  local evidence_gate_bin
  local status_runner_bin

  mkdir -p "${artifact_dir}"
  write_artifact "${events_jsonl}" '{"event":"live-smoke-start"}'

  DIFF_LANE_FORCE_RCH=1 bash "${REPO_ROOT}/scripts/verify_differential_ci_lane.sh" \
    --lane "${DIFF_CI_LANE}" \
    --json \
    --seed "${SEED}" \
    --generated-unix-ms "${GENERATED_UNIX_MS}" > "${artifact_dir}/differential_ci_lane.stdout"

  evidence_gate_bin="$(runner_binary "parity_evidence_matrix_gate" "${EVIDENCE_GATE_BIN}")"
  "${evidence_gate_bin}" \
    --workspace-root "${REPO_ROOT}" \
    --output "${evidence_json}" > "${artifact_dir}/parity_evidence_matrix.stdout"

  status_runner_bin="$(runner_binary "parity_status_report_runner" "${STATUS_RUNNER_BIN}")"
  "${status_runner_bin}" \
    --workspace-root "${REPO_ROOT}" \
    --oracle-preflight-json "${preflight_json}" \
    --differential-manifest-json "${differential_json}" \
    --generated-unix-ms "${GENERATED_UNIX_MS}" \
    --output-json "${status_json}" \
    --output-human "${status_md}" > "${artifact_dir}/parity_status_report.stdout"

  local steps_json
  steps_json="$(
    jq -s '.' < <(
      phase_json "preflight_doctor" "bash scripts/verify_differential_ci_lane.sh --lane ${DIFF_CI_LANE} --json" "pass" 0 "${GENERATED_UNIX_MS}" "$((GENERATED_UNIX_MS + 1))" '["oracle_preflight"]' "null"
      phase_json "differential_ci" "bash scripts/verify_differential_ci_lane.sh --lane ${DIFF_CI_LANE} --json" "pass" 0 "$((GENERATED_UNIX_MS + 2))" "$((GENERATED_UNIX_MS + 3))" '["differential_manifest"]' "null"
      phase_json "parity_evidence_matrix" "cargo run -p fsqlite-harness --bin parity_evidence_matrix_gate" "pass" 0 "$((GENERATED_UNIX_MS + 4))" "$((GENERATED_UNIX_MS + 5))" '["parity_evidence_matrix"]' "null"
      phase_json "parity_status_report" "cargo run -p fsqlite-harness --bin parity_status_report_runner" "pass" 0 "$((GENERATED_UNIX_MS + 6))" "$((GENERATED_UNIX_MS + 7))" '["parity_status_json","parity_status_markdown"]' "null"
      phase_json "certificate_readiness" "cargo run -p fsqlite-harness --bin parity_verification_workflow_runner" "pass" 0 "$((GENERATED_UNIX_MS + 8))" "$((GENERATED_UNIX_MS + 9))" '["oracle_preflight","differential_manifest","parity_evidence_matrix","parity_status_json","parity_status_markdown"]' "null"
    )
  )"

  local artifacts_json
  artifacts_json="$(
    jq -s '.' < <(
      artifact_json "oracle_preflight" "oracle_preflight_json" "${preflight_json}" "bash scripts/verify_differential_ci_lane.sh --lane ${DIFF_CI_LANE} --json" "${GENERATED_UNIX_MS}" true
      artifact_json "differential_manifest" "differential_manifest_json" "${differential_json}" "bash scripts/verify_differential_ci_lane.sh --lane ${DIFF_CI_LANE} --json" "${GENERATED_UNIX_MS}" true
      artifact_json "parity_evidence_matrix" "parity_evidence_matrix_json" "${evidence_json}" "cargo run -p fsqlite-harness --bin parity_evidence_matrix_gate" "${GENERATED_UNIX_MS}" true
      artifact_json "parity_status_json" "parity_status_json" "${status_json}" "cargo run -p fsqlite-harness --bin parity_status_report_runner" "${GENERATED_UNIX_MS}" true
      artifact_json "parity_status_markdown" "parity_status_markdown" "${status_md}" "cargo run -p fsqlite-harness --bin parity_status_report_runner" "${GENERATED_UNIX_MS}" true
      artifact_json "workflow_events" "workflow_events_jsonl" "${events_jsonl}" "bash scripts/verify_bd_2yqp6_7_8_user_parity_workflow.sh --scenario live-smoke" "${GENERATED_UNIX_MS}" false
    )
  )"

  jq -n \
    --arg run_id "${RUN_ID}-live-smoke" \
    --arg trace_id "${TRACE_ID}-live-smoke" \
    --arg scenario_id "${SCENARIO_ID}-live-smoke" \
    --argjson seed "${SEED}" \
    --argjson generated_unix_ms "${GENERATED_UNIX_MS}" \
    --argjson freshness_budget_ms "86400000" \
    --argjson steps "${steps_json}" \
    --argjson artifacts "${artifacts_json}" \
    '{
      run_id: $run_id,
      trace_id: $trace_id,
      scenario_id: $scenario_id,
      seed: $seed,
      generated_unix_ms: $generated_unix_ms,
      freshness_budget_ms: $freshness_budget_ms,
      steps: $steps,
      artifacts: $artifacts
    }' > "${input_json}"
}

run_one_scenario() {
  local scenario="$1"
  local expected_exit="$2"
  local artifact_dir="${ARTIFACT_ROOT}/${RUN_ID}/${scenario}"
  local input_json="${artifact_dir}/workflow_input.json"
  local report_json="${artifact_dir}/parity_verification_workflow.json"
  local report_md="${artifact_dir}/parity_verification_workflow.md"
  local runner_exit=0
  local workflow_runner_bin

  mkdir -p "${artifact_dir}"
  if [[ "${scenario}" == "live-smoke" ]]; then
    build_live_input "${artifact_dir}" "${input_json}"
  else
    build_synthetic_input "${scenario}" "${artifact_dir}" "${input_json}"
  fi

  workflow_runner_bin="$(runner_binary "parity_verification_workflow_runner" "${WORKFLOW_RUNNER_BIN}")"

  set +e
  "${workflow_runner_bin}" \
    --workspace-root "${REPO_ROOT}" \
    --input-json "${input_json}" \
    --output-json "${report_json}" \
    --output-human "${report_md}"
  runner_exit=$?
  set -e

  if [[ "${runner_exit}" -ne "${expected_exit}" ]]; then
    echo "ERROR: scenario ${scenario} expected exit ${expected_exit}, got ${runner_exit}" >&2
    return 1
  fi

  jq -e \
    --arg scenario "${scenario}" \
    --arg bead_id "${BEAD_ID}" \
    '
      .bead_id == $bead_id and
      (.artifact_index | type == "array" and length >= 5) and
      (.steps | map(.phase)) == [
        "preflight_doctor",
        "differential_ci",
        "parity_evidence_matrix",
        "parity_status_report",
        "certificate_readiness"
      ] and
      (
        ($scenario == "synthetic-success" or $scenario == "live-smoke") == .workflow_complete
      )
    ' "${report_json}" >/dev/null

  if [[ "${JSON_OUTPUT}" == "true" ]]; then
    jq -n \
      --arg scenario "${scenario}" \
      --arg report_json "${report_json}" \
      --arg report_markdown "${report_md}" \
      --arg result "pass" \
      '{scenario: $scenario, report_json: $report_json, report_markdown: $report_markdown, result: $result}'
  else
    echo "scenario=${scenario}"
    echo "report_json=${report_json}"
    echo "report_markdown=${report_md}"
    echo "expected_exit=${expected_exit}"
    echo "result=pass"
  fi
}

if [[ "${ALL_SYNTHETIC}" == "true" ]]; then
  run_one_scenario "synthetic-success" 0
  run_one_scenario "synthetic-preflight-fail" 1
  run_one_scenario "synthetic-gate-fail" 1
  run_one_scenario "synthetic-stale-evidence" 1
  exit 0
fi

case "${SCENARIO}" in
  synthetic-success | live-smoke)
    run_one_scenario "${SCENARIO}" 0
    ;;
  synthetic-preflight-fail | synthetic-gate-fail | synthetic-stale-evidence)
    run_one_scenario "${SCENARIO}" 1
    exit 1
    ;;
esac
