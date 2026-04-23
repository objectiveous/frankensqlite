#!/usr/bin/env bash
set -euo pipefail

BEAD_ID="bd-1dp9.6.7.3.4"
RUN_ID="${BEAD_ID}-$(date -u +%Y%m%dT%H%M%SZ)-$$"
ARTIFACT_DIR="artifacts/${BEAD_ID}/${RUN_ID}"
TARGET_DIR="${TMPDIR:-/tmp}/rch_target_${BEAD_ID//\./_}"
PLANNER_LOG="${ARTIFACT_DIR}/planner_probe_tests.log"
CODEGEN_LOG="${ARTIFACT_DIR}/codegen_directive_tests.log"
EQP_LOG="${ARTIFACT_DIR}/eqp_detail_tests.log"
SHA_LOG="${ARTIFACT_DIR}/sha256.txt"
REPORT_JSON="${ARTIFACT_DIR}/gate_report.json"

mkdir -p "${ARTIFACT_DIR}"

# --- Layer 1: Planner probe extraction tests ---
planner_cmd=(
  rch exec -- env "CARGO_TARGET_DIR=${TARGET_DIR}"
  cargo test -p fsqlite-planner extract_probe -- --nocapture
)
printf '+ '; printf ' %q' "${planner_cmd[@]}"; printf '\n'
"${planner_cmd[@]}" 2>&1 | tee "${PLANNER_LOG}"

# --- Layer 2: VDBE codegen directive honor/bypass tests ---
codegen_cmd=(
  rch exec -- env "CARGO_TARGET_DIR=${TARGET_DIR}"
  cargo test -p fsqlite-vdbe --lib codegen_select -- --nocapture
)
printf '+ '; printf ' %q' "${codegen_cmd[@]}"; printf '\n'
"${codegen_cmd[@]}" 2>&1 | tee "${CODEGEN_LOG}"

# --- Layer 3: EQP detail integration tests ---
eqp_cmd=(
  rch exec -- env "CARGO_TARGET_DIR=${TARGET_DIR}"
  cargo test -p fsqlite-core --lib explain_query_plan_reports -- --nocapture
)
printf '+ '; printf ' %q' "${eqp_cmd[@]}"; printf '\n'
"${eqp_cmd[@]}" 2>&1 | tee "${EQP_LOG}"

# --- Assertions ---
grep -q 'test result: ok' "${PLANNER_LOG}"
grep -q 'test result: ok' "${CODEGEN_LOG}"
grep -q 'test result: ok' "${EQP_LOG}"
grep -q 'extract_probe_rowid_equality' "${PLANNER_LOG}"
grep -q 'extract_probe_index_equality' "${PLANNER_LOG}"
grep -q 'extract_probe_index_range' "${PLANNER_LOG}"
grep -q 'extract_probe_in_list' "${PLANNER_LOG}"
grep -q 'test_codegen_select_honors_planner_full_scan_directive' "${CODEGEN_LOG}"
grep -q 'test_codegen_select_bypasses_stale_planner_rowid_directive' "${CODEGEN_LOG}"
grep -q 'test_explain_query_plan_reports_planner_selected_detail' "${EQP_LOG}"

sha256sum "${PLANNER_LOG}" "${CODEGEN_LOG}" "${EQP_LOG}" | tee "${SHA_LOG}" >/dev/null

jq -n \
  --arg bead_id "${BEAD_ID}" \
  --arg run_id "${RUN_ID}" \
  --arg planner_log "${PLANNER_LOG}" \
  --arg codegen_log "${CODEGEN_LOG}" \
  --arg eqp_log "${EQP_LOG}" \
  --arg planner_cmd "${planner_cmd[*]}" \
  --arg codegen_cmd "${codegen_cmd[*]}" \
  --arg eqp_cmd "${eqp_cmd[*]}" \
  --arg sha_log "${SHA_LOG}" \
  '{
    bead_id: $bead_id,
    run_id: $run_id,
    artifacts: {
      planner_probe: $planner_log,
      codegen_directive: $codegen_log,
      eqp_detail: $eqp_log,
      sha256: $sha_log
    },
    replay_commands: {
      planner_probe: $planner_cmd,
      codegen_directive: $codegen_cmd,
      eqp_detail: $eqp_cmd
    },
    assertions: [
      "planner extract_probe_rowid_equality passes",
      "planner extract_probe_index_equality passes",
      "planner extract_probe_index_range passes",
      "planner extract_probe_in_list passes",
      "codegen honors planner full-scan directive over index probe",
      "codegen bypasses stale planner rowid directive",
      "EQP detail reports planner-selected access path"
    ]
  }' > "${REPORT_JSON}"

echo "[GATE PASS] ${BEAD_ID} physical-plan handoff verification passed"
echo "Artifacts: ${ARTIFACT_DIR}"
