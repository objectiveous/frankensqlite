#!/usr/bin/env bash
# Verification gate for bd-t6sv2.8: Page Cache Efficiency Monitor
set -euo pipefail

echo "=== bd-t6sv2.8: Page Cache Efficiency Monitor Verification ==="

cargo test -p fsqlite-core test_pragma_cache_ -- --nocapture 2>&1
cargo test -p fsqlite-core test_fsqlite_cache_pages_table_function_ -- --nocapture 2>&1
cargo test -p fsqlite-pager test_cache_efficiency_snapshot_matches_raw_cache_metrics -- --nocapture 2>&1

echo
echo "[GATE PASS] bd-t6sv2.8 Page Cache Efficiency Monitor — focused verification passed"
