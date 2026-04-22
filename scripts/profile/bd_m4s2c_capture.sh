#!/usr/bin/env bash
# Capture bd-m4s2c CPU, allocation, off-CPU, and timing profiles.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
source "${SCRIPT_DIR}/bd_m4s2c_scenarios.sh"

BEAD_ID="bd-m4s2c"
RUN_ID="${RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)}"
OUT_DIR="${OUT_DIR:-${WORKSPACE_ROOT}/tests/artifacts/perf/${BEAD_ID}-${RUN_ID}}"
if [[ -z "${CARGO_TARGET_DIR:-}" || "${CARGO_TARGET_DIR}" == "/data/tmp/cargo-target" ]]; then
    CARGO_TARGET_DIR="${TMPDIR:-/tmp}/rch_target_fsqlite_profile_${BEAD_ID}"
fi
BIN_DIR="${CARGO_TARGET_DIR}/release-perf"
PERF_FREQ="${PERF_FREQ:-999}"
SAMPLE_RATE="${SAMPLE_RATE:-1000}"

export CARGO_TARGET_DIR
export CARGO_PROFILE_RELEASE_PERF_DEBUG="${CARGO_PROFILE_RELEASE_PERF_DEBUG:-line-tables-only}"
export CARGO_PROFILE_RELEASE_PERF_STRIP="${CARGO_PROFILE_RELEASE_PERF_STRIP:-false}"
export RUSTFLAGS="${RUSTFLAGS:-} -C force-frame-pointers=yes -C debuginfo=line-tables-only -C strip=none"

usage() {
    cat <<USAGE
usage: $0 [all|scenario...]

Scenarios:
  ${BD_M4S2C_SCENARIOS[*]}

Environment:
  OUT_DIR                default: ${OUT_DIR}
  CARGO_TARGET_DIR       default: ${CARGO_TARGET_DIR}
  BD_M4S2C_MT_ITERS      default: 3
  BD_M4S2C_UPDATE_ITERS  default: 1
USAGE
}

require_tool() {
    if ! command -v "$1" >/dev/null 2>&1; then
        printf 'missing required tool: %s\n' "$1" >&2
        printf 'run: scripts/profile/bd_m4s2c_install_tools.sh\n' >&2
        return 1
    fi
}

write_unavailable_svg() {
    local svg_path="$1"
    local label="$2"
    cat > "${svg_path}" <<SVG
<svg xmlns="http://www.w3.org/2000/svg" width="960" height="120">
  <rect width="100%" height="100%" fill="#f8fafc"/>
  <text x="24" y="52" font-family="monospace" font-size="18" fill="#0f172a">${label}</text>
  <text x="24" y="84" font-family="monospace" font-size="14" fill="#334155">See adjacent .log artifact for the capture failure.</text>
</svg>
SVG
}

render_perf_svg() {
    local perf_data="$1"
    local folded="$2"
    local svg="$3"
    local title="$4"

    if [[ -x /opt/FlameGraph/stackcollapse-perf.pl && -x /opt/FlameGraph/flamegraph.pl ]]; then
        perf script -i "${perf_data}" | /opt/FlameGraph/stackcollapse-perf.pl > "${folded}"
        /opt/FlameGraph/flamegraph.pl --title "${title}" "${folded}" > "${svg}"
        return 0
    fi

    if command -v stackcollapse-perf.pl >/dev/null 2>&1 && command -v flamegraph.pl >/dev/null 2>&1; then
        perf script -i "${perf_data}" | stackcollapse-perf.pl > "${folded}"
        flamegraph.pl --title "${title}" "${folded}" > "${svg}"
        return 0
    fi

    if command -v inferno-collapse-perf >/dev/null 2>&1 && command -v inferno-flamegraph >/dev/null 2>&1; then
        perf script -i "${perf_data}" | inferno-collapse-perf > "${folded}"
        inferno-flamegraph --title "${title}" < "${folded}" > "${svg}"
        return 0
    fi

    write_unavailable_svg "${svg}" "flamegraph renderer unavailable"
}

build_release_perf() {
    cargo build --profile release-perf -p fsqlite-e2e \
        --bin comprehensive-bench \
        --bin perf-update-delete \
        --bin mt-mvcc-bench
    cargo test --profile release-perf -p fsqlite-core \
        --test fast_path_separation \
        --no-run
}

