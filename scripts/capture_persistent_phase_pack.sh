#!/usr/bin/env bash
# bd-db300.1.7.2: Capture authoritative persistent 8t and 16t phase-attribution packs.
#
# Uses the Criterion bench entrypoint:
#   crates/fsqlite-e2e/benches/concurrent_write_persistent_bench.rs
#
# Capture surface:
#   FSQLITE_PERSISTENT_PHASE_ATTRIBUTION_DIR → provenance.json + samples.jsonl
#
# Thread counts exercised: 8, 16 (the two degraded regimes from 2026-03-20).
#
# Usage:
#   ./scripts/capture_persistent_phase_pack.sh [output_dir]
#
# Output:
#   <output_dir>/
#     provenance/environment.yaml   — machine/build provenance
#     8t/provenance.json            — Criterion bench provenance (auto-generated)
#     8t/samples.jsonl              — per-iteration phase attribution (auto-generated)
#     8t/criterion_stdout.log       — raw Criterion output
#     16t/provenance.json
#     16t/samples.jsonl
#     16t/criterion_stdout.log
#     rerun.sh                      — one-command reproducibility entrypoint
set -euo pipefail

BEAD_ID="bd-db300.1.7.2"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
OUTPUT_DIR="${1:-${PROJECT_ROOT}/artifacts/persistent_phase_pack_${TIMESTAMP}}"

mkdir -p "$OUTPUT_DIR/provenance"

echo "=== ${BEAD_ID}: Authoritative Persistent Phase-Attribution Pack ==="
echo "Output: $OUTPUT_DIR"
echo "Timestamp: $TIMESTAMP"

# ── Provenance ──────────────────────────────────────────────────────────
echo "--- Capturing environment provenance ---"
{
    echo "bead_id: ${BEAD_ID}"
    echo "capture_timestamp: ${TIMESTAMP}"
    echo "capture_script: scripts/capture_persistent_phase_pack.sh"
    echo "hostname: $(hostname)"
    echo "uname: $(uname -a)"
    echo "cpu_model: $(grep 'model name' /proc/cpuinfo 2>/dev/null | head -1 | cut -d: -f2 | xargs || echo unknown)"
    echo "cpu_count: $(nproc)"
    echo "memory_gb: $(free -g 2>/dev/null | awk '/^Mem:/{print $2}' || echo unknown)"
    echo "numa_nodes: $(ls -d /sys/devices/system/node/node* 2>/dev/null | wc -l || echo 1)"
    echo "load_avg: $(cat /proc/loadavg 2>/dev/null || echo unknown)"
    echo "git_commit: $(git -C "$PROJECT_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
    echo "git_branch: $(git -C "$PROJECT_ROOT" branch --show-current 2>/dev/null || echo unknown)"
    echo "git_dirty_files: $(git -C "$PROJECT_ROOT" diff --name-only 2>/dev/null | wc -l || echo unknown)"
    echo "rust_version: $(rustc --version 2>/dev/null || echo unknown)"
    echo "cargo_profile: release-perf"
    echo "bench_entrypoint: crates/fsqlite-e2e/benches/concurrent_write_persistent_bench.rs"
    echo "capture_env: FSQLITE_PERSISTENT_PHASE_ATTRIBUTION_DIR"
    echo "reference_comparator: C SQLite via rusqlite (built-in to bench)"
    echo "thread_counts: [8, 16]"
    echo "warmup_measurement_disclaimer: |"
    echo "  The Criterion harness runs warmup iterations before measurement."
    echo "  FSQLITE_PERSISTENT_PHASE_ATTRIBUTION_DIR captures ALL iterations"
    echo "  (warmup + measurement) in samples.jsonl. The harness does NOT tag"
    echo "  which samples are warmup vs measurement. Consumers should discard"
    echo "  the first sample_size warmup iterations or use Criterion's own"
    echo "  estimates for authoritative throughput. The samples.jsonl is"
    echo "  authoritative for phase-attribution distributions and wake-reason"
    echo "  accounting, not for headline throughput numbers."
} > "$OUTPUT_DIR/provenance/environment.yaml"

