#!/usr/bin/env bash
# verify_pool_advisor.sh — deterministic verifier for bd-t6sv2.10
set -euo pipefail

BEAD_ID="bd-t6sv2.10"
SCHEMA_VERSION="2"
RUN_ID="${BEAD_ID}-$(date -u +%Y%m%dT%H%M%SZ)-$$"
REPORT_DIR="test-results"
REPORT_FILE="${REPORT_DIR}/${BEAD_ID}-pool-advisor-verify.json"
JSON_MODE=false
NO_RCH=false

for arg in "$@"; do
  case "$arg" in
    --json) JSON_MODE=true ;;
    --no-rch) NO_RCH=true ;;
    *)
      echo "ERROR: unknown argument '$arg'" >&2
      exit 2
      ;;
  esac
done

mkdir -p "$REPORT_DIR"

run_build() {
  if ! $NO_RCH && command -v rch >/dev/null 2>&1; then
    rch exec -- "$@"
  else
    "$@"
  fi
}

parse_passed() {
  echo "$1" | grep 'test result:' | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' | tail -1 || echo "0"
}

parse_failed() {
  echo "$1" | grep 'test result:' | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+' | tail -1 || echo "0"
}

declare -A CASE_STATUS
declare -A CASE_PASSED
declare -A CASE_FAILED
declare -A CASE_EXIT
declare -A CASE_DURATION_MS
declare -A CASE_COMMAND
declare -a CASE_IDS
declare -a SCENARIO_CASE_IDS

run_scenario_case() {
  local case_id="$1"
  SCENARIO_CASE_IDS+=("$case_id")
  run_case "$@"
}

run_case() {
  local case_id="$1"
  shift

  CASE_IDS+=("$case_id")
  CASE_COMMAND["$case_id"]="$*"
  echo "phase=${case_id} bead_id=${BEAD_ID} run_id=${RUN_ID}"

  local output
  local exit_code
  local started_ns
  local finished_ns
  local elapsed_ms
  started_ns=$(date +%s%N)
  set +e
  output=$("$@" 2>&1)
  exit_code=$?
  set -e
  finished_ns=$(date +%s%N)
  elapsed_ms=$(((finished_ns - started_ns) / 1000000))

  local passed
  local failed
  passed=$(parse_passed "$output")
  failed=$(parse_failed "$output")
  if [[ "$passed" == "0" && "$failed" == "0" && "$exit_code" -eq 0 ]]; then
    passed=1
  fi

  CASE_PASSED["$case_id"]="$passed"
  CASE_FAILED["$case_id"]="$failed"
  CASE_EXIT["$case_id"]="$exit_code"
  CASE_DURATION_MS["$case_id"]="$elapsed_ms"

  if [ "$exit_code" -eq 0 ] && [ "$failed" -eq 0 ]; then
    CASE_STATUS["$case_id"]="pass"
  else
    CASE_STATUS["$case_id"]="fail"
  fi
}

run_case \
  "validator_suite" \
  run_build cargo test -p fsqlite-observability connection_pool::tests:: -- --nocapture

run_scenario_case \
  "scenario_single_conn" \
  run_build cargo test -p fsqlite-observability test_single_connection_serialization_is_detected -- --exact --nocapture

run_scenario_case \
  "scenario_over_pool" \
  run_build cargo test -p fsqlite-observability test_over_pooling_is_detected_when_parallelism_is_low -- --exact --nocapture

run_scenario_case \
  "scenario_stale_snapshot" \
  run_build cargo test -p fsqlite-observability test_stale_idle_snapshot_holders_are_flagged -- --exact --nocapture

run_scenario_case \
  "scenario_thrash" \
  run_build cargo test -p fsqlite-observability test_connection_thrashing_is_detected -- --exact --nocapture

run_scenario_case \
  "scenario_hot_loop" \
  run_build cargo test -p fsqlite-observability test_unprepared_hot_loop_is_detected -- --exact --nocapture

run_scenario_case \
  "scenario_recommendation_bounds" \
  run_build cargo test -p fsqlite-observability test_recommendation_is_bounded_by_cpu_and_grows_with_writer_need -- --exact --nocapture

run_scenario_case \
  "scenario_validation_json" \
  run_build cargo test -p fsqlite-observability test_validation_report_serializes_auditable_fields -- --exact --nocapture

run_scenario_case \
  "scenario_simulation_json" \
  run_build cargo test -p fsqlite-observability test_simulation_report_serializes_candidate_metrics -- --exact --nocapture

run_scenario_case \
  "scenario_docs_validator" \
  run_build cargo test -p fsqlite-observability test_docs_validator_example_matches_exported_api -- --exact --nocapture

run_scenario_case \
  "scenario_docs_simulator" \
  run_build cargo test -p fsqlite-observability test_docs_simulator_example_matches_exported_api -- --exact --nocapture

run_scenario_case \
  "scenario_simulation_stability" \
  run_build cargo test -p fsqlite-observability test_simulator_is_stable_across_runs -- --exact --nocapture

run_case \
  "doctest_suite" \
  run_build cargo test -p fsqlite-observability --doc -- --nocapture

run_case \
  "clippy_observability" \
  run_build cargo clippy -p fsqlite-observability --all-targets --no-deps -- -D warnings

run_case \
  "core_pragma_suite" \
  run_build cargo test -p fsqlite-core connection_stats -- --nocapture

run_case \
  "core_pragma_lifecycle" \
  run_build cargo test -p fsqlite-core test_pragma_connection_stats_reports_shared_pool_lifecycle -- --exact --nocapture

