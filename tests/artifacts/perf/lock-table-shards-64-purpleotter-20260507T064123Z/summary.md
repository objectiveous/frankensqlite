# Current HEAD small-DML focused regression

Candidate state measured from local `HEAD` after two peer commits:

- `a18bc551 perf(mvcc): drop LOCK_TABLE_SHARDS from 256 to 64 to cut fresh-DB open allocation`
- `05d5eac5 perf(connection,vdbe): lazy-allocate VersionStore and skip Arc clone for programs that can't read snapshots`

The focused measurement below rejects the combined local `HEAD` state against
the earlier baseline binary. It should not be read as isolated attribution to
the shard-fanout commit alone.

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

## Disposition

Rejected on the focused target before full-matrix promotion for the combined
local `HEAD` binary. Because `05d5eac5` landed between the old baseline binary
and this build, isolate `a18bc551` and `05d5eac5` before assigning blame or
adding a final negative-ledger entry for either individual idea. Do not treat
the current local `HEAD` small-DML result as a keep without a same-window
Section 6 and full quick matrix win that outweigh these focused regressions.
