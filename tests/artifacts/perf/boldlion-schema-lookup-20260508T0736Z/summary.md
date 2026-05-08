# Prepared Direct INSERT Schema Lookup Rejection

Date: 2026-05-08
Agent: BoldLion
Base commit: `7563874a02e99881ae5466f4a6c121f3f17d572d`
Candidate source: dirty tree with one local `crates/fsqlite-core/src/connection.rs`
edit, later restored.

## Candidate

In `prepared_direct_simple_insert_plan`, the candidate replaced the linear
case-insensitive schema scan:

```text
schema.iter().find(|table| table.name.eq_ignore_ascii_case(table_name))
```

with the existing `schema_index_of(table_name)` side-index lookup. The direct
INSERT eligibility checks and execution path were otherwise unchanged.

## Proof Before Measurement

- `cargo fmt -p fsqlite-core --check`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-schema-check-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_prepared_insert_precomputes_direct_simple_insert_plan -- --nocapture`
- `env CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`

All passed.

## Measurement

- Candidate INSERT:
  `env FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/cargo-target/release-perf/comprehensive-bench --quick --filter insert --no-html --json-out tests/artifacts/perf/boldlion-schema-lookup-20260508T0736Z/candidate-insert.json`
- Candidate UPDATE/DELETE:
  `env FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/cargo-target/release-perf/comprehensive-bench --quick --filter update --no-html --json-out tests/artifacts/perf/boldlion-schema-lookup-20260508T0736Z/candidate-update.json`

## Result

Rejected and manually reverted.

The INSERT filter was mixed versus
`tests/artifacts/perf/boldlion-dml-next-20260508T0622Z/pagebuf-revert-insert.json`:

| Metric | Prior post-pagebuf | Candidate |
| --- | ---: | ---: |
| Average ratio | 0.8442288062 | 0.8281973547 |
| Geomean ratio | 0.8145695151 | 0.7982037686 |
| P90 ratio | 1.1272106776 | 1.1662110161 |
| P99 ratio | 1.2931548041 | 1.3405968544 |

The UPDATE/DELETE filter rejected the candidate:

| Scenario | Candidate ratio | Candidate C | Candidate F |
| --- | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 1.6681464056 | 0.079068 ms | 0.131897 ms |
| 100 rows / update 10 rows | 1.4013856256 | 0.082995 ms | 0.116308 ms |

No full quick run was justified because the focused DML tail was already worse
than the current post-pagebuf full quick p99 tail (`1.6681464056` vs
`1.4337080362`).

## Follow-Up Boundary

Do not retry this lookup swap as a standalone prepared direct INSERT setup
optimization. It is only worth revisiting as part of a broader prepared statement
setup redesign that wins the focused DML tails and the full quick matrix in the
same measurement window.
