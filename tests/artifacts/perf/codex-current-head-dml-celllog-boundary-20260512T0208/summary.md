# Current-HEAD DML Cell-Log Boundary Check

Date: 2026-05-12

Commit under review: `ed8a950e1f58cdf6f5fd5193b3987c87f84a0b8b`

## Commands

Current focused DML profile:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-current-dml-target CARGO_BUILD_JOBS=2 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-codex-current-dml-target/release-perf/comprehensive-bench \
  --quick --filter update \
  --json-out tests/artifacts/perf/codex-current-head-dml-profile-20260512T0208/update-delete-profile.json \
  --no-html \
  2>&1 | tee tests/artifacts/perf/codex-current-head-dml-profile-20260512T0208/stdout.txt
```

The first rebuild attempt with `CARGO_TARGET_DIR=/tmp/...` failed during RCH
artifact retrieval because `/tmp` was full. The `/data/tmp` rebuild succeeded.
The benchmark still warns that the binary predates HEAD because the latest
intervening commits were docs/artifact-only.

## Focused Profile

`UPDATE/DELETEThroughput` remained in the same shape:

- `100 rows / update 10 rows`: C `0.004158 ms`, F `0.005721 ms`, `1.376x`.
- `100 rows / delete 5 rows`: C `0.002234 ms`, F `0.010029 ms`, `4.489x`.
- `1000 rows / update 100 rows`: C `0.036047 ms`, F `0.026780 ms`, `0.743x`.
- `1000 rows / delete 50 rows`: C `0.015909 ms`, F `0.027462 ms`, `1.726x`.
- `10000 rows / update 1000 rows`: C `0.361768 ms`, F `0.245149 ms`, `0.678x`.
- `10000 rows / delete 500 rows`: C `0.162494 ms`, F `0.260478 ms`, `1.603x`.

All DELETE rows stayed on the prepared direct path (`slow=0`). The 500-row
DELETE row still reported:

```text
delete_qf_ns=1658
delete_seek_ns=33388
delete_physical_ns=14217
delete_leaf_start=64/67
delete_leaf_active=433/496
delete_leaf_miss=63
delete_leaf_flush=64/64
delete_leaf_flush_ns=85020
delete_leaf_materialize=64/71452
delete_leaf_write=64/7558
commit_us=44.3
```

## Fresh Source Check

The tempting next step was to connect the existing cell-level MVCC scaffolding
to prepared direct DELETE. That is not safe as a narrow patch:

- `CellVisibilityLog` can record and commit row deltas, and lifecycle tests
  prove manually pushed logical pages can update `commit_index`.
- The active B-tree cursor path reads pages through `PageReader::read_btree_page_data`
  and does not consult `CellVisibilityLog`.
- `SharedTxnPageIo` and `TransactionPageIo` expose page reads/writes and
  witnesses, not a logical cell-delete operation.
- The existing direct DELETE path physically mutates a retained
  `TableLeafDeleteRun` and publishes it with `write_page_data`.

Recording `cell_log.record_delete()` from the direct DELETE path without a new
materialized read view would make the statement report affected rows while the
base B-tree page still contains those rows for read-your-writes, savepoints, and
later scans. Manually pushing pages into `txn.write_set` would only fix
commit-index publication, not in-transaction visibility.

## Outcome

No source patch was landed. The credible next lever is still the broader
transaction-local DML mutation operator, but its first implementation must add a
real read/materialization boundary across B-tree page reads, transaction-local
logical deltas, rollback/savepoint state, quotient-filter/count-cache
invalidation, and MVCC publication. A standalone direct DELETE cell-log hook is
blocked.
