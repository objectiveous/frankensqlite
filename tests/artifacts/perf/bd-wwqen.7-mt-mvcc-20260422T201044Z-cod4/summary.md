# bd-wwqen.7 mt-mvcc benchmark rerun attempt

Run id: `bd-wwqen.7-mt-mvcc-20260422T201044Z-cod4`
Date: 2026-04-22
Agent: `cod4`

## Command

```bash
rch exec -- env CARGO_TARGET_DIR=/home/ubuntu/rch_target_fsqlite_cod4 cargo run --profile release-perf -p fsqlite-e2e --bin mt-mvcc-bench -- --rows-per-thread=1000 --threads=1,8,16 --iters=10
```

## Intended measurement

- Latest April MT fairness benchmark: `fsqlite-e2e` binary `mt-mvcc-bench`.
- Requested thread counts: 1, 8, 16.
- Requested statistics: p50/p95/p99 for FrankenSQLite and SQLite oracle.
- Baseline comparison target: 2026-04-19 average ratio `2.74x`.

## Result

The rerun was blocked before the benchmark binary emitted rows. `mt_mvcc_bench.stdout` is empty. Raw RCH and compiler output is in `mt_mvcc_bench.stderr`.

The benchmark code in HEAD already contains percentile output support:

- `655648dc feat(e2e/mt-mvcc-bench): p50/p95/p99 reporting over iterations`
- `1a9178ea feat(e2e/mt-mvcc-bench): add p95/p99 throughput columns to output table`

## Blocker

Remote RCH worker `ts2` reached cargo execution, but `fsqlite-core` failed to compile from unrelated dirty `connection.rs` changes before `mt-mvcc-bench` could run.

Primary compiler failures from `mt_mvcc_bench.stderr`:

- `error[E0599]`: no method named `execute_writable_schema_insert` found for `&connection::Connection`.
- `error[E0599]`: no method named `execute_writable_schema_update` found for `&connection::Connection`.
- `error[E0061]`: `eval_group_agg_join_expr` now takes 4 arguments, but multiple call sites in `crates/fsqlite-core/src/connection.rs` still pass 3.

Representative arity-error locations:

- `crates/fsqlite-core/src/connection.rs:36295`
- `crates/fsqlite-core/src/connection.rs:36613`
- `crates/fsqlite-core/src/connection.rs:36662`
- `crates/fsqlite-core/src/connection.rs:38294`
- `crates/fsqlite-core/src/connection.rs:38546`
- `crates/fsqlite-core/src/connection.rs:38594`
- `crates/fsqlite-core/src/connection.rs:52062`
- `crates/fsqlite-core/src/connection.rs:52071`
- `crates/fsqlite-core/src/connection.rs:52103`
- `crates/fsqlite-core/src/connection.rs:52140`
- `crates/fsqlite-core/src/connection.rs:52141`
- `crates/fsqlite-core/src/connection.rs:52142`
- `crates/fsqlite-core/src/connection.rs:52179`
- `crates/fsqlite-core/src/connection.rs:52185`

Because no benchmark rows were produced, there is no valid p50/p95/p99 table and no valid average-ratio comparison against the 2026-04-19 `2.74x` baseline for this attempt.
