# Current-head INSERT profile

Date: 2026-05-05
Agent: CyanGorge
Git HEAD: `f3d6f54b05d3312541bb4cd21011b1f91bba19b3`
Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 ./.rch-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/insert-current-head-profile-cyangorge-20260505T2340Z/report.json
```

The run used the already-built `release-perf` benchmark binary. The source
worktree was dirty only in coordination/documentation surfaces, not in benchmark
code paths.

## Aggregate result

- Scenarios: 25 INSERT rows.
- FrankenSQLite faster: 0.
- C SQLite faster: 25.
- Average F/C ratio: `2.4316x`.
- Geomean F/C ratio: `2.3183x`.
- Median F/C ratio: `2.4128x`.
- P99 F/C ratio: `4.0792x`.
- Insert-only weighted score: `1.7491` with observed weight `0.4`.

## Slowest rows

| Ratio | Category | Scenario | FSQLite median | C SQLite median |
| ---: | --- | --- | ---: | ---: |
| `4.0792x` | `write_bulk` | `1000-rows-batched-1000-txn` small 3-col | `1.915017 ms` | `0.469460 ms` |
| `3.9544x` | `write_bulk` | `single-transaction-large-10col` 10K rows | `39.377173 ms` | `9.957816 ms` |
| `3.7630x` | `write_bulk` | `single-transaction-small-3col` 100 rows | `0.298249 ms` | `0.079258 ms` |
| `3.5117x` | `write_bulk` | `single-transaction-tiny-1col` 100 rows | `0.257963 ms` | `0.073458 ms` |
| `3.3104x` | `write_bulk` | `record-size large 10-col` 10K rows | `78.006306 ms` | `23.564037 ms` |

## Profile signals

Representative 10K-row direct INSERT rows show the remaining cost is split
between record construction, B-tree append/balance work, and WAL commit frame
preparation. Examples from `stderr.log`:

- `fs_insert_single_txn_tiny_1col_10000`: `insert_us=9437.7`,
  `commit_us=297.1`, `btree_insert_ns=3565980`,
  `btree_leaf_payload_stage_ns=1179973`, `row_build_ns=361571`.
- `fs_insert_record_size_small_3col_10000`: `insert_us=18495.9`,
  `commit_us=304.7`, `row_build_ns=2633283`, `serialize_ns=985336`,
  `btree_insert_ns=6096241`, `btree_leaf_payload_stage_ns=2259867`.
- `fs_insert_record_size_medium_6col_10000`: `insert_us=25359.3`,
  `commit_us=1599.9`, `row_build_ns=5524634`, `serialize_ns=2332579`,
  `btree_insert_ns=7857843`, `btree_quick_balance_ns=1212424`,
  `commit_phase_b_us=1134`.
- `fs_insert_record_size_large_10col_10000`: `insert_us=29112.8`,
  `commit_us=21022.7`, `row_build_ns=7645557`, `serialize_ns=2348813`,
  `btree_insert_ns=11089642`, `btree_quick_balance_ns=5998853`,
  `commit_prepare_us=5620`, `commit_batch_build_us=5613`,
  `commit_wal_append_us=5407`, `commit_flush_frame_prep_us=7609`.

## Interpretation

The highest-EV next lever is still the direct INSERT write path, but recent
negative ledger entries fence off several tempting clone/cache reshuffles:
record header specializations, retained-page handoff variants, staged
take/restore forwarding, and standalone WAL checksum/page-index shortcuts.

The profile points toward a more structural retained-leaf batching design:
coalesce repeated rightmost-leaf appends so the page image is not cloned/staged
once per row while still preserving read-your-own-write, conflict detection,
split handling, rollback, and commit publication. That design should only be
attempted with a fresh reservation on the B-tree/connection edit surface and a
same-window INSERT matrix A/B.
