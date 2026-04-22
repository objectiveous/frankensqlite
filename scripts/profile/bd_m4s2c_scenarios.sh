#!/usr/bin/env bash
# Scenario command definitions for bd-m4s2c profile capture.

set -euo pipefail

BD_M4S2C_SCENARIOS=(
    insert_1000_small_3col
    update_100_of_1000
    mt_writers_8x500
    mt_writers_16x500
)

bd_m4s2c_scenario_label() {
    case "$1" in
        insert_1000_small_3col)
            printf '%s\n' 'Single-thread INSERT 1000 small_3col'
            ;;
        update_100_of_1000)
            printf '%s\n' 'UPDATE 100 rows of a 1000-row table'
            ;;
        mt_writers_8x500)
            printf '%s\n' 'Concurrent writers: 8 threads, 500 rows/thread'
            ;;
        mt_writers_16x500)
            printf '%s\n' 'Concurrent writers: 16 threads, 500 rows/thread'
            ;;
        *)
            printf 'unknown scenario: %s\n' "$1" >&2
            return 2
            ;;
    esac
}

bd_m4s2c_needs_offcpu() {
    case "$1" in
        mt_writers_8x500|mt_writers_16x500)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

bd_m4s2c_scenario_command() {
    local scenario="$1"
    local workspace_root="$2"
    local bin_dir="$3"
    local mt_iters="${BD_M4S2C_MT_ITERS:-3}"
    local update_iters="${BD_M4S2C_UPDATE_ITERS:-1}"

    case "${scenario}" in
        insert_1000_small_3col)
            printf 'cd %q && cargo test --profile release-perf -p fsqlite-core --test fast_path_separation manual_profile_bench_shape_prepared_direct_insert_1000 -- --ignored --nocapture\n' \
                "${workspace_root}"
            ;;
        update_100_of_1000)
            printf '%q 1000 %q update\n' \
                "${bin_dir}/perf-update-delete" \
                "${update_iters}"
            ;;
        mt_writers_8x500)
            printf '%q --threads=8 --rows-per-thread=500 --iters=%q\n' \
                "${bin_dir}/mt-mvcc-bench" \
                "${mt_iters}"
            ;;
        mt_writers_16x500)
            printf '%q --threads=16 --rows-per-thread=500 --iters=%q\n' \
                "${bin_dir}/mt-mvcc-bench" \
                "${mt_iters}"
            ;;
        *)
            printf 'unknown scenario: %s\n' "${scenario}" >&2
            return 2
            ;;
    esac
}

bd_m4s2c_validate_scenario() {
    local candidate="$1"
    local scenario
    for scenario in "${BD_M4S2C_SCENARIOS[@]}"; do
        if [[ "${scenario}" == "${candidate}" ]]; then
            return 0
        fi
    done
    printf 'unknown scenario: %s\n' "${candidate}" >&2
    printf 'valid scenarios: %s\n' "${BD_M4S2C_SCENARIOS[*]}" >&2
    return 2
}