scenario_command() {
    bd_m4s2c_scenario_command "$1" "${WORKSPACE_ROOT}" "${BIN_DIR}"
}

capture_time() {
    local scenario="$1"
    local cmd="$2"
    /usr/bin/time -v -o "${OUT_DIR}/time-${scenario}.txt" bash -lc "${cmd}" \
        > "${OUT_DIR}/run-${scenario}.stdout.log" \
        2> "${OUT_DIR}/run-${scenario}.stderr.log"
}

capture_samply() {
    local scenario="$1"
    local cmd="$2"
    samply record \
        --save-only \
        --rate "${SAMPLE_RATE}" \
        --profile-name "${BEAD_ID}-${scenario}" \
        -o "${OUT_DIR}/cpu-${scenario}.samply.json.gz" \
        -- bash -lc "${cmd}" \
        > "${OUT_DIR}/samply-${scenario}.log" \
        2>&1
}

capture_cpu_svg() {
    local scenario="$1"
    local cmd="$2"
    local data="${OUT_DIR}/perf-cpu-${scenario}.data"
    local folded="${OUT_DIR}/cpu-${scenario}.folded"
    local svg="${OUT_DIR}/cpu-${scenario}.svg"

    if perf record -g --call-graph dwarf,8192 -F "${PERF_FREQ}" \
        -o "${data}" -- bash -lc "${cmd}" \
        > "${OUT_DIR}/perf-cpu-${scenario}.log" 2>&1; then
        render_perf_svg "${data}" "${folded}" "${svg}" "${BEAD_ID} ${scenario} CPU"
    else
        write_unavailable_svg "${svg}" "perf CPU capture unavailable for ${scenario}"
    fi
}

capture_heaptrack() {
    local scenario="$1"
    local cmd="$2"
    local prefix="${OUT_DIR}/heaptrack-${scenario}"
    local raw_path=""
    local summary="${OUT_DIR}/alloc-${scenario}.txt"
    local folded="${OUT_DIR}/alloc-${scenario}.folded"
    local svg="${OUT_DIR}/alloc-${scenario}.svg"
    local json="${OUT_DIR}/alloc-${scenario}.json"

    heaptrack --record-only -o "${prefix}" bash -lc "${cmd}" \
        > "${OUT_DIR}/heaptrack-${scenario}.log" 2>&1 || true
    raw_path="$(
        find "${OUT_DIR}" -maxdepth 1 -type f -name "heaptrack-${scenario}*.gz" \
            -printf '%T@ %p\n' | sort -nr | head -1 | cut -d' ' -f2-
    )"

    if [[ -n "${raw_path}" ]]; then
        heaptrack_print -f "${raw_path}" > "${summary}" 2> "${OUT_DIR}/heaptrack-print-${scenario}.log" || true
        heaptrack_print -f "${raw_path}" -F "${folded}" \
            >> "${OUT_DIR}/heaptrack-print-${scenario}.log" 2>&1 || true
        if [[ -s "${folded}" ]]; then
            if [[ -x /opt/FlameGraph/flamegraph.pl ]]; then
                /opt/FlameGraph/flamegraph.pl --title "${BEAD_ID} ${scenario} allocations" \
                    --colors mem --countname allocations < "${folded}" > "${svg}"
            elif command -v flamegraph.pl >/dev/null 2>&1; then
                flamegraph.pl --title "${BEAD_ID} ${scenario} allocations" \
                    --colors mem --countname allocations < "${folded}" > "${svg}"
            elif command -v inferno-flamegraph >/dev/null 2>&1; then
                inferno-flamegraph --title "${BEAD_ID} ${scenario} allocations" \
                    < "${folded}" > "${svg}"
            fi
        fi
        jq -n \
            --arg scenario "${scenario}" \
            --arg raw_path "${raw_path}" \
            --arg summary_path "${summary}" \
            --arg folded_path "${folded}" \
            --arg svg_path "${svg}" \
            '{scenario: $scenario, tool: "heaptrack", raw_path: $raw_path, summary_path: $summary_path, folded_path: $folded_path, svg_path: $svg_path}' \
            > "${json}"
    else
        jq -n \
            --arg scenario "${scenario}" \
            --arg log_path "${OUT_DIR}/heaptrack-${scenario}.log" \
            '{scenario: $scenario, tool: "heaptrack", unavailable: true, log_path: $log_path}' \
            > "${json}"
    fi
}

