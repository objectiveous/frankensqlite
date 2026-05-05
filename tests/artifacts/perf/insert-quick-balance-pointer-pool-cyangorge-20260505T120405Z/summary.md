# Rejected INSERT quick-balance pointer-pool candidate

Date: 2026-05-05 12:04 UTC

Candidate:
- Replaced the quick-balance success path's `vec![result.new_cell_ptr]` with a helper that reused the existing thread-local cell-pointer `Vec<u16>` pool.
- Touched only `crates/fsqlite-btree/src/cursor.rs`.

Behavior proof:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-check-target cargo test -p fsqlite-btree rightmost_leaf_hint -- --nocapture
```

Result: 8 tests passed.

Benchmark command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --filter insert --json-out tests/artifacts/perf/insert-quick-balance-pointer-pool-cyangorge-20260505T120405Z/report.json --no-html
```

Baseline:
- `tests/artifacts/perf/insert-quick-balance-exact-space-cyangorge-20260505T115109Z/report.json`

Candidate:
- `tests/artifacts/perf/insert-quick-balance-pointer-pool-cyangorge-20260505T120405Z/report.json`
- `tests/artifacts/perf/insert-quick-balance-pointer-pool-cyangorge-20260505T120405Z/run.log`
- `tests/artifacts/perf/insert-quick-balance-pointer-pool-cyangorge-20260505T120405Z/stdout.txt`

Decision:
- Rejected and reverted.
- The ratio summary improved, but C SQLite variance drove much of that apparent movement.
- FrankenSQLite median regressed on split-heavy rows: `large_10col` 10K `34.756 ms` -> `37.287 ms`; `large_10col` 100K `415.902 ms` -> `451.660 ms`.
- The direct hot counter also worsened for `large_10col` 10K: `btree_quick_balance_ns` `4.309 ms` -> `5.262 ms`.

Retry condition:
- Only revisit this if allocator profiling proves the one-cell `Vec` allocation is dominant and a cheaper no-TLS path is available.
