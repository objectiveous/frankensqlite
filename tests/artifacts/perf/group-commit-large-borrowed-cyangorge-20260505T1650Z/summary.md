# Large Borrowed WAL Commit Candidate - CyanGorge - 2026-05-05T1650Z

## Scope

Attempted a narrow pager candidate for very large WAL commits after the insert
CPU profile showed `pager::build_group_commit_batch` cloning staged page bytes
into owned `TransactionFrameBatch` frames.

The attempted source change was in `crates/fsqlite-pager/src/pager.rs` and was
reverted after measurement.

## Candidate Shape

- Promoted the existing borrowed `collect_wal_commit_batch` helper out of
  `#[cfg(test)]`.
- Added a `BORROWED_WAL_DIRECT_COMMIT_MIN_FRAMES = 512` threshold.
- For commits above that threshold, tried to append borrowed frame refs directly
  while still using:
  - the pinned WAL conflict snapshot,
  - WAL backend prepare/finalize/append validation,
  - the DB-file `Reserved` lock,
  - the WAL sync policy,
  - and the `inner.db_size` update.

The goal was to avoid cloning large staged-page sets into an owned
`TransactionFrameBatch` when the transaction is already too large to benefit
from ordinary 64-frame group batching.

## Correctness Checks

Passed:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-pager-candidate-target cargo test -p fsqlite-pager test_collect_wal_commit_batch -- --nocapture
```

Result: `4` tests passed.

Passed when serialized:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-pager-candidate-target cargo test -p fsqlite-pager group_commit -- --nocapture --test-threads=1
```

Result: `22` tests passed.

The same `group_commit` filter without `--test-threads=1` produced fault-hook
interference between tests. The serialized rerun passed, so I did not treat
that parallel-filter run as a candidate-specific correctness failure.

## Benchmark Command

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-dml-pageio-release-target cargo run -p fsqlite-e2e --bin comprehensive-bench --profile release-perf -- --quick --filter insert --json-out tests/artifacts/perf/group-commit-large-borrowed-cyangorge-20260505T1650Z/candidate-report.json --no-html
```

Baseline comparison used:

`tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`

## Result

Baseline clean report:

- Insert weighted score: `1.699053`
- Average ratio: `2.460952x`
- Geomean ratio: `2.362302x`
- `write_bulk` geomean: `2.515348x`
- `write_single` geomean: `1.490767x`

Candidate dirty report:

- Insert weighted score: `1.787694`
- Average ratio: `2.459143x`
- Geomean ratio: `2.390798x`
- `write_bulk` geomean: `2.526914x`
- `write_single` geomean: `1.592921x`

Important FSQLite medians did not produce a clean large-row win:

- `large_10col` 10K single transaction: baseline `36.165071 ms`, candidate
  `37.493052 ms`.
- record-size `large_10col` 10K: baseline `37.055950 ms`, candidate
  `37.160930 ms`.
- record-size `medium_6col` 10K: baseline `9.888943 ms`, candidate
  `11.164965 ms`.

## Caveat

While this run was in progress, an unrelated dirty
`crates/fsqlite-btree/src/cursor.rs` diff appeared in the shared worktree. That
means the benchmark is not a clean isolated A/B for the pager patch alone.
Because the aggregate score and target rows were already not a keep, I reverted
only my `pager.rs` patch and did not spend more time reconstructing an isolated
repeat.

Disposition: abandoned/reverted. Do not retry this exact borrowed large-commit
threshold without an isolated A/B and a proof that it still preserves the
group-commit fault/publish semantics.
