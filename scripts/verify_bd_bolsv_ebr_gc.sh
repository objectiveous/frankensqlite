#!/usr/bin/env bash
set -euo pipefail

BEAD_ID="bd-bolsv"
SCRIPT_NAME="$(basename "$0")"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${BEAD_ID}-${TIMESTAMP}-ebr-gc"
ARTIFACT_DIR="artifacts/${BEAD_ID}/${RUN_ID}"
mkdir -p "${ARTIFACT_DIR}"

CHECK_LOG="${ARTIFACT_DIR}/cargo-check.log"
TEST_GUARD_LOG="${ARTIFACT_DIR}/test_try_recycle_retired_slots_waits_for_active_guard_release.log"
TEST_REUSE_LOG="${ARTIFACT_DIR}/test_gc_tick_recycles_retired_slot_for_next_publish_after_guard_release.log"

CHECK_CMD=(
  cargo check -p fsqlite-mvcc --all-targets
)
TEST_GUARD_CMD=(
  cargo test -p fsqlite-mvcc test_try_recycle_retired_slots_waits_for_active_guard_release -- --nocapture
)
TEST_REUSE_CMD=(
  cargo test -p fsqlite-mvcc test_gc_tick_recycles_retired_slot_for_next_publish_after_guard_release -- --nocapture
)

echo "Running: ${CHECK_CMD[*]}"
"${CHECK_CMD[@]}" 2>&1 | tee "${CHECK_LOG}"

echo "Running: ${TEST_GUARD_CMD[*]}"
"${TEST_GUARD_CMD[@]}" 2>&1 | tee "${TEST_GUARD_LOG}"

echo "Running: ${TEST_REUSE_CMD[*]}"
"${TEST_REUSE_CMD[@]}" 2>&1 | tee "${TEST_REUSE_LOG}"

cat > "${ARTIFACT_DIR}/summary.md" <<EOF
# ${BEAD_ID} Verification Summary

- script: \`${SCRIPT_NAME}\`
- run_id: \`${RUN_ID}\`
- artifact_dir: \`${ARTIFACT_DIR}\`
- compile_check: PASS
- test_try_recycle_retired_slots_waits_for_active_guard_release: PASS
- test_gc_tick_recycles_retired_slot_for_next_publish_after_guard_release: PASS

## Commands

\`\`\`bash
${CHECK_CMD[*]}
${TEST_GUARD_CMD[*]}
${TEST_REUSE_CMD[*]}
\`\`\`

## Logs

- \`${CHECK_LOG}\`
- \`${TEST_GUARD_LOG}\`
- \`${TEST_REUSE_LOG}\`
EOF

cat > "${ARTIFACT_DIR}/report.json" <<EOF
{
  "bead_id": "${BEAD_ID}",
  "run_id": "${RUN_ID}",
  "script": "${SCRIPT_NAME}",
  "commands": [
    "${CHECK_CMD[*]}",
    "${TEST_GUARD_CMD[*]}",
    "${TEST_REUSE_CMD[*]}"
  ],
  "results": {
    "cargo_check": "pass",
    "test_try_recycle_retired_slots_waits_for_active_guard_release": "pass",
    "test_gc_tick_recycles_retired_slot_for_next_publish_after_guard_release": "pass"
  },
  "logs": {
    "cargo_check": "${CHECK_LOG}",
    "test_try_recycle_retired_slots_waits_for_active_guard_release": "${TEST_GUARD_LOG}",
    "test_gc_tick_recycles_retired_slot_for_next_publish_after_guard_release": "${TEST_REUSE_LOG}"
  }
}
EOF

echo "Artifacts written to ${ARTIFACT_DIR}"
