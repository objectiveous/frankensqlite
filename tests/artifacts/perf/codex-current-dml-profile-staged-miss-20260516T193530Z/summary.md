# Current DML staged-miss profile refresh

- Date: 2026-05-16
- Source revision reported by benchmark: `main @ 9e44c6cb581db765f588eacb0fa369d39fa59f99`
  plus the uncommitted staged-miss instrumentation diff in
  `crates/fsqlite-core/src/connection.rs`,
  `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`, and
  `crates/fsqlite-e2e/src/bin/perf_update_delete.rs`.
- Command:
  `rch exec -- env FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-current-dml-profile-staged-miss-20260516T193530Z/update-delete.json --no-html`
- Local retained artifact: `run.log`
- Note: the benchmark reported writing `update-delete.json`, but RCH only retrieved `run.log` locally.

## Matrix

| Scenario | C SQLite | FrankenSQLite | Result |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 4.5 us | 6.2 us | 1.37x slower |
| 100 rows / delete 5 rows | 2.6 us | 6.6 us | 2.53x slower |
| 1000 rows / update 100 rows | 43.4 us | 32.6 us | 1.33x faster |
| 1000 rows / delete 50 rows | 18.1 us | 32.3 us | 1.78x slower |
| 10000 rows / update 1000 rows | 437.4 us | 312.3 us | 1.40x faster |
| 10000 rows / delete 500 rows | 160.2 us | 335.3 us | 2.09x slower |

Summary line: 6 scenarios, 2 FrankenSQLite faster, 0 comparable, 4 C SQLite faster, average F/C ratio 1.54x.

## DML Profile Notes

- `fs_delete_100`: `delete_leaf_start=1/1`, `delete_leaf_active=4/4`, `delete_leaf_miss_staged=0`, `delete_leaf_flush=1/1`, `delete_leaf_flush_ns=2363`, `direct_flush_ns=3035`, `commit_roundtrip_ns=2013`.
- `fs_delete_1000`: `delete_leaf_start=6/6`, `delete_leaf_active=44/49`, `delete_leaf_miss=5`, `delete_leaf_miss_staged=0`, `delete_leaf_miss_out_of_leaf=5`, `delete_leaf_flush=6/6`, `delete_leaf_flush_ns=8312`, `direct_flush_ns=8963`, `commit_roundtrip_ns=3986`.
- `fs_delete_10000`: `delete_leaf_start=64/67`, `delete_leaf_active=433/496`, `delete_leaf_miss=63`, `delete_leaf_miss_staged=0`, `delete_leaf_miss_out_of_leaf=60`, `delete_leaf_miss_last_cell=3`, `delete_leaf_flush=64/64`, `delete_leaf_flush_ns=88203`, `delete_leaf_materialize=64/68324`, `delete_leaf_search=560/67225`, `delete_leaf_dupcheck=500/19271`, `direct_flush_ns=13450`, `commit_roundtrip_ns=42123`.

## Interpretation

The new staged-run active-miss counter is useful instrumentation but does not expose a new bottleneck in the current `update-delete` matrix: all staged-only miss counts are zero. The remaining DELETE gap is still the previously identified retained leaf-run and transaction/publication envelope: active rowid probes, per-leaf materialization/search/flush, and commit roundtrip. This run should not be used to justify a staged-run-specific optimization.
