# Deferred UPDATE/DELETE Microbatch Carry Rejection - 2026-05-08

## Scope

Measured a one-lever candidate on top of `c06f2410` that extended the
existing prepared-statement microbatch carry from repeated in-transaction
INSERT calls to the deferred direct UPDATE/DELETE path.

## Candidate

Touched `crates/fsqlite-core/src/connection.rs` only.

The candidate let repeated direct-simple UPDATE/DELETE statements inside an
explicit transaction skip `ensure_schema_unchanged_with_prebound_publication()`
after the first renewal when the statement program pointer, bind arity, schema
cookie, schema generation, function-registry generation, explicit-transaction
state, and concurrent-session id matched the cached microbatch epoch.

## Correctness Proof

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-swiftgate-ud-microbatch-check \
  CARGO_BUILD_JOBS=8 \
  cargo test -p fsqlite-core test_stmt_microbatch_coalesces_repeated_update_delete -- --nocapture
```

Result: passed. The temporary test verified that repeated direct UPDATE and
DELETE calls both produced microbatch hits and preserved the final row count.

```bash
cargo fmt --check -p fsqlite-core
```

Result: passed after the candidate was restored.

## Benchmark Commands

Baseline used `/data/tmp/frankensqlite-swiftgate-current-dml-target/release-perf/perf-update-delete`.
Candidate used `/data/tmp/frankensqlite-swiftgate-ud-microbatch-target/release-perf/perf-update-delete`.

```bash
perf-update-delete 100 20000 update compare isolated
perf-update-delete 100 20000 delete compare isolated
perf-update-delete 100 1000 both compare standard
```

## Results

| Scenario | Baseline FSQLite | Candidate FSQLite | Decision |
|---|---:|---:|---|
| isolated update, per row | 661 ns | 651 ns | small isolated improvement |
| isolated delete, per row | 1707 ns | 1706 ns | unchanged |
| standard both, update per row | 1418 ns | 1510 ns | worse |
| standard both, delete per row | 1768 ns | 1934 ns | worse |

The candidate did not meet the keep gate. It slightly improved the isolated
UPDATE loop but did not move DELETE and worsened the standard 100-row DML run
that includes the real benchmark setup/prepare/transaction envelope.

## Decision

Rejected and source restored. Do not retry deferred UPDATE/DELETE microbatch
carry as a standalone DML optimization. Reconsider only if future profiling
shows `ensure_schema_unchanged_with_prebound_publication()` dominating the
direct UPDATE/DELETE mutate phase and a same-window focused UPDATE/DELETE
matrix improves both the isolated mutation loop and the standard section gate.
