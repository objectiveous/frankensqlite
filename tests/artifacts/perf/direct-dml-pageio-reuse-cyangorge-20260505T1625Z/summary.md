# Direct DML Page I/O Reuse Candidate - CyanGorge - 2026-05-05T1625Z

## Scope

Independent validation of the current dirty `crates/fsqlite-core/src/connection.rs`
and `crates/fsqlite-vdbe/src/engine.rs` candidate that reuses a parked
`SharedTxnPageIo` wrapper for prepared concurrent direct INSERT/UPDATE/DELETE.

The source files were exclusively reserved by PurpleCoast while this was
measured, so this artifact is a handoff only. I did not edit, stage, or revert
those source files.

## Candidate Shape Observed

- Adds `Connection::prepared_direct_dml_page_io: RefCell<Option<SharedTxnPageIo>>`.
- Adds `Connection::direct_dml_page_io_with_concurrent(...)` to refill a parked
  wrapper with the current pager transaction and concurrent MVCC context.
- Adds `Connection::park_direct_dml_page_io(...)` to drain the transaction back
  out and keep the wrapper parked.
- Adds `SharedTxnPageIo::refill_concurrent(...)` and
  `SharedTxnPageIo::drain_reusable_transaction(...)`.
- Replaces the per-execution `SharedTxnPageIo::with_concurrent(...)` calls in
  direct INSERT/UPDATE/DELETE with the reusable wrapper.

## Behavior Proof

Focused direct-simple test run against the dirty candidate:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-dml-pageio-target cargo test -p fsqlite-core direct_simple -- --nocapture
```

Result:

- `36` matching `fsqlite-core` direct-simple tests passed.
- The run included prepared direct INSERT tests and direct UPDATE/DELETE
  fast-path tests.
- The remote test command itself exited `0`; the local RCH process was later
  terminated only after it had finished the test and then spent several minutes
  retrieving the remote target directory.

## Benchmark Command

The benchmark run compiled locally because `rch exec` treated the shell wrapper
as a non-compilation command:

```bash
rch exec -- bash -lc 'env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-dml-pageio-release-target cargo run -p fsqlite-e2e --bin comprehensive-bench --profile release-perf -- --quick --filter insert --json-out tests/artifacts/perf/direct-dml-pageio-reuse-cyangorge-20260505T1625Z/candidate-report.json --no-html > tests/artifacts/perf/direct-dml-pageio-reuse-cyangorge-20260505T1625Z/candidate-stdout.txt 2> tests/artifacts/perf/direct-dml-pageio-reuse-cyangorge-20260505T1625Z/candidate-stderr.txt'
```

Baseline comparison used the existing clean insert report:

`tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`

## Aggregate Result

Baseline clean report:

- Insert weighted score: `1.699053`
- Average ratio: `2.460952x`
- Geomean ratio: `2.362302x`
- `write_bulk` geomean: `2.515348x`
- `write_single` geomean: `1.490767x`

Candidate dirty report:

- Insert weighted score: `1.637155`
- Average ratio: `2.493287x`
- Geomean ratio: `2.382967x`
- `write_bulk` geomean: `2.559594x`
- `write_single` geomean: `1.410574x`

The primary weighted score improves because the benchmark weighting favors
`write_single`, but the average/geomean ratios and `write_bulk` geomean move
the wrong way. Since the current campaign is targeting the remaining slowest
insert/update/delete rows, this is not a clean keep.

## FSQLite Median Movement on Important Rows

Rows where the candidate improved FSQLite absolute time:

- `large_10col` 1K single transaction: `2.069145 ms -> 1.881363 ms` (`-9.08%`).
- `tiny_1col` 100 single transaction: `0.266789 ms -> 0.254336 ms` (`-4.67%`).
- `large_10col` 100 single transaction: `0.396833 ms -> 0.387616 ms` (`-2.32%`).

Rows where the candidate regressed important target shapes:

- `large_10col` 10K single transaction: `36.165071 ms -> 37.664367 ms` (`+4.15%`).
- record-size `large_10col` 10K: `37.055950 ms -> 39.948064 ms` (`+7.80%`).
- record-size `medium_6col` 10K: `9.888943 ms -> 10.520894 ms` (`+6.39%`).
- `small_3col` 10K batched 1K/txn: `6.668030 ms -> 7.343192 ms` (`+10.13%`).
- `small_3col` 10K single transaction: `6.608088 ms -> 7.897029 ms` (`+19.51%`).
- `medium_6col` 1K single transaction: `1.404401 ms -> 1.689895 ms` (`+20.33%`).
- `small_3col` 1K single transaction: `0.805519 ms -> 1.128433 ms` (`+40.09%`).
- `medium_6col` 100 single transaction: `0.270346 ms -> 0.419807 ms` (`+55.29%`).

## Disposition

Treat this candidate as rejected/mixed unless a same-machine paired A/B repeat
shows the slowest `write_bulk` rows improving without the large/medium/small
single-transaction regressions above. The direct-simple correctness tests are
green, so the issue is benchmark value, not an observed functional failure.