capture_offcpu() {
    local scenario="$1"
    local cmd="$2"
    local data="${OUT_DIR}/offcpu-${scenario}.data"
    local folded="${OUT_DIR}/offcpu-${scenario}.folded"
    local svg="${OUT_DIR}/offcpu-${scenario}.svg"

    if perf record -e sched:sched_switch -g --call-graph dwarf,8192 \
        -o "${data}" -- bash -lc "${cmd}" \
        > "${OUT_DIR}/offcpu-${scenario}.log" 2>&1; then
        render_perf_svg "${data}" "${folded}" "${svg}" "${BEAD_ID} ${scenario} off-CPU"
    else
        write_unavailable_svg "${svg}" "off-CPU capture unavailable for ${scenario}"
    fi
}

write_markdown_templates() {
    cat > "${OUT_DIR}/hotspot-table.md" <<'MD'
# bd-m4s2c Hotspot Table

| 2026-04-19 hotspot | baseline self-time | current self-time | evidence artifact | movement | winning commit / note |
| --- | ---: | ---: | --- | --- | --- |
| memcpy | 6.77% | TBD | TBD | TBD | TBD |
| CellRef::parse | 4.80% | TBD | TBD | TBD | TBD |
| execute_prepared_direct_simple_insert | 4.24% | TBD | TBD | TBD | TBD |
| _int_malloc | 3.96% | TBD | TBD | TBD | TBD |
| Arc::make_mut | 1.77% | TBD | TBD | TBD | TBD |
| Vec::finish_grow | 1.20% | TBD | TBD | TBD | TBD |
| WalChecksumTransform | 0.79% | TBD | TBD | TBD | TBD |
MD

    cat > "${OUT_DIR}/hypothesis-ledger.md" <<'MD'
# bd-m4s2c Hypothesis Ledger

| hypothesis | scenario evidence | supports / rejects | next bead |
| --- | --- | --- | --- |
| TBD | TBD | TBD | TBD |
MD
}

run_baseline() {
    local hyperfine_args=(--warmup 3 --runs 20 --export-json "${OUT_DIR}/baseline.json")
    local scenario cmd
    for scenario in "$@"; do
        cmd="$(scenario_command "${scenario}")"
        printf '%s\n' "${cmd}" > "${OUT_DIR}/cmd-${scenario}.txt"
        hyperfine_args+=(-n "${scenario}" "${cmd}")
    done
    hyperfine "${hyperfine_args[@]}" > "${OUT_DIR}/hyperfine.log" 2>&1
}

capture_one() {
    local scenario="$1"
    local cmd
    cmd="$(scenario_command "${scenario}")"
    printf '%s\n' "${cmd}" > "${OUT_DIR}/cmd-${scenario}.txt"

    capture_time "${scenario}" "${cmd}"
    capture_samply "${scenario}" "${cmd}"
    capture_cpu_svg "${scenario}" "${cmd}"
    capture_heaptrack "${scenario}" "${cmd}"
    if bd_m4s2c_needs_offcpu "${scenario}"; then
        capture_offcpu "${scenario}" "${cmd}"
    fi
}

main() {
    local requested=("$@")
    local scenarios=()
    local scenario

    if [[ ${#requested[@]} -eq 0 || "${requested[0]}" == "all" ]]; then
        scenarios=("${BD_M4S2C_SCENARIOS[@]}")
    else
        for scenario in "${requested[@]}"; do
            if [[ "${scenario}" == "-h" || "${scenario}" == "--help" ]]; then
                usage
                return 0
            fi
            bd_m4s2c_validate_scenario "${scenario}"
            scenarios+=("${scenario}")
        done
    fi

    for tool in jq cargo rustc samply heaptrack heaptrack_print hyperfine perf /usr/bin/time; do
        require_tool "${tool}"
    done

    mkdir -p "${OUT_DIR}"
    cd "${WORKSPACE_ROOT}"
    "${SCRIPT_DIR}/bd_m4s2c_env_fingerprint.sh" "${OUT_DIR}/fingerprint.json"
    build_release_perf
    run_baseline "${scenarios[@]}"
    for scenario in "${scenarios[@]}"; do
        capture_one "${scenario}"
    done
    write_markdown_templates
    printf 'bd-m4s2c artifacts: %s\n' "${OUT_DIR}"
}

main "$@"
