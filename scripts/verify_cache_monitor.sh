#!/usr/bin/env bash
# verify_cache_monitor.sh — deterministic verifier for bd-t6sv2.8
set -euo pipefail

BEAD_ID="bd-t6sv2.8"
SCHEMA_VERSION="2"
RUN_ID="${BEAD_ID}-$(date -u +%Y%m%dT%H%M%SZ)-$$"
REPORT_DIR="test-results"
REPORT_FILE="${REPORT_DIR}/${BEAD_ID}-cache-monitor-verify.json"
LOG_DIR="${REPORT_DIR}/${BEAD_ID}-cache-monitor-logs"
SCENARIO_JSONL="${LOG_DIR}/scenario-events.jsonl"
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

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq is required for structured cache-monitor evidence output" >&2
  exit 2
fi

mkdir -p "$REPORT_DIR" "$LOG_DIR"
: >"$SCENARIO_JSONL"

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
declare -A CASE_LOG_PATH
declare -a CASE_IDS
declare -a SCENARIO_CASE_IDS

run_case() {
  local case_id="$1"
  shift

  CASE_IDS+=("$case_id")
  CASE_COMMAND["$case_id"]="$*"
  CASE_LOG_PATH["$case_id"]="${LOG_DIR}/${case_id}.log"

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

  printf '%s\n' "$output" >"${CASE_LOG_PATH["$case_id"]}"

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

  printf '%s\n' "$output" | sed -n 's/^CACHE_MONITOR://p' >>"$SCENARIO_JSONL"
}

run_scenario_case() {
  local case_id="$1"
  SCENARIO_CASE_IDS+=("$case_id")
  run_case "$@"
}

run_case \
  "core_pragma_suite" \
  run_build cargo test -p fsqlite-core test_pragma_cache_ -- --nocapture

run_case \
  "core_virtual_table_suite" \
  run_build cargo test -p fsqlite-core test_fsqlite_cache_pages_table_function_ -- --nocapture

run_case \
  "pager_efficiency_snapshot_suite" \
  run_build cargo test -p fsqlite-pager test_cache_efficiency_snapshot_matches_raw_cache_metrics -- --nocapture

run_scenario_case \
  "pager_sequential_workload" \
  run_build cargo test -p fsqlite-pager test_cache_monitor_sequential_scan_reports_recency_queue_bias -- --nocapture

run_scenario_case \
  "pager_hotset_workload" \
  run_build cargo test -p fsqlite-pager test_cache_monitor_hotset_reports_frequency_queue_bias -- --nocapture

run_scenario_case \
  "pager_snapshot_ranking" \
  run_build cargo test -p fsqlite-pager test_cache_monitor_page_snapshots_rank_hot_pages_by_access_frequency -- --nocapture

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

if [ -s "$SCENARIO_JSONL" ]; then
  SCENARIOS_JSON="$(jq -s '.' "$SCENARIO_JSONL")"
else
  SCENARIOS_JSON='[]'
fi

if ! jq -e '
  length >= 3 and
  all(
    .[];
    .bead_id == "bd-t6sv2.8" and
    (.test_name | type == "string") and
    (.workload_type | type == "string") and
    (.elapsed_ns | type == "number") and
    (.total_accesses | type == "number") and
    (.hit_rate_pct | type == "number") and
    (.eviction_count | type == "number") and
    (.cached_pages | type == "number") and
    (.extra.replay_hint | type == "string")
  )
' >/dev/null <<<"$SCENARIOS_JSON"; then
  VERDICT="fail"
fi

CASES_JSON="$(
  for case_id in "${CASE_IDS[@]}"; do
    jq -n \
      --arg case_id "$case_id" \
      --arg status "${CASE_STATUS["$case_id"]}" \
      --arg command "${CASE_COMMAND["$case_id"]}" \
      --arg log_path "${CASE_LOG_PATH["$case_id"]}" \
      --argjson exit_code "${CASE_EXIT["$case_id"]}" \
      --argjson duration_ms "${CASE_DURATION_MS["$case_id"]}" \
      --argjson passed "${CASE_PASSED["$case_id"]}" \
      --argjson failed "${CASE_FAILED["$case_id"]}" \
      '{($case_id): {
        status: $status,
        command: $command,
        log_path: $log_path,
        exit_code: $exit_code,
        duration_ms: $duration_ms,
        passed: $passed,
        failed: $failed
      }}'
  done | jq -s 'add'
)"

SCENARIO_CASES_JSON="$(
  printf '%s\n' "${SCENARIO_CASE_IDS[@]}" | jq -R . | jq -s .
)"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
REPORT_CONTENT="$(
  jq -n \
    --arg schema_version "$SCHEMA_VERSION" \
    --arg bead_id "$BEAD_ID" \
    --arg run_id "$RUN_ID" \
    --arg timestamp "$TIMESTAMP" \
    --arg verdict "$VERDICT" \
    --arg replay_command "bash scripts/verify_cache_monitor.sh --json" \
    --argjson cases "$CASES_JSON" \
    --argjson scenarios "$SCENARIOS_JSON" \
    --argjson scenario_cases "$SCENARIO_CASES_JSON" \
    --argjson total_passed "$TOTAL_PASSED" \
    --argjson total_failed "$TOTAL_FAILED" \
    --argjson passed_cases "$PASSED_CASES" \
    --argjson total_cases "$TOTAL_CASES" \
    --argjson scenario_passed_cases "$SCENARIO_PASSED_CASES" \
    --argjson scenario_total_cases "$SCENARIO_TOTAL_CASES" \
    '{
      schema_version: $schema_version,
      bead_id: $bead_id,
      run_id: $run_id,
      timestamp: $timestamp,
      verdict: $verdict,
      replay_command: $replay_command,
      scenario_cases: $scenario_cases,
      observability: {
        required_fields: [
          "bead_id",
          "test_name",
          "workload_type",
          "elapsed_ns",
          "total_accesses",
          "hit_rate_pct",
          "eviction_count",
          "cached_pages",
          "extra.replay_hint"
        ]
      },
      scenarios: $scenarios,
      cases: $cases,
      totals: {
        passed: $total_passed,
        failed: $total_failed,
        passed_cases: $passed_cases,
        total_cases: $total_cases,
        scenario_passed_cases: $scenario_passed_cases,
        scenario_total_cases: $scenario_total_cases
      }
    }'
)"

printf '%s\n' "$REPORT_CONTENT" >"$REPORT_FILE"

if $JSON_MODE; then
  printf '%s\n' "$REPORT_CONTENT"
else
  echo "=== ${BEAD_ID}: Page Cache Efficiency Monitor Verification ==="
  echo "report_path=${REPORT_FILE}"
  echo "verdict=${VERDICT} passed_cases=${PASSED_CASES}/${TOTAL_CASES}"
  jq -r '
    .scenarios[]
    | "scenario=\(.workload_type) test=\(.test_name) hit_rate_pct=\(.hit_rate_pct|floor) evictions=\(.eviction_count) cached_pages=\(.cached_pages)"' \
    "$REPORT_FILE"
fi

if [ "$VERDICT" != "pass" ]; then
  exit 1
fi
