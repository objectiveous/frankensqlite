#!/usr/bin/env bash
set -euo pipefail

BEAD_ID="bd-1dp9.6.7.9.4"
RUN_ID="${BEAD_ID}-$(date -u +%Y%m%dT%H%M%SZ)"
ARTIFACT_DIR="artifacts/${BEAD_ID}/${RUN_ID}"
TARGET_DIR="${TMPDIR:-/tmp}/rch_target_bd_1dp9_6_7_9_4"
POLICY_LOG="${ARTIFACT_DIR}/commit_service_policy_tests.log"
CHECKPOINT_LOG="${ARTIFACT_DIR}/checkpoint_coordination_tests.log"
SHA_LOG="${ARTIFACT_DIR}/sha256.txt"
REPORT_JSON="${ARTIFACT_DIR}/gate_report.json"

mkdir -p "${ARTIFACT_DIR}"

policy_cmd=(
  rch exec -- env "CARGO_TARGET_DIR=${TARGET_DIR}"
  cargo test -p fsqlite-pager commit_service_policy_ -- --nocapture
)
checkpoint_cmd=(
  rch exec -- env "CARGO_TARGET_DIR=${TARGET_DIR}"
  cargo test -p fsqlite-pager checkpoint_coordination -- --nocapture
)

printf '+'
printf ' %q' "${policy_cmd[@]}"
printf '\n'
"${policy_cmd[@]}" 2>&1 | tee "${POLICY_LOG}"

printf '+'
printf ' %q' "${checkpoint_cmd[@]}"
printf '\n'
"${checkpoint_cmd[@]}" 2>&1 | tee "${CHECKPOINT_LOG}"

rg -q 'control_epoch=' "${POLICY_LOG}"
rg -q 'target_wait_ns=' "${POLICY_LOG}"
rg -q 'actual_wait_ns=' "${POLICY_LOG}"
rg -q 'queue_age_p95_ns=' "${POLICY_LOG}"
rg -q 'batch_size=' "${POLICY_LOG}"
rg -q 'fairness_budget_ns=' "${POLICY_LOG}"
rg -q 'mode_switch_reason=' "${POLICY_LOG}"
rg -q 'mode_switch_reason="sparse_queue".*service_policy_mode="low_latency"|service_policy_mode="low_latency".*mode_switch_reason="sparse_queue"' "${POLICY_LOG}"
rg -q 'mode_switch_reason="sparse_queue".*starvation_prevented=false|starvation_prevented=false.*mode_switch_reason="sparse_queue"' "${POLICY_LOG}"
rg -q 'mode_switch_reason="tail_latency_pressure".*service_policy_mode="low_latency"|service_policy_mode="low_latency".*mode_switch_reason="tail_latency_pressure"' "${POLICY_LOG}"
rg -q 'mode_switch_reason="tail_latency_pressure".*starvation_prevented=true|starvation_prevented=true.*mode_switch_reason="tail_latency_pressure"' "${POLICY_LOG}"
rg -q 'checkpoint_phase="active_gate".*foreground_phase="begin".*interaction_rule="checkpoint_excludes_new_transactions"' "${CHECKPOINT_LOG}"
rg -q 'checkpoint_phase="backend_owned".*foreground_phase="foreground_idle".*foreground_action="checkpoint_runs"' "${CHECKPOINT_LOG}"

sha256sum "${POLICY_LOG}" "${CHECKPOINT_LOG}" | tee "${SHA_LOG}" >/dev/null

jq -n \
  --arg bead_id "${BEAD_ID}" \
  --arg run_id "${RUN_ID}" \
  --arg policy_log "${POLICY_LOG}" \
  --arg checkpoint_log "${CHECKPOINT_LOG}" \
  --arg policy_cmd "${policy_cmd[*]}" \
  --arg checkpoint_cmd "${checkpoint_cmd[*]}" \
  --arg sha_log "${SHA_LOG}" \
  '{
    bead_id: $bead_id,
    run_id: $run_id,
    policy_log: $policy_log,
    checkpoint_log: $checkpoint_log,
    replay_commands: {
      commit_service_policy: $policy_cmd,
      checkpoint_coordination: $checkpoint_cmd
    },
    artifacts: {
      sha256: $sha_log
    },
    assertions: [
      "service-policy traces emit control epoch, bounded waits, queue-age p95, fairness budget, batch size, and switch reasons",
      "sparse queue traffic stays in low-latency mode without tripping starvation prevention",
      "tail-latency pressure flips the controller into starvation prevention with zero additional wait",
      "checkpoint coordination still excludes new foreground transactions while the service policy is active",
      "checkpoint ownership and foreground-idle transitions remain visible in structured logs"
    ]
  }' > "${REPORT_JSON}"

echo "[GATE PASS] ${BEAD_ID} commit service policy verification passed"
echo "Artifacts: ${ARTIFACT_DIR}"
