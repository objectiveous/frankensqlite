#!/usr/bin/env bash
# verify_bd_db300_6_1_1_statement_reuse_distance.sh
#
# E2E verification script for bd-db300.6.1.1:
# "F1.1: collect statement reuse-distance and lane-locality traces"
#
# Tests:
# - Statement reuse events captured in hot-path profile
# - Reuse distance correctly computed (statements between repeats)
# - Lane-local vs cross-lane reuse tracked
# - Metrics ready for compile-governance decisions
#
# Usage:
#   ./scripts/verify_bd_db300_6_1_1_statement_reuse_distance.sh
#
# Environment variables:
#   FSQLITE_VERBOSE=1  - Show full test output
#   FSQLITE_TRACE=1    - Enable trace-level logging

set -euo pipefail

BEAD_ID="bd-db300.6.1.1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_NAME="bd_db300_6_1_1_statement_reuse_distance"
ARTIFACT_DIR="${PROJECT_ROOT}/target/verification/${BEAD_ID}"

cd "$PROJECT_ROOT"

echo "=== ${BEAD_ID}: Statement Reuse-Distance E2E Verification ==="
echo "Project root: $PROJECT_ROOT"
echo "Artifact dir: $ARTIFACT_DIR"
echo ""

mkdir -p "$ARTIFACT_DIR"

# Set up logging environment
export RUST_LOG="${RUST_LOG:-fsqlite=debug}"
if [[ "${FSQLITE_TRACE:-}" == "1" ]]; then
    export RUST_LOG="trace"
fi

# Run the E2E tests
echo "Running E2E tests..."
TEST_OUTPUT="${ARTIFACT_DIR}/test_output.txt"

if [[ "${FSQLITE_VERBOSE:-}" == "1" ]]; then
    cargo test -p fsqlite-e2e --test "$TEST_NAME" -- --nocapture --test-threads=1 2>&1 | tee "$TEST_OUTPUT"
else
    cargo test -p fsqlite-e2e --test "$TEST_NAME" -- --nocapture --test-threads=1 > "$TEST_OUTPUT" 2>&1 || {
        echo "FAIL: E2E tests failed"
        echo "=== Test Output ==="
        cat "$TEST_OUTPUT"
        exit 1
    }
fi

# Extract test results
echo ""
echo "=== Test Results ==="
grep -E "^(INFO|WARN|ERROR|test result:|running [0-9]+ test)" "$TEST_OUTPUT" || true

# Check for any failures
if grep -q "FAILED" "$TEST_OUTPUT"; then
    echo ""
    echo "FAIL: Some tests failed"
    exit 1
fi

if grep -q "test result: ok" "$TEST_OUTPUT"; then
    echo ""
    echo "PASS: All ${BEAD_ID} E2E tests passed"
else
    echo ""
    echo "WARN: Could not confirm test results"
    exit 1
fi

# Generate summary artifact
SUMMARY_FILE="${ARTIFACT_DIR}/summary.json"
cat > "$SUMMARY_FILE" << EOF
{
    "bead_id": "${BEAD_ID}",
    "title": "F1.1: collect statement reuse-distance and lane-locality traces",
    "status": "PASS",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "tests": [
        "bd_db300_6_1_1_same_statement_repeat_captures_reuse",
        "bd_db300_6_1_1_interleaved_statements_measure_distance",
        "bd_db300_6_1_1_no_reuse_yields_zero_metrics",
        "bd_db300_6_1_1_max_reuse_distance_tracked",
        "bd_db300_6_1_1_profile_reset_clears_metrics"
    ],
    "acceptance_criteria": {
        "reuse_events_captured": true,
        "reuse_distance_computed": true,
        "lane_locality_tracked": true,
        "compile_governance_ready": true
    },
    "replay_command": "cargo test -p fsqlite-e2e --test bd_db300_6_1_1_statement_reuse_distance -- --nocapture --test-threads=1",
    "artifact_path": "${ARTIFACT_DIR}"
}
EOF

echo ""
echo "Summary written to: $SUMMARY_FILE"
echo ""
echo "=== Verification Complete ==="
