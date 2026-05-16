# Current DML Profile Refresh

- Date: 2026-05-15 22:45Z
- Repository: `/data/projects/frankensqlite`
- Git `HEAD`: `06a37f61e0ad97ffa95449f2f97a27ea080c821c`
- Worktree: dirty; the benchmark was run during the ALTER TABLE rename repair pass before the later trigger bare-column follow-up fix.
- Build/profile command:
  `rch exec -- env FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-current-dml-profiled-20260515T224517Z/update-delete.json --no-html`
- Local artifacts: `run.log`. RCH reported `JSON report written to: tests/artifacts/perf/codex-current-dml-profiled-20260515T224517Z/update-delete.json`, but that JSON was written on the remote worker and was not retrieved locally.

## Scenario Results

| Scenario | C SQLite | FrankenSQLite | Ratio | Direction | C CV | F CV |
|---|---:|---:|---:|---|---:|---:|
| 100 rows / update 10 rows | 6.4 us | 8.8 us | 1.38x | slower | 5.1% | 10.7% |
| 100 rows / delete 5 rows | 6.5 us | 9.8 us | 1.52x | slower | 9.4% | 4.3% |
| 1000 rows / update 100 rows | 54.9 us | 55.8 us | 1.02x | comparable/slower | 41.3% | 36.9% |
| 1000 rows / delete 50 rows | 23.5 us | 61.3 us | 2.61x | slower | 28.8% | 41.0% |
| 10000 rows / update 1000 rows | 548.1 us | 388.7 us | 0.71x | faster | 6.7% | 23.9% |
| 10000 rows / delete 500 rows | 262.7 us | 421.3 us | 1.60x | slower | 23.0% | 26.3% |

Summary: 6 scenarios, FrankenSQLite faster on 1, comparable on 1, C SQLite faster on 4. Average F/C time ratio was 1.47x.

## DML Profile Highlights

The DML profile confirms the remaining gap is not VDBE fallback:

- `fs_delete_100`: `direct_delete=5`, `slow=0`, `vdbe_opcodes=0`, `delete_leaf_flush=1/1`, `delete_leaf_search=5/741ns`.
- `fs_delete_1000`: `direct_delete=50`, `slow=0`, `vdbe_opcodes=0`, `delete_leaf_flush=6/6`, `delete_leaf_search=55/10138ns`, `delete_leaf_active=44/49`, `delete_leaf_miss=5`.
- `fs_delete_10000`: `direct_delete=500`, `slow=0`, `vdbe_opcodes=0`, `delete_leaf_flush=64/64`, `delete_leaf_search=560/89746ns`, `delete_leaf_active=433/496`, `delete_leaf_miss=63`, `delete_leaf_materialize=64/86529ns`, `delete_leaf_flush_ns=108954`.
- `fs_update_10000`: `direct_update=1000`, `slow=0`, `vdbe_opcodes=0`; the large update row remains faster than C SQLite despite `bg_checks=1004`.

The `setup_us` counters are outside the measured update/delete row. They measure fixture prepopulation in `profile_fsqlite_update_delete_dml`, so they are not evidence for a source optimization in the measured DML mutation body.

## Interpretation

This refresh supports the existing negative-ledger boundary:

- Do not retry standalone retained DELETE search/admission/materialization, direct flush/publication, cancellation polling, QF/count-cache tweaks, or another narrow retained-run micro-patch.
- The remaining DELETE loss is distributed across repeated leaf search, active retained leaf work, materialization/flush, and page-local compaction/cell parsing.
- The only source-level attempt still worth considering is the broader transaction-local DML mutation operator: collect logical rowid/key mutation messages, group/sort by leaf, mutate each dirty leaf once, publish once, and prove read-view/savepoint/rollback/MVCC invalidation semantics.

No source performance patch was attempted from this artifact.
