# Prepared Direct DELETE Monotone Floor Rejection

Date: 2026-05-12
Base commit: `fb9e79e1 perf(btree): batch delete-run materialization copies`
Candidate: uncommitted `TableLeafDeleteRun` monotone search-floor fields
(`last_deleted_cell_idx`, `last_deleted_rowid`) used to start same-leaf
active-run rowid searches after the previous accepted cell for increasing
DELETE streams.

## Commands

Baseline after `fb9e79e1`:

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 10000 40 delete compare standard
FSQLITE_BENCH_PROFILE_DML=1 CARGO_TARGET_DIR=/data/tmp/frankensqlite-target cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --no-html
```

Candidate:

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-next-target cargo test -p fsqlite-btree table_leaf_delete_run -- --nocapture
CARGO_TARGET_DIR=/data/tmp/frankensqlite-next-target cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 10000 40 delete compare standard
FSQLITE_BENCH_PROFILE_DML=1 CARGO_TARGET_DIR=/data/tmp/frankensqlite-next-target cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --no-html
```

## Result

- Focused B-tree delete-run tests passed.
- Narrow compare moved only slightly: 10k-row DELETE ratio `1.35x` to `1.32x`
  versus C SQLite (`466 ns` to `477 ns` per FSQLite delete-row in separate
  runs, with C SQLite also moving from `345 ns` to `361 ns`).
- DML profile did not pass the keep gate. The 10k-row DELETE FSQLite median
  regressed from the kept post-`fb9e79e1` profile around `336.9 us` to
  `381.9 us`.
- The intended counter improved only modestly: `delete_leaf_active_ns` moved
  from about `49485 ns` to `42170 ns`.
- That was overwhelmed by other costs: `delete_leaf_materialize` moved from
  about `40346 ns` to `51479 ns`, and `delete_leaf_flush_ns` moved from
  about `53519 ns` to `64992 ns`.

## Retry Condition

Do not retry a standalone monotone retained-leaf search floor. Reconsider only
as part of a broader representation that reduces active search without
increasing flush/materialization cost and improves the absolute FSQLite median
for the 10k-row DELETE workload in the same A/B window.
