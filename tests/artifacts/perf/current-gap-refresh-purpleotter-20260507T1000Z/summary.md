# Current Gap Refresh - Depth-2 Right-Edge Bulk Append Attempt

Date: 2026-05-07
Agent: PurpleOtter
Baseline git: `def2db18a9b6c200b10f4e7cf4f2eeddb3d24bb3`

## Target

`INSERTThroughput - Transaction Strategy Comparison (small_3col)` remained
behind C SQLite on `10000 rows / batched (1000/txn)`:

- Baseline FSQLite median: `4.305162 ms`
- Baseline C SQLite median: `3.338762 ms`
- Baseline ratio: `1.2894486040035198`

The profile showed repeated right-edge btree work in the batched row:

- `row_build_ns=1601161`
- `cursor_setup_ns=390852`
- `btree_insert_ns=1531829`
- `btree_cell_assembly_calls=123`
- `btree_leaf_payload_appends=8934`
- `btree_quick_balance_hits=57`
- `btree_conservative_reloads=57`

## Candidate

Scratch source added a narrow depth-2 non-empty right-edge bulk append primitive
in `crates/fsqlite-btree/src/cursor.rs` and called it from the direct insert
page-run flush paths in `crates/fsqlite-core/src/connection.rs`.

Correctness/build checks passed before measurement:

```bash
TMPDIR=/data/tmp cargo fmt -p fsqlite-btree -p fsqlite-core
TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-bulkappend-target cargo test -p fsqlite-btree test_table_bulk_append_depth2_right_edge_sorted_records_extends_tree -- --nocapture
TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-bulkappend-target cargo test -p fsqlite-core test_prepared_direct_insert_page_run_flushes_before_read -- --nocapture
TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-bulkappend-target cargo test -p fsqlite-core test_prepared_direct_simple_insert_executes_inside_explicit_transaction -- --nocapture
TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-bulkappend-perf-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

## Measurement

Baseline:

```bash
TMPDIR=/data/tmp FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-current-gap-target/release-perf/comprehensive-bench --quick --filter transaction --json-out tests/artifacts/perf/current-gap-refresh-purpleotter-20260507T1000Z/report-transaction.json --no-html
```

Candidate:

```bash
TMPDIR=/data/tmp FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-bulkappend-perf-target/release-perf/comprehensive-bench --quick --filter transaction --json-out tests/artifacts/perf/current-gap-refresh-purpleotter-20260507T1000Z/candidate-transaction.json --no-html
```

Section aggregate appeared better, but the target row regressed:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Total scenarios | 9 | 9 |
| FSQLite faster | 5 | 6 |
| C SQLite faster | 4 | 3 |
| Average ratio | `1.0210660576003165` | `0.9924704539025738` |
| Geomean ratio | `1.0025333414848634` | `0.9776479846338291` |
| Primary weighted score | `1.0031958927979898` | `0.9054630878224692` |
| Target C median | `3.338762 ms` | `3.224106 ms` |
| Target F median | `4.305162 ms` | `4.376543 ms` |
| Target ratio | `1.2894486040035198` | `1.3574438929737422` |

The target-row profile still reported the same right-edge event shape:

- Baseline `btree_insert_ns=1531829`
- Candidate `btree_insert_ns=1517958`
- Baseline and candidate both had `btree_leaf_payload_appends=8934`
- Baseline and candidate both had `btree_quick_balance_hits=57`
- Baseline and candidate both had `btree_conservative_reloads=57`

## Decision

Rejected and manually removed from source. The benchmark row this candidate was
intended to fix moved the wrong way: FSQLite median worsened by `0.071381 ms`
and the ratio worsened by `0.0679952889702224`.

The most likely cause is that the new flush hook did not activate for the hot
benchmark shape. `try_buffer_prepared_direct_insert_page_run_with_cursor`
currently starts a direct page run only when `cursor.last(cx)?` reports an empty
tree, so after the first batch the non-empty right-edge bulk primitive has no
pending run to flush.

Retry only after connection-level buffering can safely form non-empty monotonic
insert runs, with proof that the new btree primitive is actually invoked and the
target-row FSQLite median improves before any full-matrix keep decision.
