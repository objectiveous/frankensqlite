# Direct UPDATE/DELETE Microbatch Proof Carry Recheck

Date: 2026-05-07
Agent: PurpleOtter
Baseline: detached `HEAD` worktree at `43817d8f`
Candidate: local direct UPDATE/DELETE schema-proof microbatch carry patch

## Candidate

The candidate allowed direct-simple prepared UPDATE/DELETE statements in an
explicit transaction to use the statement microbatch schema/function proof carry
even though those prepared statements conservatively set
`may_observe_change_tracking`.

The source change was rejected and the checkout is restored to clean `HEAD`.

## Correctness Checks

- `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_stmt_microbatch_coalesces_repeated_direct_update_delete -- --nocapture`
- `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture`
- `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_fast_path_update_delete_ddl_invalidation -- --nocapture`
- `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_entry_proof_no_publication_for_memory_update_delete -- --nocapture`

All focused checks passed.

## Bench Commands

- Candidate build: `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-update-delete-candidate-perf CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete`
- Baseline worktree: `/data/tmp/frankensqlite-baseline-direct-update-delete-20260507T115335Z`
- Baseline build: `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-update-delete-baseline-perf CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete`
- Section A/B: `comprehensive-bench --quick --filter update --json-out <report> --no-html`
- Profiled section A/B: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update --json-out <report> --no-html`
- Isolated A/B: `perf-update-delete <rows> <iters> <update|delete> compare isolated`

## Result

Rejected.

The no-profile section geomean improved from `1.2245037883938406` to
`1.1289754225301574`, but the per-row isolated harness rejected the candidate on
the update rows:

| Workload | Baseline FSQLite | Candidate FSQLite |
| --- | ---: | ---: |
| 100 rows / update | 635 ns/row | 694 ns/row |
| 1000 rows / update | 782 ns/row | 827 ns/row |
| 10000 rows / update | 844 ns/row | 907 ns/row |
| 100 rows / delete | 1156 ns/row | 1161 ns/row |
| 1000 rows / delete | 1103 ns/row | 1164 ns/row |
| 10000 rows / delete | 1221 ns/row | 1248 ns/row |

The remaining gap is not dominated by the schema/function proof. Do not retry
this standalone proof-carry idea; revisit only as part of a retained direct-DML
cursor/run design that removes cursor creation and root-to-leaf seek work.
