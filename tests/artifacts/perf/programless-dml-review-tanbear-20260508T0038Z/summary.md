# Programless prepared UPDATE/DELETE review

Run time: 2026-05-08T00:38Z-00:45Z

Scope: read-only review of the shared dirty `crates/fsqlite-core/src/connection.rs`
candidate that skips reusable table-program compilation for direct-simple
prepared UPDATE/DELETE and dispatches through the cached direct-DML metadata.

I did not author or stage the source diff. Captured diff:
`programless-dml-dirty.diff`.

## Candidate shape

- Add `PreparedUpdateDeleteFastPath::has_direct_simple_dispatch()`.
- For direct-simple prepared UPDATE/DELETE, build a placeholder program and set
  `PreparedStatement::db` to `None` instead of compiling a reusable table
  program.
- In `execute_precompiled_prepared_update_or_delete`, allow programless direct
  dispatch when fast-path metadata is valid, fused-entry mode is automatic, and
  statement tracing gates are off.
- Keep indexed/deferred UPDATE/DELETE on the existing reusable-program path.

## Correctness and local checks

All commands used the dirty shared tree and
`CARGO_TARGET_DIR=/data/tmp/frankensqlite-programless-dml-target`.

- `cargo fmt -p fsqlite-core --check` passed.
- `git diff --check -- crates/fsqlite-core/src/connection.rs` passed.
- `cargo test -p fsqlite-core test_direct_simple_update_prepare_skips_compiled_program -- --nocapture` passed.
- `cargo test -p fsqlite-core test_direct_simple_delete_prepare_skips_compiled_program -- --nocapture` passed.
- `cargo test -p fsqlite-core test_programless_prepared_update_delete_forced_fallback_use_deferred_path -- --nocapture` passed.
- `cargo test -p fsqlite-core test_execute_prepared_deferred_delete_direct_dispatch_counts_as_fast_path -- --nocapture` passed.

The release-perf candidate binary already existed at:
`/data/tmp/frankensqlite-programless-dml-bench-target/release-perf/comprehensive-bench`.

Baseline binary:
`/data/tmp/frankensqlite-smallvec-isolated-target/release-perf/comprehensive-bench`.

## Focused update/delete gate

Six alternating same-window `--quick --filter update` pairs were run. Primary
fields are `.summary`: `score / avg / geomean / p90 / p99 /
csqlite_faster / franken_faster`.

| Run | Baseline | Candidate |
| --- | --- | --- |
| 1 | `1.125120 / 1.141700 / 1.125120 / 1.421776 / 1.421776 / 2 / 1` | `1.151887 / 1.188721 / 1.151887 / 1.872177 / 1.872177 / 3 / 0` |
| 2 | `1.200248 / 1.216561 / 1.200248 / 1.511684 / 1.511684 / 4 / 0` | `1.019875 / 1.041768 / 1.019875 / 1.351713 / 1.351713 / 2 / 2` |
| 3 | `1.135156 / 1.144800 / 1.135156 / 1.464860 / 1.464860 / 4 / 0` | `1.070394 / 1.086619 / 1.070394 / 1.380737 / 1.380737 / 2 / 2` |
| 4 | `1.142772 / 1.165662 / 1.142772 / 1.531614 / 1.531614 / 2 / 0` | `1.115417 / 1.125218 / 1.115417 / 1.345590 / 1.345590 / 3 / 1` |
| 5 | `1.155072 / 1.172362 / 1.155072 / 1.482268 / 1.482268 / 3 / 0` | `1.135535 / 1.142710 / 1.135535 / 1.367410 / 1.367410 / 4 / 0` |
| 6 | `1.101577 / 1.120467 / 1.101577 / 1.447237 / 1.447237 / 3 / 2` | `1.119438 / 1.131224 / 1.119438 / 1.361533 / 1.361533 / 3 / 1` |

Median focused score moved from about `1.1390` to `1.1174` and median p90 moved
from about `1.4736` to `1.3645`.

## Full quick gate

Two alternating full `--quick` pairs were run.

| Run | Baseline | Candidate |
| --- | --- | --- |
| 1 | `0.350727 / 0.464332 / 0.269937 / 1.057744 / 1.519072 / 10 / 79` | `0.347246 / 0.457003 / 0.267584 / 1.025899 / 1.383672 / 8 / 81` |
| 2 | `0.353885 / 0.467876 / 0.273468 / 1.085331 / 1.533168 / 11 / 79` | `0.347734 / 0.459959 / 0.269619 / 1.044789 / 1.446846 / 9 / 79` |

Full-matrix verdict: keep candidate direction. The primary score improved in
both full quick runs, p90/p99 improved, and C-faster rows dropped.

## Remaining risk

This artifact is not a final landing proof because I did not own the dirty
source diff. Before landing, the owner should run the normal workspace gates
for the source change and stage only the intended `connection.rs` diff plus a
concise artifact bundle.
