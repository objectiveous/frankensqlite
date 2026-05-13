# DML DELETE Compare Profile Refresh

- Date: 2026-05-13T00:20Z
- Note: the artifact directory name contains `T0120Z`, but the run started at
  `T0020Z`; the directory name is left unchanged to avoid rename/delete churn.
- Source: `7ea5da35` (`docs(perf): publish f11324ca benchmark refresh`)
- Build command: `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-dml-current-target cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete`
- Profile command shape: `FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-dml-current-target/release-perf/perf-update-delete <rows> 100 delete compare standard`

## Results

| Workload | FSQLite per-row delete | C SQLite per-row delete | F/C delete ratio | Profile log |
|---|---:|---:|---:|---|
| 100 rows / delete 5 rows | 2269 ns | 432 ns | 5.25x | `delete-100.log` |
| 1000 rows / delete 50 rows | 931 ns | 318 ns | 2.93x | `delete-1000.log` |
| 10000 rows / delete 500 rows | 980 ns | 372 ns | 2.63x | `delete-10000.log` |

All three runs stayed on the prepared direct DELETE path (`slow=0`).

Representative final-iteration counters:

- 100 rows: `delete_leaf_start=1/1`, `delete_leaf_active=4/4`, `delete_leaf_flush=1/1`, `delete_leaf_materialize=1/841ns`, `delete_leaf_write=1/141ns`, `execute_body_ns=4869`, `commit_roundtrip_ns=1483`.
- 1000 rows: `delete_leaf_start=6/6`, `delete_leaf_active=44/49`, `delete_leaf_miss=5`, `delete_leaf_flush=6/6`, `delete_leaf_materialize=6/3978ns`, `delete_leaf_write=6/740ns`, `execute_body_ns=11772`, `commit_roundtrip_ns=3065`.
- 10000 rows: `delete_leaf_start=64/67`, `delete_leaf_active=433/496`, `delete_leaf_miss=63`, `delete_leaf_flush=64/64`, `delete_leaf_materialize=64/88828ns`, `delete_leaf_write=64/16590ns`, `execute_body_ns=74851`, `commit_roundtrip_ns=35997`.

## Decision

No source patch was attempted from this refresh. The current frontier remains the known physical retained DELETE materialization/write boundary. The existing pending same-leaf and monotone cross-leaf buffering already preserves read boundaries and rollback behavior, but it still mutates page-local delete-run state and materializes dirty leaf pages before publication. A smaller search/materializer/flush tweak would repeat the fenced 2026-05-09 through 2026-05-12 attempts.

The next source attempt should be the broader transaction-local DML mutation operator described in `docs/design/profile-first-optimization-cards-and-proof-packs.md`, with read-boundary flushing or a logical read-view overlay, savepoint/rollback ownership, row-count oracle tests, focused `--quick --filter update` wins, and full-quick primary-score neutrality.
