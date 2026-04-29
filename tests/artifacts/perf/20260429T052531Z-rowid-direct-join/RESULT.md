# Rowid Direct Read JOIN Perf Pass

Date: 2026-04-29

## Change Under Test

- `BtCursor::rowid()` now returns table-leaf rowids through the existing inline varint reader instead of building a full `CellRef`.
- Added `test_rowid_reads_table_leaf_without_cell_slot_parse` to prove table-leaf rowid reads do not populate the cell-slot cache.

## Benchmark

Command:

```bash
/data/tmp/cargo-target-tanibis-rowid-direct/release-perf/comprehensive-bench \
  --quick --filter join \
  --json-out tests/artifacts/perf/20260429T052531Z-rowid-direct-join/join-after.json \
  --no-html
```

Baseline: prior broad-profile JOIN JSON from `2026-04-29T035259Z`.

## Summary Delta

| Metric | Baseline | After | Delta |
| --- | ---: | ---: | ---: |
| avg ratio | 6.8375x | 5.6161x | -17.86% |
| geomean ratio | 6.4079x | 5.2734x | -17.71% |
| median ratio | 6.3131x | 5.4118x | -14.28% |
| p99 ratio | 12.4972x | 9.3753x | -24.98% |
| weighted score | 5.8213x | 4.7602x | -18.23% |

All FrankenSQLite median JOIN scenario times improved against the baseline. The 100-row LEFT JOIN ratio rose because the C SQLite median changed more than the FrankenSQLite median, but FrankenSQLite's own median still improved from `0.133249 ms` to `0.114554 ms`.

## Verification

- `cargo test -p fsqlite-btree test_rowid_reads_table_leaf_without_cell_slot_parse -- --nocapture`
- `cargo fmt --check`
- `git diff --check`
- `ubs crates/fsqlite-btree/src/cursor.rs` (exit 0; existing warning inventory remains)
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