run_case \
  "core_pragma_disconnects" \
  run_build cargo test -p fsqlite-core test_pragma_connection_stats_tracks_active_transactions_and_disconnects -- --exact --nocapture

run_case \
  "core_pragma_failed_attempts" \
  run_build cargo test -p fsqlite-core test_pragma_connection_stats_excludes_failed_statement_attempts -- --exact --nocapture

run_case \
  "docs_contract" \
  bash -lc \
  "rg -q 'multiple writer connections' docs/connection-pooling.md \
    && rg -q 'simulate_connection_pool' docs/connection-pooling.md \
    && rg -q 'validate_connection_pool' docs/connection-pooling.md \
    && rg -q 'PRAGMA fsqlite\\.connection_stats' docs/connection-pooling.md \
    && rg -q 'sqlx::Pool|sqlx' docs/connection-pooling.md \
    && rg -q 'r2d2' docs/connection-pooling.md \
    && rg -q 'deadpool' docs/connection-pooling.md \
    && rg -q 'bb8' docs/connection-pooling.md"

TOTAL_PASSED=0
TOTAL_FAILED=0
TOTAL_CASES=0
PASSED_CASES=0
VERDICT="pass"
for case_id in "${CASE_IDS[@]}"; do
  TOTAL_PASSED=$((TOTAL_PASSED + CASE_PASSED["$case_id"]))
  TOTAL_FAILED=$((TOTAL_FAILED + CASE_FAILED["$case_id"]))
  TOTAL_CASES=$((TOTAL_CASES + 1))
  if [ "${CASE_STATUS["$case_id"]}" = "pass" ]; then
    PASSED_CASES=$((PASSED_CASES + 1))
  else
    VERDICT="fail"
  fi
done

SCENARIO_TOTAL_CASES=0
SCENARIO_PASSED_CASES=0
for case_id in "${SCENARIO_CASE_IDS[@]}"; do
  SCENARIO_TOTAL_CASES=$((SCENARIO_TOTAL_CASES + 1))
  if [ "${CASE_STATUS["$case_id"]}" = "pass" ]; then
    SCENARIO_PASSED_CASES=$((SCENARIO_PASSED_CASES + 1))
  fi
done

if [ "$SCENARIO_TOTAL_CASES" -eq 0 ]; then
  RECOMMENDATION_ACCURACY_PCT=0
else
  RECOMMENDATION_ACCURACY_PCT=$((SCENARIO_PASSED_CASES * 100 / SCENARIO_TOTAL_CASES))
fi

CASES_JSON=""
for case_id in "${CASE_IDS[@]}"; do
  if [ -n "$CASES_JSON" ]; then
    CASES_JSON="${CASES_JSON},"
  fi
  CASES_JSON="${CASES_JSON}
    \"${case_id}\": {
      \"status\": \"${CASE_STATUS["$case_id"]}\",
      \"command\": \"${CASE_COMMAND["$case_id"]}\",
      \"exit_code\": ${CASE_EXIT["$case_id"]},
      \"duration_ms\": ${CASE_DURATION_MS["$case_id"]},
      \"passed\": ${CASE_PASSED["$case_id"]},
      \"failed\": ${CASE_FAILED["$case_id"]}
    }"
done

SCENARIO_CASES_JSON=""
for case_id in "${SCENARIO_CASE_IDS[@]}"; do
  if [ -n "$SCENARIO_CASES_JSON" ]; then
    SCENARIO_CASES_JSON="${SCENARIO_CASES_JSON}, "
  fi
  SCENARIO_CASES_JSON="${SCENARIO_CASES_JSON}\"${case_id}\""
done

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
REPORT_CONTENT=$(cat <<ENDJSON
{
  "schema_version": ${SCHEMA_VERSION},
  "bead_id": "${BEAD_ID}",
  "run_id": "${RUN_ID}",
  "timestamp": "${TIMESTAMP}",
  "verdict": "${VERDICT}",
  "recommendation_accuracy_pct": ${RECOMMENDATION_ACCURACY_PCT},
  "scenario_cases": [${SCENARIO_CASES_JSON}],
  "cases": {
${CASES_JSON}
  },
  "totals": {
    "passed": ${TOTAL_PASSED},
    "failed": ${TOTAL_FAILED},
    "passed_cases": ${PASSED_CASES},
    "total_cases": ${TOTAL_CASES},
    "scenario_passed_cases": ${SCENARIO_PASSED_CASES},
    "scenario_total_cases": ${SCENARIO_TOTAL_CASES}
  }
}
ENDJSON
)

echo "$REPORT_CONTENT" > "$REPORT_FILE"
REPORT_SHA=$(sha256sum "$REPORT_FILE" | cut -d' ' -f1)

if $JSON_MODE; then
  echo "$REPORT_CONTENT"
else
  echo "phase=complete bead_id=${BEAD_ID} run_id=${RUN_ID} verdict=${VERDICT}"
  for case_id in "${CASE_IDS[@]}"; do
    echo "  ${case_id}: ${CASE_STATUS["$case_id"]}"
  done
  echo "  recommendation_accuracy_pct=${RECOMMENDATION_ACCURACY_PCT}"
  echo "  scenario_cases=${SCENARIO_PASSED_CASES}/${SCENARIO_TOTAL_CASES}"
  echo "  report_path=${REPORT_FILE}"
  echo "  report_sha256=${REPORT_SHA}"
fi

if [ "$VERDICT" = "pass" ]; then
  exit 0
fi
exit 1
