# Current `UPDATE/DELETEThroughput` Focused Refresh

## Run

- Run ID: `codex-41a82af7-update-filter-refresh-20260514T215206Z`
- Date: 2026-05-14
- Source commit: `41a82af76ae343058c719859e27d0d2e8851b986`
- Host: `threadripperje`, AMD Ryzen Threadripper PRO 5995WX, Linux 6.17.0-19-generic
- Toolchain: `rustc 1.97.0-nightly (ff9a9ea07 2026-05-13)`
- Command:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-41a82af7-update-filter-target \
  FSQLITE_BENCH_PROFILE_DML=1 \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter update \
  --json-out tests/artifacts/perf/codex-41a82af7-update-filter-refresh-20260514T215206Z/update-filter.json \
  --no-html
```

## Result

Focused `--quick --filter update` still leaves DELETE as the current red
frontier. The section summary reports 6 scenarios: 2 FrankenSQLite-faster,
0 comparable, and 4 C SQLite-faster. The focused geomean F/C ratio is
`1.5010`; because the run is filtered, the weighted primary score only covers
the `write_single` category.

| Scenario | C SQLite median | FrankenSQLite median | F/C ratio | Notes |
|---|---:|---:|---:|---|
| 100 rows / update 10 rows | 4.188 us | 7.063 us | 1.686x slower | FSQLite CV 134.5%, noisy small row |
| 100 rows / delete 5 rows | 2.314 us | 7.414 us | 3.204x slower | stable small DELETE red row |
| 1000 rows / update 100 rows | 36.097 us | 29.996 us | 0.831x | FSQLite faster |
| 1000 rows / delete 50 rows | 15.940 us | 30.867 us | 1.936x slower | DELETE remains red |
| 10000 rows / update 1000 rows | 374.942 us | 276.478 us | 0.737x | FSQLite faster |
| 10000 rows / delete 500 rows | 162.825 us | 290.434 us | 1.784x slower | DELETE remains red |

## Profile Notes

All DELETE rows stayed on the prepared direct path (`slow=0`). The representative
10K/500 DELETE profile reported:

- `delete_leaf_start=64/67`
- `delete_leaf_active=433/496`
- `delete_leaf_miss=63`
- `delete_leaf_miss_out_of_leaf=60`
- `delete_leaf_miss_last_cell=3`
- `delete_leaf_flush=64/64`
- `delete_leaf_flush_ns=56345`
- `delete_leaf_materialize=64/43294`
- `delete_leaf_search=560/39835`
- `delete_leaf_dupcheck=500/12609`
- `delete_leaf_compact=497/15363`
- `delete_leaf_cellparse=497/13142`
- `execute_body_ns=63849`
- `direct_flush_calls=2`
- `direct_flush_ns=7845`
- `commit_roundtrip_ns=22702`

## Interpretation

This current-head refresh reconfirms the open `bd-db300.11.1`
transaction-local DML mutation-operator card. The same retained leaf-run shape
is still visible: many same-leaf active hits, 64 physical dirty-leaf flushes,
and repeated leaf materialization/search work on the 500-row DELETE case.

This does not invalidate the negative-results ledger. Standalone retained
leaf-run tweaks, physical tombstone-only buffers, dense-rowid queued overlays,
prepared rowid-keyspace buffers, direct-flush wrappers, and freed-page lookup
micro-patches remain fenced off. The next credible implementation slice is the
broader logical rowid/key-space mutation operator integration boundary, with
read-your-writes, rollback/savepoint, cache-invalidation, and MVCC publication
proof coverage before applying the focused DELETE keep gate.
