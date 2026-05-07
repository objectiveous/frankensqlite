# Current HEAD shard/version-store matrix check

Candidate state measured from local `HEAD` after two peer commits:

- `a18bc551 perf(mvcc): drop LOCK_TABLE_SHARDS from 256 to 64 to cut fresh-DB open allocation`
- `05d5eac5 perf(connection,vdbe): lazy-allocate VersionStore and skip Arc clone for programs that can't read snapshots`

The focused probe below looked worse than the earlier baseline binary, but the
full quick matrix improved. Treat the matrix result as the keep gate.

## Scope

- Target workload: small `:memory:` UPDATE/DELETE fixed cost where profiles showed `SharedMvccState::new` and fallback MVCC metadata allocation during fresh connection setup.
- Touched source: `crates/fsqlite-mvcc/src/core_types.rs`.
- Candidate shape: current local `HEAD` built after the fallback shard fanout
  reduction and lazy `VersionStore` changes, compared with the pre-commit
  baseline binary at `/data/tmp/frankensqlite-current-autocommit-target`.

## Correctness proof

- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-lockshards64-target cargo test -p fsqlite-mvcc commit_index -- --nocapture`
  - Result: 16 passed.
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purpleotter-lockshards64-target cargo test -p fsqlite-mvcc in_process_lock_table -- --nocapture`
  - Result: 7 passed, 1 ignored.

## Focused same-window measurement

Commands:

- Baseline: `/data/tmp/frankensqlite-current-autocommit-target/release-perf/perf-update-delete 100 20000 delete fsqlite standard`
- Candidate local `HEAD`: `/data/tmp/frankensqlite-purpleotter-lockshards64-perf-target/release-perf/perf-update-delete 100 20000 delete fsqlite standard`
- Baseline: `/data/tmp/frankensqlite-current-autocommit-target/release-perf/perf-update-delete 100 20000 update fsqlite standard`
- Candidate local `HEAD`: `/data/tmp/frankensqlite-purpleotter-lockshards64-perf-target/release-perf/perf-update-delete 100 20000 update fsqlite standard`

Results:

| Row | Baseline | Candidate local `HEAD` | Direction |
| --- | ---: | ---: | --- |
| 100-row DELETE standard | 2351 ns/deleted row | 2522 ns/deleted row | regressed 7.3% |
| 100-row UPDATE standard | 1425 ns/updated row | 1597 ns/updated row | regressed 12.1% |

## Full quick matrix

Command:

- `/data/tmp/frankensqlite-purpleotter-lockshards64-perf-target/release-perf/comprehensive-bench --quick --json-out tests/artifacts/perf/lock-table-shards-64-purpleotter-20260507T064123Z/report-full.json --no-html`

Compared with `tests/artifacts/perf/current-head-after-autocommit-preserialize-purpleotter-20260507T0615Z/report-full.json`:

| Metric | Baseline | Candidate local `HEAD` | Direction |
| --- | ---: | ---: | --- |
| Primary weighted score | 0.390728 | 0.370574 | improved |
| Geomean ratio | 0.290045 | 0.277310 | improved |
| Median ratio | 0.307856 | 0.292624 | improved |
| C SQLite faster rows | 20 | 14 | improved |
| FrankenSQLite faster rows | 70 | 73 | improved |
| Write-single geomean | 1.250352 | 1.192892 | improved |
| Write-bulk geomean | 0.967384 | 0.951178 | improved |
| Concurrent-writers geomean | 0.770590 | 0.742122 | improved |

## Disposition

Keep at the matrix level for the combined local `HEAD` state. The focused
100-row `perf-update-delete` probe regressed, but the full quick matrix improved
the primary score and reduced C-faster rows. Because `a18bc551` and `05d5eac5`
were not isolated here, do not attribute the matrix win to either individual
commit without an isolated same-window run.
