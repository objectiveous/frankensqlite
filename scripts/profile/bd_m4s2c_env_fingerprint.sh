#!/usr/bin/env bash
# Emit a JSON environment fingerprint for bd-m4s2c profiling runs.

set -euo pipefail

out_path="${1:-}"

tool_version() {
    local tool="$1"
    shift
    if command -v "${tool}" >/dev/null 2>&1; then
        "${tool}" "$@" 2>&1 | head -1
    else
        printf 'missing'
    fi
}

git_value() {
    local key="$1"
    git "${key}" 2>/dev/null || true
}

json="$(
    jq -n \
        --arg schema_version "fsqlite.perf.bd-m4s2c.fingerprint.v1" \
        --arg captured_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        --arg hostname "$(hostname 2>/dev/null || true)" \
        --arg uname "$(uname -a 2>/dev/null || true)" \
        --arg git_head "$(git rev-parse HEAD 2>/dev/null || true)" \
        --arg git_branch "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)" \
        --arg git_status "$(git status --short 2>/dev/null || true)" \
        --arg rustc "$(tool_version rustc --version)" \
        --arg cargo "$(tool_version cargo --version)" \
        --arg samply "$(tool_version samply --version)" \
        --arg heaptrack "$(tool_version heaptrack --version)" \
        --arg heaptrack_print "$(tool_version heaptrack_print --version)" \
        --arg hyperfine "$(tool_version hyperfine --version)" \
        --arg perf "$(tool_version perf --version)" \
        --arg jq "$(tool_version jq --version)" \
        --arg cargo_target_dir "${CARGO_TARGET_DIR:-}" \
        --arg rustflags "${RUSTFLAGS:-}" \
        --arg profile_debug "${CARGO_PROFILE_RELEASE_PERF_DEBUG:-}" \
        --arg profile_strip "${CARGO_PROFILE_RELEASE_PERF_STRIP:-}" \
        --arg profile_lto "${CARGO_PROFILE_RELEASE_PERF_LTO:-}" \
        '{
          schema_version: $schema_version,
          captured_at: $captured_at,
          host: {
            hostname: $hostname,
            uname: $uname
          },
          source: {
            git_head: $git_head,
            git_branch: $git_branch,
            git_status: $git_status
          },
          toolchain: {
            rustc: $rustc,
            cargo: $cargo,
            samply: $samply,
            heaptrack: $heaptrack,
            heaptrack_print: $heaptrack_print,
            hyperfine: $hyperfine,
            perf: $perf,
            jq: $jq
          },
          build: {
            cargo_target_dir: $cargo_target_dir,
            rustflags: $rustflags,
            cargo_profile_release_perf_debug: $profile_debug,
            cargo_profile_release_perf_strip: $profile_strip,
            cargo_profile_release_perf_lto: $profile_lto
          }
        }'
)"

if [[ -n "${out_path}" ]]; then
    mkdir -p "$(dirname "${out_path}")"
    printf '%s\n' "${json}" > "${out_path}"
else
    printf '%s\n' "${json}"
fi
