# Deferred Direct DML Microbatch Carry Dirty Evaluation

Date: 2026-05-08T17:29Z

## Scope

Read-only evaluation of the dirty `crates/fsqlite-core/src/connection.rs`
candidate held by SwiftGate. RusticGrove did not edit or stage that source file.

Candidate shape observed in `dirty-connection.diff`: extend statement
microbatch proof carry to deferred direct UPDATE/DELETE dispatch, skipping
per-row prepared schema renewal on in-transaction repeated direct DML calls.

## Inputs

- Clean baseline: detached worktree
  `/data/tmp/frankensqlite-rusticgrove-clean-microbatch-20260508T1729Z`
  at `c06f2410cdf0a75e6f8344c006f7e95d92c412aa`.
- Dirty candidate: shared worktree at the same `HEAD` plus the
  `connection.rs` diff captured in `dirty-connection.diff`.
- Artifact directory reservation:
  `tests/artifacts/perf/rusticgrove-microbatch-dirty-eval-20260508T1729Z/**`.

## Verification

- Dirty targeted correctness test:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-rusticgrove-microbatch-dirty-target CARGO_BUILD_JOBS=12 cargo test -p fsqlite-core test_stmt_microbatch_coalesces_repeated_update_delete -- --nocapture`
  - Remote command reached `exit=0`; see `dirty-microbatch-test.stderr`.
  - `rch` then hung retrieving `.rch-target`; only the retrieval process was
    terminated after the test result was already captured.
- Dirty benchmark binary:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-rusticgrove-microbatch-dirty-target-local CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
- Clean benchmark binary:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-rusticgrove-microbatch-clean-target-local CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`

## Benchmark Command

Both clean and dirty binaries were run with:

```text
env FSQLITE_BENCH_PROFILE_DML=1 <binary> --quick --filter update --json-out <json> --no-html
```

## Results

| Run | Faster / Comparable / C faster | Avg | Geomean | Median | P90 | Weighted |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Clean | 3 / 1 / 2 | 0.975842 | 0.959935 | 0.959373 | 1.323705 | 0.959935 |
| Dirty | 0 / 3 / 3 | 1.078065 | 1.070111 | 1.050020 | 1.378579 | 1.070111 |
| Clean repeat | 3 / 1 / 2 | 1.088422 | 1.058634 | 0.963800 | 1.578668 | 1.058634 |
| Dirty repeat | 2 / 2 / 2 | 1.114729 | 1.092257 | 0.997542 | 1.511301 | 1.092257 |

Target-row movement:

- First run: dirty regressed `1000 rows / update 100 rows`
  `0.382076 ms -> 0.405018 ms` and `10000 rows / update 1000 rows`
  `3.039041 ms -> 3.703404 ms` versus clean.
- Repeat: dirty still lost the section gate versus clean repeat
  (`1.092257` vs `1.058634` geomean) and worsened the small tails:
  `100 rows / update 10 rows` ratio `1.364696` and
  `100 rows / delete 5 rows` ratio `1.511301`.

Profile counters did not show the desired section-level payoff. The candidate
kept one schema refresh per statement in the profile logs and moved enough
overhead into the direct DML path that the focused DML gate lost.

## Decision

Rejected as a standalone optimization. Do not land or retry this deferred
direct UPDATE/DELETE microbatch schema-proof carry shape unless a future design
first proves same-window focused DML improvement and then survives a broader
quick matrix. The current dirty candidate does not beat clean `HEAD` on the
focused UPDATE/DELETE section, including the repeat.

## Ledger Blocker

`docs/progress/perf-negative-results.md` was reserved by SwiftGate when this
decision was made, so RusticGrove did not edit through the lock. A patch-ready
negative-ledger entry should record this rejection under the same wording as
the decision above.
