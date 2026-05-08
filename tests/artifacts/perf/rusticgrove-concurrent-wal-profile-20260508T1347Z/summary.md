# Concurrent Writer Profile - 2026-05-08

## Scope

Clean measurement from detached worktree
`/data/tmp/frankensqlite-rusticgrove-concurrent-profile-20260508T1347Z`
at commit `e305a172`.

The local shared worktree had unrelated peer edits in
`crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`, so benchmark runs were
performed from the detached clean worktree.

## Commands

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-rusticgrove-concurrent-profile-20260508T1347Z \
cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --filter concurrent \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/rusticgrove-concurrent-wal-profile-20260508T1347Z/concurrent-profile.json \
  --html /data/projects/frankensqlite/tests/artifacts/perf/rusticgrove-concurrent-wal-profile-20260508T1347Z/concurrent-profile.html
```

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-rusticgrove-concurrent-profile-20260508T1347Z \
RUSTFLAGS=-Cforce-frame-pointers=yes \
cargo build --profile release-perf -p fsqlite-e2e --bin mt-mvcc-bench
```

```bash
perf record -F 1999 --call-graph fp \
  -o tests/artifacts/perf/rusticgrove-concurrent-wal-profile-20260508T1347Z/perf-mt-2t.data \
  -- /data/tmp/frankensqlite-target-rusticgrove-concurrent-profile-20260508T1347Z/release-perf/mt-mvcc-bench \
    --rows-per-thread=1000 --threads=2 --iters=12 \
    --json-output=tests/artifacts/perf/rusticgrove-concurrent-wal-profile-20260508T1347Z/mt-mvcc-2t.json \
    --summary-md=tests/artifacts/perf/rusticgrove-concurrent-wal-profile-20260508T1347Z/mt-mvcc-2t.md \
    --history-json=tests/artifacts/perf/rusticgrove-concurrent-wal-profile-20260508T1347Z/mt-mvcc-history.json
```

## Concurrent Matrix

| Scenario | C SQLite mean ms | FrankenSQLite mean ms | Ratio |
|---|---:|---:|---:|
| 2 writers x 1000 rows | 12.753 | 14.017 | 1.099x |
| 4 writers x 1000 rows | 20.125 | 21.944 | 1.090x |
| 8 writers x 1000 rows | 90.777 | 38.725 | 0.427x |

Summary: `1/0/2` faster/comparable/slower, average ratio `0.872x`,
geomean/primary `0.800x`.

CI p95 regression gate observed the worst mt ratio at 2 writers:
FrankenSQLite p95 `20.832 ms` vs C SQLite p95 `14.437 ms`, ratio `1.443x`.

## Focused mt-mvcc Result

`mt-mvcc-bench --threads=2 --rows-per-thread=1000 --iters=12`:

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio |
|---:|---:|---:|---:|---:|---:|---:|
| 2 | 439282 | 683369 | 0.64x | 4.55 | 2.93 | 1.56x |

Both engines reported `0` failed rows.

## Perf Headline

The 2-thread profile collected `735` samples with no lost samples. The hot
FrankenSQLite path is direct insert and B-tree right-edge work:

- `execute_prepared_with_params_after_background_status`: `25.78%` children
- `execute_precompiled_prepared_insert_fast`: `24.88%`
- `execute_prepared_direct_simple_insert`: `23.82%`
- `execute_prepared_direct_simple_insert_with_cursor<SharedTxnPageIo>`:
  `17.80%`
- `table_insert`: `9.46%`
- `table_seek_for_insert`: `4.42%`
- `table_insert_from_current_position`: `3.90%`
- `table_try_append_cached_rightmost_leaf_hint`: `3.28%`
- `move_to_rightmost_leaf`: `2.10%`

Commit and WAL work is not dominant in this profile:

- `execute_commit_with_cx`: `3.75%`
- `SimpleTransaction::commit`: `2.62%`
- WAL group commit closure: `1.54%`
- `WalBackendAdapter::prepare_append_frames`: `0.69%`
- `finalize_concurrent_commit`: `0.67%`

## Decision

Do not pursue a WAL/checksum/commit-only optimization from this evidence. The
remaining concurrent-writer gap is mostly in prepared direct INSERT and the
B-tree right-edge append/seek path. Any next candidate should target that
surface and avoid already-rejected standalone ideas such as file-backed
preserialized-record widening, expression-only direct INSERT specializations,
or WAL micro-optimizations.
