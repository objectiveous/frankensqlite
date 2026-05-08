# Small Record Append Serializer Gate

Date: 2026-05-08
Agent: BoldLion
Source commit: `565158958cc06c39d82be7364b3949790fdca545`

## Change

`crates/fsqlite-types/src/record.rs` now sends precomputed-header records with
`max_body_size <= 384` through the append serializer, while preserving the
existing single-slot path and the fixed-slice path for wider records.

This is a size-gated retry of the earlier broad append serializer candidate:
small direct-DML rows get the cheaper append construction, but wide rows such as
`large_10col` stay off the path that previously regressed the benchmark matrix.

## Focused INSERT Gate

Baseline: `tests/artifacts/perf/windyibis-frontier-refresh-20260508T0745Z/head756-insert-profile.json`

Candidate: `candidate-insert.json`

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | `0.8220829871` | `0.7916321420` |
| Average ratio | `0.8372309293` | `0.8134270995` |
| Geomean ratio | `0.8013196058` | `0.7907960961` |
| P90 ratio | `1.1073353870` | `1.0936426700` |
| P99 ratio | `1.6871174266` | `1.1332489753` |
| Faster / comparable / slower | `17 / 4 / 4` | `18 / 2 / 5` |

Rows above `0.95x` in the focused candidate:

| Ratio | Section | Row |
| ---: | --- | --- |
| `1.133249` | INSERT txn strategy small_3col | `100 rows / batched (100/txn)` |
| `1.110058` | INSERT txn strategy small_3col | `100 rows / single txn` |
| `1.093643` | INSERT single txn large_10col | `100 rows` |
| `1.081025` | INSERT single txn small_3col | `100 rows` |
| `1.054972` | INSERT single txn medium_6col | `100 rows` |
| `1.000034` | INSERT single txn large_10col | `10000 rows` |
| `0.967266` | INSERT record size large_10col | `10K rows` |

## Full Quick Gate

Baseline: `tests/artifacts/perf/windyibis-frontier-refresh-20260508T0745Z/head756-full-quick.json`

Candidate: `candidate-full-quick.json`

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | `0.3470296740` | `0.3432318915` |
| Average ratio | `0.4646237940` | `0.4518673317` |
| Geomean ratio | `0.2645872563` | `0.2620653981` |
| P90 ratio | `1.0877373564` | `1.0070909280` |
| P99 ratio | `1.6217478587` | `1.4215818420` |
| Faster / comparable / slower | `79 / 3 / 11` | `81 / 4 / 8` |

Rows above `0.95x` in the candidate full quick:

| Ratio | Section | Row |
| ---: | --- | --- |
| `1.421582` | INSERT single txn medium_6col | `100 rows` |
| `1.387707` | UPDATE/DELETEThroughput | `100 rows / update 10 rows` |
| `1.324468` | UPDATE/DELETEThroughput | `100 rows / delete 5 rows` |
| `1.129074` | Concurrent Writers | `2 writers x 1000 rows` |
| `1.115393` | INSERT txn strategy small_3col | `100 rows / batched (100/txn)` |
| `1.101077` | INSERT txn strategy small_3col | `100 rows / single txn` |
| `1.070846` | INSERT single txn large_10col | `100 rows` |
| `1.067521` | INSERT single txn small_3col | `100 rows` |
| `1.010545` | INSERT single txn tiny_1col | `100 rows` |
| `1.007091` | Concurrent Writers | `4 writers x 1000 rows` |
| `0.988483` | UPDATE/DELETEThroughput | `1000 rows / update 100 rows` |
| `0.968519` | INSERT single txn large_10col | `10000 rows` |

The `medium_6col` 100-row row did not reproduce in the focused INSERT gate
(`1.054972x` there) and had high FSQLITE CV in the full quick run. The aggregate
full matrix still improved, so the source candidate is a keeper.

## Verification

Commands run:

```text
cargo fmt --check
ubs crates/fsqlite-types/src/record.rs
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-small-append-check-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-types precomputed_header -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
env FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/cargo-target/release-perf/comprehensive-bench --quick --filter insert --no-html --json-out tests/artifacts/perf/boldlion-small-record-append-20260508T0800Z/candidate-insert.json
env CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=8 /data/tmp/cargo-target/release-perf/comprehensive-bench --quick --no-html --json-out tests/artifacts/perf/boldlion-small-record-append-20260508T0800Z/candidate-full-quick.json
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-small-append-workspace-target CARGO_BUILD_JOBS=8 cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-small-append-workspace-target CARGO_BUILD_JOBS=8 cargo clippy --workspace --all-targets -- -D warnings
```

Results:

- `cargo fmt --check`: passed.
- Focused `fsqlite-types` tests: passed, 6 tests.
- Workspace `cargo check --workspace --all-targets`: passed through `rch`.
- Workspace `cargo clippy --workspace --all-targets -- -D warnings`: passed through `rch`.
- UBS exited 0 with no critical findings; it reported pre-existing warning
  inventories in `record.rs`.
