#!/usr/bin/env bash
# bd-db300.1.7.1: Capture authoritative c1 hot-path artifact packs.
#
# Captures release-perf artifact packs for the worst low-concurrency cells
# across ALL canonical fixtures and workloads.
#
# Runs C SQLite control, FrankenSQLite MVCC, and FrankenSQLite single-writer
# at c1 for each fixture, plus per-fixture hot-path profiles for the worst
# workload (commutative_inserts_disjoint_keys).
#
# Usage:
#   ./scripts/capture_c1_evidence_pack.sh [OUTPUT_DIR]
#
# Environment overrides:
#   CARGO_TARGET_DIR  — target directory for the release-perf binary
#   DB_FIXTURES       — comma-separated fixture list (default: all 3 canonical)
#   REPEAT            — measurement iterations per cell (default: 3)
#
# Requirements:
#   Build first, or let the script build:
#     CARGO_TARGET_DIR=/tmp/c1-evidence cargo build --profile release-perf -p fsqlite-e2e

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

OUTPUT_DIR="${1:-${PROJECT_ROOT}/artifacts/c1_evidence_pack_$(date +%Y%m%d_%H%M%S)}"
mkdir -p "$OUTPUT_DIR"

# Record build metadata.
cat > "$OUTPUT_DIR/build_metadata.json" << METADATA
{
  "date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "bead": "bd-db300.1.7.1",
  "profile": "release-perf",
  "hostname": "$(hostname)",
  "rustc_version": "$(rustc --version)",
  "cargo_target_dir": "${CARGO_TARGET_DIR:-default}",
  "git_sha": "$(git -C "$PROJECT_ROOT" rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "git_dirty": "$(git -C "$PROJECT_ROOT" status --porcelain 2>/dev/null | wc -l | tr -d ' ')",
  "cpu_model": "$(grep 'model name' /proc/cpuinfo 2>/dev/null | head -1 | cut -d: -f2 | xargs || echo 'unknown')",
  "cpu_cores": "$(nproc 2>/dev/null || echo 'unknown')"
}
METADATA

echo "=== bd-db300.1.7.1: c1 Evidence Pack ==="
echo "Output: $OUTPUT_DIR"
echo "Profile: release-perf"
echo ""

BINARY="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}/release-perf/realdb-e2e"
if [ ! -f "$BINARY" ]; then
    echo "Building realdb-e2e with release-perf profile..."
    cd "$PROJECT_ROOT"
    cargo build --profile release-perf -p fsqlite-e2e --bin realdb-e2e
fi

# Canonical workloads for c1 evidence.
WORKLOADS="commutative_inserts_disjoint_keys,hot_page_contention,mixed_read_write"
CONCURRENCY="1"
REPEAT="${REPEAT:-3}"
# Canonical fixtures from beads_benchmark_campaign.v1.json.
# Override with DB_FIXTURES="frankensqlite" for a single-fixture run.
DB_FIXTURES="${DB_FIXTURES:-frankensqlite,frankentui,frankensearch}"

IFS=',' read -ra FIXTURE_ARRAY <<< "$DB_FIXTURES"

for DB_FIXTURE in "${FIXTURE_ARRAY[@]}"; do
    echo "====== Fixture: $DB_FIXTURE ======"

    echo "--- C SQLite control (c1, fixture=$DB_FIXTURE) ---"
    "$BINARY" bench \
        --db "$DB_FIXTURE" \
        --preset "$WORKLOADS" \
        --concurrency "$CONCURRENCY" \
        --engine sqlite3 \
        --repeat "$REPEAT" \
        --output-jsonl "$OUTPUT_DIR/c1_${DB_FIXTURE}_sqlite3.jsonl" \
        --pretty 2>&1 | tee "$OUTPUT_DIR/c1_${DB_FIXTURE}_sqlite3_stdout.log"

    echo ""
    echo "--- FrankenSQLite MVCC (c1, fixture=$DB_FIXTURE) ---"
    "$BINARY" bench \
        --db "$DB_FIXTURE" \
        --preset "$WORKLOADS" \
        --concurrency "$CONCURRENCY" \
        --engine fsqlite \
        --mvcc \
        --repeat "$REPEAT" \
        --output-jsonl "$OUTPUT_DIR/c1_${DB_FIXTURE}_fsqlite_mvcc.jsonl" \
        --pretty 2>&1 | tee "$OUTPUT_DIR/c1_${DB_FIXTURE}_fsqlite_mvcc_stdout.log"

    echo ""
    echo "--- FrankenSQLite single-writer (c1, fixture=$DB_FIXTURE) ---"
    "$BINARY" bench \
        --db "$DB_FIXTURE" \
        --preset "$WORKLOADS" \
        --concurrency "$CONCURRENCY" \
        --engine fsqlite \
        --no-mvcc \
        --repeat "$REPEAT" \
        --output-jsonl "$OUTPUT_DIR/c1_${DB_FIXTURE}_fsqlite_single.jsonl" \
        --pretty 2>&1 | tee "$OUTPUT_DIR/c1_${DB_FIXTURE}_fsqlite_single_stdout.log"

    echo ""

    # Hot-path profile for worst workload on THIS fixture.
    HP_DIR="$OUTPUT_DIR/hotprofile_${DB_FIXTURE}_commutative_c1"
    mkdir -p "$HP_DIR"
    echo "--- Hot-path profile (c1, commutative_inserts_disjoint_keys, fixture=$DB_FIXTURE) ---"
    "$BINARY" hot-profile \
        --db "$DB_FIXTURE" \
        --preset "commutative_inserts_disjoint_keys" \
        --concurrency 1 \
        --mvcc \
        --output-dir "$HP_DIR" \
        --pretty 2>&1 | tee "$OUTPUT_DIR/c1_${DB_FIXTURE}_hotprofile_commutative.log"

    echo ""
done

echo "=== Evidence pack complete: $OUTPUT_DIR ==="
echo "Files:"
ls -la "$OUTPUT_DIR/"
echo ""
echo "To analyze (example for frankensqlite):"
echo "  cat $OUTPUT_DIR/c1_frankensqlite_sqlite3.jsonl | python3 -m json.tool"
echo "  cat $OUTPUT_DIR/c1_frankensqlite_fsqlite_mvcc.jsonl | python3 -m json.tool"
echo ""
echo "Hot-path profile artifacts per fixture:"
for DB_FIXTURE in "${FIXTURE_ARRAY[@]}"; do
    echo "  $OUTPUT_DIR/hotprofile_${DB_FIXTURE}_commutative_c1/"
done
