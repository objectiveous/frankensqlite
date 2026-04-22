#!/usr/bin/env bash
# Best-effort profiler tool bootstrap for bd-m4s2c RCH workers.

set -euo pipefail

need_tool() {
    ! command -v "$1" >/dev/null 2>&1
}

if need_tool samply; then
    cargo install samply --locked
fi

deb_packages=()
if need_tool heaptrack || need_tool heaptrack_print; then
    deb_packages+=(heaptrack)
fi
if need_tool hyperfine; then
    deb_packages+=(hyperfine)
fi
if need_tool jq; then
    deb_packages+=(jq)
fi
if need_tool perf; then
    deb_packages+=(linux-perf linux-tools-common linux-tools-generic)
fi

if ((${#deb_packages[@]} > 0)); then
    if ! command -v sudo >/dev/null 2>&1; then
        printf 'missing sudo; cannot install apt packages: %s\n' "${deb_packages[*]}" >&2
        exit 1
    fi
    sudo apt-get update
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y "${deb_packages[@]}" || {
        status=0
        for package in "${deb_packages[@]}"; do
            sudo DEBIAN_FRONTEND=noninteractive apt-get install -y "${package}" || status=1
        done
        exit "${status}"
    }
fi

if ! command -v flamegraph.pl >/dev/null 2>&1 \
    && ! [[ -x /opt/FlameGraph/flamegraph.pl ]] \
    && ! command -v inferno-flamegraph >/dev/null 2>&1; then
    cargo install inferno --locked
fi

for tool in samply heaptrack heaptrack_print hyperfine perf jq; do
    printf '%-16s' "${tool}"
    command -v "${tool}" || true
done