# ── Build ───────────────────────────────────────────────────────────────
# Build and run are both local-only. The Criterion bench binary must be
# compiled on the same machine that runs it — rch remote builds do NOT
# materialize a usable local binary and would cause a redundant rebuild.
echo "--- Building release-perf benchmark binary (local) ---"
cd "$PROJECT_ROOT"
cargo bench --profile release-perf -p fsqlite-e2e \
    --bench concurrent_write_persistent_bench --no-run 2>&1 | tail -5

# ── Run function ────────────────────────────────────────────────────────
run_persistent_bench() {
    local thread_count="$1"
    local label="${thread_count}t"
    local run_dir="$OUTPUT_DIR/${label}"
    mkdir -p "$run_dir"

    echo ""
    echo "=== Capturing ${label} persistent phase pack ==="
    echo "Thread count: $thread_count"
    echo "Phase attribution dir: $run_dir"

    # The Criterion bench selects thread counts via its internal group.
    # We filter to only the matching thread count using --bench-filter.
    # The bench writes provenance.json and samples.jsonl to the dir
    # specified by FSQLITE_PERSISTENT_PHASE_ATTRIBUTION_DIR.
    # Criterion filter: group label is "persistent_concurrent_write_{N}t".
    FSQLITE_PERSISTENT_PHASE_ATTRIBUTION_DIR="$run_dir" \
    cargo bench --profile release-perf -p fsqlite-e2e \
        --bench concurrent_write_persistent_bench \
        -- "persistent_concurrent_write_${thread_count}t" \
        2>&1 | tee "$run_dir/criterion_stdout.log"

    # Verify outputs exist.
    local provenance="$run_dir/provenance.json"
    local samples="$run_dir/samples.jsonl"

    if [ -f "$provenance" ]; then
        echo "  provenance.json: $(wc -c < "$provenance") bytes"
    else
        echo "  WARNING: provenance.json not generated"
    fi

    if [ -f "$samples" ]; then
        local sample_count
        sample_count=$(wc -l < "$samples")
        echo "  samples.jsonl: ${sample_count} records"
    else
        echo "  WARNING: samples.jsonl not generated"
    fi

    echo "--- ${label} capture complete ---"
}

# ── Load check ──────────────────────────────────────────────────────────
echo ""
echo "--- Pre-flight load check ---"
LOAD_1MIN=$(awk '{print $1}' /proc/loadavg 2>/dev/null || echo 0)
echo "Load average (1min): $LOAD_1MIN"
CPU_COUNT=$(nproc)
echo "CPU count: $CPU_COUNT"

# Warn if load is high relative to CPU count.
if awk "BEGIN { exit !($LOAD_1MIN > $CPU_COUNT * 0.8) }" 2>/dev/null; then
    echo "WARNING: Load is >80% of CPU count. Results may be noisy."
    echo "Consider waiting for load to drop or isolating the benchmark."
    echo "Proceeding anyway — noise will be visible in tail distributions."
fi

# ── Execute ─────────────────────────────────────────────────────────────
run_persistent_bench 8
run_persistent_bench 16

# ── Summary ─────────────────────────────────────────────────────────────
echo ""
echo "=== Pack capture complete ==="
echo "Output directory: $OUTPUT_DIR"
echo ""
echo "Artifacts:"
find "$OUTPUT_DIR" -type f | sort | while read -r f; do
    echo "  $(du -h "$f" | cut -f1)  $f"
done

# ── Rerun entrypoint ────────────────────────────────────────────────────
cat > "$OUTPUT_DIR/rerun.sh" << RERUN_EOF
#!/usr/bin/env bash
# One-command rerun of the persistent phase-attribution pack.
# Original capture: ${TIMESTAMP}
# Bead: ${BEAD_ID}
set -euo pipefail
cd "$PROJECT_ROOT"
exec ./scripts/capture_persistent_phase_pack.sh "\${1:-$OUTPUT_DIR.rerun_\$(date +%Y%m%d_%H%M%S)}"
RERUN_EOF
chmod +x "$OUTPUT_DIR/rerun.sh"
echo ""
echo "Rerun: $OUTPUT_DIR/rerun.sh"
