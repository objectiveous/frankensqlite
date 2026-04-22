#!/usr/bin/env bash
# verify_bd_2nzo8_1_3_fts5_baseline.sh — bd-2nzo8.1.3 FTS5 baseline runner
#
# Usage:
#   bash scripts/verify_bd_2nzo8_1_3_fts5_baseline.sh [--json] [--scenario ID] [--seed N] [--perf-iterations N] [--artifact-root PATH]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUN_ID="${RUN_ID:-bd-2nzo8-1-3-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
TRACE_ID="${TRACE_ID:-fts5-baseline-${RUN_ID}}"
SCENARIO_ID="${SCENARIO_ID:-all}"
SEED="${SEED:-2020408013}"
PERF_ITERATIONS="${FSQLITE_FTS5_PERF_ITERATIONS:-3}"
ARTIFACT_ROOT="${FSQLITE_FTS5_ARTIFACT_ROOT:-$REPO_ROOT/artifacts}"
JSON_OUTPUT=false
MAX_ATTEMPTS="${BD_2NZO8_1_3_MAX_ATTEMPTS:-2}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --json)
            JSON_OUTPUT=true
            shift
            ;;
        --scenario)
            shift
            [[ $# -gt 0 ]] || { echo "ERROR: --scenario requires value" >&2; exit 2; }
            SCENARIO_ID="$1"
            shift
            ;;
        --seed)
            shift
            [[ $# -gt 0 ]] || { echo "ERROR: --seed requires value" >&2; exit 2; }
            SEED="$1"
            shift
            ;;
        --perf-iterations)
            shift
            [[ $# -gt 0 ]] || { echo "ERROR: --perf-iterations requires value" >&2; exit 2; }
            PERF_ITERATIONS="$1"
            shift
            ;;
        --artifact-root)
            shift
            [[ $# -gt 0 ]] || { echo "ERROR: --artifact-root requires value" >&2; exit 2; }
            ARTIFACT_ROOT="$1"
            shift
            ;;
        --max-attempts)
            shift
            [[ $# -gt 0 ]] || { echo "ERROR: --max-attempts requires value" >&2; exit 2; }
            MAX_ATTEMPTS="$1"
            shift
            ;;
        *)
            echo "ERROR: unknown argument: $1" >&2
            exit 2
            ;;
    esac
done

if ! [[ "$MAX_ATTEMPTS" =~ ^[1-9][0-9]*$ ]]; then
    echo "ERROR: --max-attempts must be a positive integer" >&2
    exit 2
fi

if ! [[ "$PERF_ITERATIONS" =~ ^[1-9][0-9]*$ ]]; then
    echo "ERROR: --perf-iterations must be a positive integer" >&2
    exit 2
fi

if ! [[ "$SEED" =~ ^[0-9]+$ ]]; then
    echo "ERROR: --seed must be a non-negative integer" >&2
    exit 2
fi

if ! command -v rch >/dev/null 2>&1; then
    echo "ERROR: rch is required for bd-2nzo8.1.3 verification" >&2
    exit 3
fi

VERIFY_OUTPUT_DIR="$REPO_ROOT/test-results/bd_2nzo8_1_3"
mkdir -p "$VERIFY_OUTPUT_DIR"

RESULT="pass"
TEST_LOG=""
ARTIFACT_PATH=""
DIVERGENT_SCENARIO_COUNT=""
ATTEMPT_USED=0

for (( attempt=1; attempt<=MAX_ATTEMPTS; attempt++ )); do
    ATTEMPT_USED="$attempt"
    TEST_LOG="$VERIFY_OUTPUT_DIR/cargo-${RUN_ID}-attempt-${attempt}.log"
    RESULT="pass"

    if ! rch exec -- env \
        CARGO_TARGET_DIR="${TMPDIR:-/tmp}/rch_target_frankensqlite_$(whoami)_fts5_baseline_$$" \
        FSQLITE_FTS5_BASELINE_E2E=1 \
        RUN_ID="$RUN_ID" \
        TRACE_ID="$TRACE_ID" \
        SCENARIO_ID="$SCENARIO_ID" \
        SEED="$SEED" \
        FSQLITE_FTS5_ARTIFACT_ROOT="$ARTIFACT_ROOT" \
        FSQLITE_FTS5_PERF_ITERATIONS="$PERF_ITERATIONS" \
        cargo test -p fsqlite-harness --test bd_2nzo8_1_3_fts5_baseline test_bd_2nzo8_1_3_env_replay_entrypoint -- --nocapture \
        >"$TEST_LOG" 2>&1; then
        RESULT="fail"
    fi

    ARTIFACT_PATH="$({ rg -o 'artifact_path=[^ ]+' "$TEST_LOG" | tail -n1 | sed 's/^artifact_path=//'; } || true)"
    DIVERGENT_SCENARIO_COUNT="$({ rg -o 'divergent_scenario_count=[0-9]+' "$TEST_LOG" | tail -n1 | sed 's/^divergent_scenario_count=//'; } || true)"

    if [[ "$RESULT" == "pass" ]]; then
        break
    fi

    if [[ -n "$ARTIFACT_PATH" ]]; then
        break
    fi

    if (( attempt < MAX_ATTEMPTS )); then
        echo "WARN: bd-2nzo8.1.3 verification failed before artifact emission (attempt $attempt/$MAX_ATTEMPTS), retrying..." >&2
    fi
done

if [[ -z "$ARTIFACT_PATH" ]]; then
    ARTIFACT_PATH="$ARTIFACT_ROOT/bd-2nzo8.1.3/$RUN_ID"
fi

if [[ "$RESULT" == "pass" ]]; then
    for required in events.jsonl manifest.json summary.json artifact_hashes.txt replay.env diff_report.json benchmark_summary.json memory_io_summary.json; do
        if [[ ! -f "$ARTIFACT_PATH/$required" ]]; then
            echo "ERROR: missing required artifact $ARTIFACT_PATH/$required" >&2
            RESULT="fail"
            break
        fi
    done
fi

VERIFY_ARTIFACT_PATH="$VERIFY_OUTPUT_DIR/verify-${RUN_ID}.json"
SUMMARY_HASH=""
if [[ -f "$ARTIFACT_PATH/summary.json" ]]; then
    SUMMARY_HASH="$(sha256sum "$ARTIFACT_PATH/summary.json" | awk '{print $1}')"
fi

cat >"$VERIFY_ARTIFACT_PATH" <<ENDVERIFY
{
  "run_id": "$RUN_ID",
  "trace_id": "$TRACE_ID",
  "bead_id": "bd-2nzo8.1.3",
  "scenario_id": "$SCENARIO_ID",
  "seed": $SEED,
  "perf_iterations": $PERF_ITERATIONS,
  "attempt_used": $ATTEMPT_USED,
  "max_attempts": $MAX_ATTEMPTS,
  "result": "$RESULT",
  "artifact_path": "$ARTIFACT_PATH",
  "summary_hash": "$SUMMARY_HASH",
  "divergent_scenario_count": "${DIVERGENT_SCENARIO_COUNT:-unknown}",
  "test_log_path": "$TEST_LOG"
}
ENDVERIFY

VERIFY_HASH="$(sha256sum "$VERIFY_ARTIFACT_PATH" | awk '{print $1}')"

if [[ "$JSON_OUTPUT" == "true" ]]; then
    cat <<ENDJSON
{
  "run_id": "$RUN_ID",
  "trace_id": "$TRACE_ID",
  "bead_id": "bd-2nzo8.1.3",
  "scenario_id": "$SCENARIO_ID",
  "seed": $SEED,
  "perf_iterations": $PERF_ITERATIONS,
  "attempt_used": $ATTEMPT_USED,
  "max_attempts": $MAX_ATTEMPTS,
  "result": "$RESULT",
  "artifact_path": "$ARTIFACT_PATH",
  "summary_hash": "$SUMMARY_HASH",
  "divergent_scenario_count": "${DIVERGENT_SCENARIO_COUNT:-unknown}",
  "verify_artifact_path": "$VERIFY_ARTIFACT_PATH",
  "verify_artifact_hash": "$VERIFY_HASH",
  "test_log_path": "$TEST_LOG"
}
ENDJSON
else
    echo "=== bd-2nzo8.1.3 FTS5 Baseline Verification ==="
    echo "Run ID:       $RUN_ID"
    echo "Trace ID:     $TRACE_ID"
    echo "Scenario:     $SCENARIO_ID"
    echo "Seed:         $SEED"
    echo "Iterations:   $PERF_ITERATIONS"
    echo "Result:       $RESULT"
    echo "Divergences:  ${DIVERGENT_SCENARIO_COUNT:-unknown}"
    echo "Artifact dir: $ARTIFACT_PATH"
    echo "Summary hash: $SUMMARY_HASH"
    echo "Verify JSON:  $VERIFY_ARTIFACT_PATH"
    echo "Verify hash:  $VERIFY_HASH"
    echo "Cargo log:    $TEST_LOG"
fi

[[ "$RESULT" == "pass" ]]
