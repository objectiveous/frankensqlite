#!/usr/bin/env bash
# verify_bd_db300_6_1_2_first_hit_warmed_hit.sh
#
# E2E verification script for bd-db300.6.1.2:
# "F1.2: measure first-hit warmed-hit and background-compile interference"
#
# Tests:
# - First-hit (cache miss) statements incur compile overhead
# - Warmed-hit (cache hit) statements reuse compiled plans
# - Compile overhead is captured in hot-path profile
#
# Usage:
#   ./scripts/verify_bd_db300_6_1_2_first_hit_warmed_hit.sh
#
# Environment variables:
#   FSQLITE_VERBOSE=1  - Show full test output
#   FSQLITE_TRACE=1    - Enable trace-level logging

set -euo pipefail

BEAD_ID="bd-db300.6.1.2"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_NAME="bd_db300_6_1_2_first_hit_warmed_hit"
ARTIFACT_DIR="${PROJECT_ROOT}/target/verification/${BEAD_ID}"

cd "$PROJECT_ROOT"

echo "=== ${BEAD_ID}: First-Hit vs Warmed-Hit E2E Verification ==="
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
    "title": "F1.2: measure first-hit warmed-hit and background-compile interference",
    "status": "PASS",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "tests": [
        "bd_db300_6_1_2_first_statement_is_first_hit",
        "bd_db300_6_1_2_repeated_statement_is_warmed_hit",
        "bd_db300_6_1_2_first_hit_has_compile_overhead",
        "bd_db300_6_1_2_warmed_hit_faster_than_first_hit",
        "bd_db300_6_1_2_profile_reset_clears_hit_metrics",
        "bd_db300_6_1_2_compile_cache_consistency"
    ],
    "acceptance_criteria": {
        "first_hit_latency_measured": true,
        "warmed_hit_latency_measured": true,
        "compile_overhead_captured": true,
        "grounded_in_fixtures": true
    },
    "replay_command": "cargo test -p fsqlite-e2e --test bd_db300_6_1_2_first_hit_warmed_hit -- --nocapture --test-threads=1",
    "artifact_path": "${ARTIFACT_DIR}"
}
EOF

echo ""
echo "Summary written to: $SUMMARY_FILE"
echo ""
echo "=== Verification Complete ==="
