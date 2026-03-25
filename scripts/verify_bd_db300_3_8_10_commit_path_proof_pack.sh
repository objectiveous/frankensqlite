#!/usr/bin/env bash
# Verification gate for bd-db300.3.8.10:
# orchestration-only commit-path proof pack using existing persistent, fairness, and crash evidence surfaces.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="bd-db300.3.8.10"
SCENARIO_ID="COMMIT-PATH-PROOF-PACK"
SEED="${BD_DB300_3_8_10_SEED:-3810}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}"
TRACE_ID="trace-${RUN_ID}"

JSON_OUTPUT=false
DRY_RUN="${DRY_RUN:-0}"
FORCE_CROSS_PROCESS="${FORCE_CROSS_PROCESS:-0}"
PERSISTENT_BASELINE_DIR="${PERSISTENT_BASELINE_DIR:-artifacts/bd-db300.3.8.8-full-20260324T200655Z}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      JSON_OUTPUT=true
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --force-cross-process)
      FORCE_CROSS_PROCESS=1
      shift
      ;;
    --persistent-baseline-dir)
      shift
      [[ $# -gt 0 ]] || {
        echo "ERROR: --persistent-baseline-dir requires a value" >&2
        exit 2
      }
      PERSISTENT_BASELINE_DIR="$1"
      shift
      ;;
    *)
      echo "ERROR: unknown argument '$1'" >&2
      exit 2
      ;;
  esac
done

