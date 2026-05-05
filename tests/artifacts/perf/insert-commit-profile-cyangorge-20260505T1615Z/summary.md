# Insert Commit Profile - CyanGorge - 2026-05-05T1615Z

## Scope

Profiled the insert-only comprehensive benchmark after the full quick matrix
showed remaining C SQLite wins concentrated in `write_bulk` and `write_single`
rows, and after UPDATE/DELETE profiling showed their apparent gap was mostly
prepopulation/direct-INSERT setup cost.

## Commands

Build:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-insert-profile-target RUSTFLAGS=-Cforce-frame-pointers=yes cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Profile run:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 perf record -F 999 -g -o tests/artifacts/perf/insert-commit-profile-cyangorge-20260505T1615Z/perf.data -- /data/tmp/frankensqlite-cyangorge-insert-profile-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/insert-commit-profile-cyangorge-20260505T1615Z/report.json --no-html
```

Reports:

```bash
perf report -i tests/artifacts/perf/insert-commit-profile-cyangorge-20260505T1615Z/perf.data --stdio --children > tests/artifacts/perf/insert-commit-profile-cyangorge-20260505T1615Z/perf-report-children.txt
perf report -i tests/artifacts/perf/insert-commit-profile-cyangorge-20260505T1615Z/perf.data --stdio --no-children > tests/artifacts/perf/insert-commit-profile-cyangorge-20260505T1615Z/perf-report-nochildren.txt
```

## Benchmark Summary

From `report.json`:

- Scenarios: `25`
- FrankenSQLite faster: `0`
- C SQLite faster: `25`
- Average F/C ratio: `2.313700x`
- Geomean F/C ratio: `2.231227x`
- Median F/C ratio: `2.121431x`
- Insert-only weighted score: `1.641974`
- `write_bulk` geomean: `2.365435x`
- `write_single` geomean: `1.453845x`

## Built-in Insert Counters

Representative `FSQLITE_BENCH_PROFILE_INSERT=1` counters:

- `fs_insert_record_size_tiny_1col_10000`: `insert_us=9569.9`, `commit_roundtrip_ns=58360`, `row_build_ns=343804`, `btree_insert_ns=3657330`, `page_pool_misses=21`.
- `fs_insert_record_size_small_3col_10000`: `insert_us=11957.5`, `commit_roundtrip_ns=184245`, `row_build_ns=1489113`, `serialize_ns=602399`, `btree_insert_ns=3827300`, `page_pool_misses=68`.
- `fs_insert_record_size_medium_6col_10000`: `insert_us=14416.3`, `commit_roundtrip_ns=826588`, `row_build_ns=2768690`, `serialize_ns=1239141`, `btree_insert_ns=4274014`, `page_pool_misses=459`.
- `fs_insert_record_size_large_10col_10000`: `insert_us=22298.4`, `commit_roundtrip_ns=15517241`, `row_build_ns=5950129`, `serialize_ns=1777991`, `btree_insert_ns=8097191`, `btree_quick_balance_ns=4062327`, `page_pool_misses=2013`.

The large-record row is the important shape: single-transaction insert body is
still expensive, but commit roundtrip dominates the remaining gap once staged
payload volume crosses roughly 2K pages.

## CPU Profile Highlights

Children report:

- `Connection::execute_prepared_direct_simple_insert`: `14.67%` in one major
  insert section and `13.98%` in another.
- `BtCursor<SharedTxnPageIo>::table_try_append_cached_rightmost_leaf_hint`:
  `5.96%` / `6.30%` beneath direct INSERT.
- `SimpleTransaction::commit_wal_group_commit_with_snapshot`: `7.21%` /
  `5.87%` beneath insert sections.
- `WalBackendAdapter::append_prepared_frames`: `2.14%` / `1.73%` beneath group
  commit.
- `pager::build_group_commit_batch`: `2.13%` / `1.59%`, mostly allocator/copy
  work from cloning staged pages into owned group-commit frames.

No-children report:

- `__memmove_avx_unaligned_erms`: `8.14%` self. Some samples are C SQLite, but
  the FSQLite commit path also shows WAL append copies through `MemoryFile`.
- `Connection::execute_prepared_direct_simple_insert`: `4.17%` self.
- `_int_malloc`: `2.61%` self.
- `__memset_avx2_unaligned_erms`: `2.41%` self.
- `WalChecksumTransform::for_wal_frame`: `1.38%` self.
- `push_prepared_direct_simple_insert_value`: about `1.49%` self.
- `serialize_record_iter_into_impl`: about `1.14%` self.
- `SharedTxnPageIo::clear_stale_synthetic_pending_commit_surface`: about `1.10%` self.

## Candidate Implications

Rejected/fenced by prior artifacts:

- WAL checksum header transforms are not a standalone win.
- Direct prepared-WAL publication from frame metadata was mixed/noisy and
  regressed important rows.
- Rightmost-leaf writer callback and page-data handoff ideas are already fenced
  by the negative-results ledger.

Current promising but high-risk lead:

- `pager::build_group_commit_batch` clones each staged page with
  `staged_page.as_page_bytes().to_vec()` before the batch enters the
  consolidator. That clone is necessary for general group commit because the
  batch may outlive the caller's `write_set` while waiting for peers. A safe
  optimization would need a separate immediate/single-batch path that preserves
  conflict snapshots, page-one header promotion, WAL lock ordering, sync policy,
  and concurrent-writer semantics. Do not attempt this as a casual helper
  rewrite.

Narrower candidate under active validation:

- Reuse the `SharedTxnPageIo` wrapper for prepared concurrent direct
  INSERT/UPDATE/DELETE so each execution does not rebuild the cloneable
  `Rc<RefCell<...>>` pair. This targets the direct-DML setup part of the same
  profile without weakening group-commit ownership rules.