resolve_path() {
  local path="$1"
  if [[ "${path}" == /* ]]; then
    printf '%s\n' "${path}"
  else
    printf '%s\n' "${WORKSPACE_ROOT}/${path}"
  fi
}

PERSISTENT_BASELINE_DIR="$(resolve_path "${PERSISTENT_BASELINE_DIR}")"
PERSISTENT_SCORECARD_JSON="${PERSISTENT_BASELINE_DIR}/persistent_scorecard.json"
PERSISTENT_SUMMARY_MD="${PERSISTENT_BASELINE_DIR}/summary.md"
PERSISTENT_RERUN_SH="${PERSISTENT_BASELINE_DIR}/rerun.sh"

CRASH_MATRIX_SCRIPT="${WORKSPACE_ROOT}/scripts/verify_bd_2yqp6_6_3_crash_recovery_corruption_matrix.sh"
FAIRNESS_SCRIPT="${WORKSPACE_ROOT}/scripts/verify_bd_1r0ha_3_concurrent_writer_e2e.sh"
CROSS_PROCESS_SCRIPT="${WORKSPACE_ROOT}/scripts/verify_bd_2g5_5_cross_process.sh"

ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
REPORT_JSON="${ARTIFACT_DIR}/report.json"
MANIFEST_JSON="${ARTIFACT_DIR}/manifest.json"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
HASHES_TXT="${ARTIFACT_DIR}/artifact_hashes.txt"
CRASH_JSON="${ARTIFACT_DIR}/crash_matrix_wrapper.json"
CRASH_LOG="${ARTIFACT_DIR}/crash_matrix_wrapper.log"
FAIRNESS_LOG="${ARTIFACT_DIR}/fairness_wrapper.log"
CROSS_JSON="${ARTIFACT_DIR}/cross_process_wrapper.json"
CROSS_LOG="${ARTIFACT_DIR}/cross_process_wrapper.log"

mkdir -p "${ARTIFACT_DIR}"
: > "${EVENTS_JSONL}"
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

require_file() {
  local phase="$1"
  local path="$2"
  if [[ ! -f "${path}" ]]; then
    emit_event "${phase}" "fail" "fail" "missing ${path}"
    echo "ERROR: missing ${path}" >&2
    exit 1
  fi
}

extract_cross_process_policy() {
  jq -r '
    if any(.critical_regimes[];
      (.collapse_override_applies // false)
      or ((.phase_metrics_medians.wake_timeout_median // 0) > 0)
      or ((.phase_metrics_medians.lock_topology_limited_sample_count // 0) > 0)
    )
    then "true"
    else "false"
    end
  ' "${PERSISTENT_SCORECARD_JSON}"
}

echo "=== ${BEAD_ID}: commit-path proof pack ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"
echo "persistent_baseline_dir=${PERSISTENT_BASELINE_DIR}"

emit_event "bootstrap" "start" "running" "proof pack started"

emit_event "persistent_baseline" "start" "running" "validating persistent baseline of record"
require_file "persistent_baseline" "${PERSISTENT_SCORECARD_JSON}"
require_file "persistent_baseline" "${PERSISTENT_SUMMARY_MD}"
if ! jq -e '.bead_id == "bd-db300.3.8.8" and .honest_gate_summary.complete_regime_count == 2' "${PERSISTENT_SCORECARD_JSON}" >/dev/null; then
  emit_event "persistent_baseline" "fail" "fail" "persistent baseline contract invalid"
  echo "ERROR: invalid persistent baseline scorecard: ${PERSISTENT_SCORECARD_JSON}" >&2
  exit 1
fi
PERSISTENT_VERDICT="$(jq -r '.honest_gate_summary.verdict' "${PERSISTENT_SCORECARD_JSON}")"
PERSISTENT_8T_VERDICT="$(jq -r '.critical_regimes[] | select(.regime_id=="persistent_concurrent_write_8t") | .verdict' "${PERSISTENT_SCORECARD_JSON}")"
PERSISTENT_16T_VERDICT="$(jq -r '.critical_regimes[] | select(.regime_id=="persistent_concurrent_write_16t") | .verdict' "${PERSISTENT_SCORECARD_JSON}")"
PERSISTENT_8T_RATIO="$(jq -r '.critical_regimes[] | select(.regime_id=="persistent_concurrent_write_8t") | .throughput_ratio_vs_sqlite' "${PERSISTENT_SCORECARD_JSON}")"
PERSISTENT_16T_RATIO="$(jq -r '.critical_regimes[] | select(.regime_id=="persistent_concurrent_write_16t") | .throughput_ratio_vs_sqlite' "${PERSISTENT_SCORECARD_JSON}")"
NEEDS_CROSS_PROCESS="$(extract_cross_process_policy)"
CROSS_PROCESS_REASON="persistent baseline does not show collapse override, timeout wakes, or lock-topology risk"
if [[ "${FORCE_CROSS_PROCESS}" == "1" ]]; then
  NEEDS_CROSS_PROCESS="true"
  CROSS_PROCESS_REASON="forced by FORCE_CROSS_PROCESS=1"
fi
emit_event "persistent_baseline" "pass" "pass" "persistent verdict=${PERSISTENT_VERDICT} 8t=${PERSISTENT_8T_VERDICT} 16t=${PERSISTENT_16T_VERDICT}"

require_file "child_scripts" "${CRASH_MATRIX_SCRIPT}"
require_file "child_scripts" "${FAIRNESS_SCRIPT}"
require_file "child_scripts" "${CROSS_PROCESS_SCRIPT}"
emit_event "child_scripts" "pass" "pass" "required child scripts are present"

CRASH_RESULT="dry_run"
FAIRNESS_RESULT="dry_run"
CROSS_RESULT="skipped"
CRASH_REPORT_PATH=""
FAIRNESS_REPORT_PATH=""
CROSS_REPORT_PATH=""

if [[ "${DRY_RUN}" != "1" ]]; then
  emit_event "crash_matrix" "start" "running" "running crash/corruption wrapper"
  set +e
  (
    cd "${WORKSPACE_ROOT}"
    bash "${CRASH_MATRIX_SCRIPT}" --json
  ) >"${CRASH_JSON}" 2>"${CRASH_LOG}"
  CRASH_STATUS=$?
  set -e
  if [[ -f "${CRASH_JSON}" ]]; then
    CRASH_RESULT="$(jq -r '.result // "fail"' "${CRASH_JSON}" 2>/dev/null || echo fail)"
    CRASH_REPORT_PATH="$(jq -r '.artifact_bundle.report_path // .report_json // ""' "${CRASH_JSON}" 2>/dev/null || true)"
  else
    CRASH_RESULT="fail"
  fi
  emit_event "crash_matrix" "$([[ ${CRASH_STATUS} -eq 0 ]] && echo pass || echo fail)" "${CRASH_RESULT}" "crash matrix wrapper completed"

  emit_event "fairness" "start" "running" "running concurrent-writer fairness wrapper"
  set +e
  (
    cd "${WORKSPACE_ROOT}"
    bash "${FAIRNESS_SCRIPT}"
  ) >"${FAIRNESS_LOG}" 2>&1
  FAIRNESS_STATUS=$?
  set -e
  FAIRNESS_RUN_ID="$(sed -n 's/^run_id=//p' "${FAIRNESS_LOG}" | tail -n1)"
  FAIRNESS_ARTIFACT_DIR="$(sed -n 's/^Artifacts: //p' "${FAIRNESS_LOG}" | tail -n1)"
  if [[ -z "${FAIRNESS_ARTIFACT_DIR}" && -n "${FAIRNESS_RUN_ID}" ]]; then
    FAIRNESS_ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/bd-1r0ha.3/${FAIRNESS_RUN_ID}"
  elif [[ -n "${FAIRNESS_ARTIFACT_DIR}" && "${FAIRNESS_ARTIFACT_DIR}" != /* ]]; then
    FAIRNESS_ARTIFACT_DIR="${WORKSPACE_ROOT}/${FAIRNESS_ARTIFACT_DIR}"
  fi
  if [[ -n "${FAIRNESS_ARTIFACT_DIR}" && -f "${FAIRNESS_ARTIFACT_DIR}/report.json" ]]; then
    FAIRNESS_REPORT_PATH="${FAIRNESS_ARTIFACT_DIR}/report.json"
    FAIRNESS_RESULT="$(jq -r '.result // "fail"' "${FAIRNESS_REPORT_PATH}" 2>/dev/null || echo fail)"
  elif [[ ${FAIRNESS_STATUS} -eq 0 ]]; then
    FAIRNESS_RESULT="pass"
  else
    FAIRNESS_RESULT="fail"
  fi
  emit_event "fairness" "$([[ ${FAIRNESS_STATUS} -eq 0 ]] && echo pass || echo fail)" "${FAIRNESS_RESULT}" "fairness wrapper completed"

  if [[ "${NEEDS_CROSS_PROCESS}" == "true" ]]; then
    emit_event "cross_process" "start" "running" "running cross-process wrapper"
    set +e
    (
      cd "${WORKSPACE_ROOT}"
      bash "${CROSS_PROCESS_SCRIPT}" --json
    ) >"${CROSS_JSON}" 2>"${CROSS_LOG}"
    CROSS_STATUS=$?
    set -e
    if [[ -f "${CROSS_JSON}" ]]; then
      CROSS_RESULT="$(jq -r '.result // "fail"' "${CROSS_JSON}" 2>/dev/null || echo fail)"
      CROSS_REPORT_PATH="$(jq -r '.artifact_bundle.report_path // ""' "${CROSS_JSON}" 2>/dev/null || true)"
    else
      CROSS_RESULT="fail"
    fi
    emit_event "cross_process" "$([[ ${CROSS_STATUS} -eq 0 ]] && echo pass || echo fail)" "${CROSS_RESULT}" "cross-process wrapper completed"
  else
    emit_event "cross_process" "info" "skipped" "${CROSS_PROCESS_REASON}"
  fi
else
  emit_event "crash_matrix" "info" "dry_run" "skipped wrapper execution in dry-run mode"
  emit_event "fairness" "info" "dry_run" "skipped wrapper execution in dry-run mode"
  emit_event "cross_process" "info" "skipped" "${CROSS_PROCESS_REASON}"
fi

if [[ "${DRY_RUN}" == "1" ]]; then
  RESULT="dry_run"
elif [[ "${PERSISTENT_VERDICT}" == "pass" && "${CRASH_RESULT}" == "pass" && "${FAIRNESS_RESULT}" == "pass" && ( "${CROSS_RESULT}" == "pass" || "${CROSS_RESULT}" == "skipped" ) ]]; then
  RESULT="pass"
else
  RESULT="fail"
fi

(
  cd "${ARTIFACT_DIR}"
  find . -type f ! -name "$(basename "${HASHES_TXT}")" -print0 \
    | sort -z \
    | xargs -0 sha256sum > "$(basename "${HASHES_TXT}")"
)

jq -n \
  --arg trace_id "${TRACE_ID}" \
  --arg run_id "${RUN_ID}" \
  --arg scenario_id "${SCENARIO_ID}" \
  --arg seed "${SEED}" \
  --arg bead_id "${BEAD_ID}" \
  --arg result "${RESULT}" \
  --arg persistent_baseline_dir "${PERSISTENT_BASELINE_DIR}" \
  --arg persistent_scorecard_json "${PERSISTENT_SCORECARD_JSON}" \
  --arg persistent_summary_md "${PERSISTENT_SUMMARY_MD}" \
  --arg persistent_rerun_sh "${PERSISTENT_RERUN_SH}" \
  --arg persistent_verdict "${PERSISTENT_VERDICT}" \
  --arg persistent_8t_verdict "${PERSISTENT_8T_VERDICT}" \
  --arg persistent_16t_verdict "${PERSISTENT_16T_VERDICT}" \
  --arg persistent_8t_ratio "${PERSISTENT_8T_RATIO}" \
  --arg persistent_16t_ratio "${PERSISTENT_16T_RATIO}" \
  --arg crash_result "${CRASH_RESULT}" \
  --arg crash_report_path "${CRASH_REPORT_PATH}" \
  --arg fairness_result "${FAIRNESS_RESULT}" \
  --arg fairness_report_path "${FAIRNESS_REPORT_PATH}" \
  --arg cross_result "${CROSS_RESULT}" \
  --arg cross_report_path "${CROSS_REPORT_PATH}" \
  --arg needs_cross_process "${NEEDS_CROSS_PROCESS}" \
  --arg cross_process_reason "${CROSS_PROCESS_REASON}" \
  --arg events_jsonl "${EVENTS_JSONL}" \
  --arg manifest_json "${MANIFEST_JSON}" \
  --arg summary_md "${SUMMARY_MD}" \
  --arg hashes_txt "${HASHES_TXT}" \
  '{
    trace_id: $trace_id,
    run_id: $run_id,
    scenario_id: $scenario_id,
    seed: $seed,
    bead_id: $bead_id,
    result: $result,
    persistent_baseline: {
      dir: $persistent_baseline_dir,
      scorecard_json: $persistent_scorecard_json,
      summary_md: $persistent_summary_md,
      rerun_sh: (if $persistent_rerun_sh == ($persistent_baseline_dir + "/rerun.sh") then $persistent_rerun_sh else $persistent_rerun_sh end),
      verdict: $persistent_verdict,
      critical_regimes: {
        "8t": { verdict: $persistent_8t_verdict, throughput_ratio_vs_sqlite: $persistent_8t_ratio },
        "16t": { verdict: $persistent_16t_verdict, throughput_ratio_vs_sqlite: $persistent_16t_ratio }
      }
    },
    checks: {
      persistent_baseline: {
        result: (if $persistent_verdict == "pass" then "pass" else "fail" end)
      },
      crash_matrix: {
        result: $crash_result,
        report_path: (if ($crash_report_path | length) > 0 then $crash_report_path else null end)
      },
      fairness: {
        result: $fairness_result,
        report_path: (if ($fairness_report_path | length) > 0 then $fairness_report_path else null end)
      },
      cross_process: {
        result: $cross_result,
        report_path: (if ($cross_report_path | length) > 0 then $cross_report_path else null end),
        required: ($needs_cross_process == "true"),
        reason: $cross_process_reason
      }
    },
    artifacts: {
      events_jsonl: $events_jsonl,
      manifest_json: $manifest_json,
      summary_md: $summary_md,
      hashes_txt: $hashes_txt
    },
    replay_command: "bash scripts/verify_bd_db300_3_8_10_commit_path_proof_pack.sh"
  }' > "${REPORT_JSON}"

cat > "${MANIFEST_JSON}" <<EOF_MANIFEST
{
  "trace_id": "${TRACE_ID}",
  "run_id": "${RUN_ID}",
  "scenario_id": "${SCENARIO_ID}",
  "seed": ${SEED},
  "bead_id": "${BEAD_ID}",
  "artifacts": [
    {"path":"${EVENTS_JSONL}"},
    {"path":"${REPORT_JSON}"},
    {"path":"${SUMMARY_MD}"},
    {"path":"${HASHES_TXT}"},
    {"path":"${PERSISTENT_SCORECARD_JSON}"},
    {"path":"${PERSISTENT_SUMMARY_MD}"}
  ]
}
EOF_MANIFEST

cat > "${SUMMARY_MD}" <<EOF_SUMMARY
# ${BEAD_ID} Commit-Path Proof Pack

- run_id: \`${RUN_ID}\`
- trace_id: \`${TRACE_ID}\`
- scenario_id: \`${SCENARIO_ID}\`
- result: \`${RESULT}\`
- persistent baseline of record: \`${PERSISTENT_BASELINE_DIR}\`

## Persistent Baseline

- verdict: \`${PERSISTENT_VERDICT}\`
- 8t: \`${PERSISTENT_8T_VERDICT}\` at \`${PERSISTENT_8T_RATIO}x\`
- 16t: \`${PERSISTENT_16T_VERDICT}\` at \`${PERSISTENT_16T_RATIO}x\`
- summary: \`${PERSISTENT_SUMMARY_MD}\`
- scorecard: \`${PERSISTENT_SCORECARD_JSON}\`

## Proof Surfaces

- crash/corruption matrix wrapper: \`${CRASH_RESULT}\`${CRASH_REPORT_PATH:+ at \`${CRASH_REPORT_PATH}\`}
- concurrent-writer fairness wrapper: \`${FAIRNESS_RESULT}\`${FAIRNESS_REPORT_PATH:+ at \`${FAIRNESS_REPORT_PATH}\`}
- cross-process wrapper: \`${CROSS_RESULT}\`${CROSS_REPORT_PATH:+ at \`${CROSS_REPORT_PATH}\`}
- cross-process policy: ${CROSS_PROCESS_REASON}

## Notes

- This pack is orchestration-only. It reuses the fixed persistent pack of record plus existing child verification scripts and tests.
- It does not rerun the persistent benchmark itself; it treats \`${PERSISTENT_BASELINE_DIR}\` as the persistent baseline of record for bd-db300.3.8.10.
- A red persistent baseline keeps this pack non-green even when the crash/fairness proofs pass.
- Dry-run mode validates the baseline contract and script wiring without running child wrappers.
EOF_SUMMARY

emit_event "finalize" "info" "${RESULT}" "report written to ${REPORT_JSON}"

if [[ "${JSON_OUTPUT}" == "true" ]]; then
  cat "${REPORT_JSON}"
else
  echo "Result:                 ${RESULT}"
  echo "Persistent baseline:    ${PERSISTENT_BASELINE_DIR}"
  echo "Persistent verdict:     ${PERSISTENT_VERDICT}"
  echo "Crash matrix result:    ${CRASH_RESULT}"
  echo "Fairness result:        ${FAIRNESS_RESULT}"
  echo "Cross-process result:   ${CROSS_RESULT}"
  echo "Report:                 ${REPORT_JSON}"
  echo "Summary:                ${SUMMARY_MD}"
fi

[[ "${RESULT}" == "pass" || "${RESULT}" == "dry_run" ]]
