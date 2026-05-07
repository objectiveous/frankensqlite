# Non-empty direct INSERT page-run candidate

Date: 2026-05-07
Agent: PurpleOtter
Status: connection-level candidate rejected and removed from source; B-tree
cursor-stack correctness guard retained.

## Target

`INSERTThroughput - Transaction Strategy Comparison (small_3col)`,
especially `10000 rows / batched (1000/txn)`, which remained slower than C
SQLite after the retained append-hint work.

## Candidate Shape

The candidate allowed prepared direct INSERT page-run buffering to start after
the table was already non-empty by using the retained append hint as proof that
the new explicit rowid was strictly beyond the current right edge. The first
right-edge row still inserted normally; later rows in the batch were buffered
and flushed at the normal read/savepoint/commit boundary.

During correctness testing this exposed a separate B-tree cursor bug:
`table_append_after_last_position` could restore only a cached child leaf after
a right-edge split and later treat that leaf as a depth-1 root. The cursor-stack
guard and regression test for that bug were kept separately.

## Correctness Proof Before Measurement

After adding the cursor-stack guard:

```bash
TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-nonempty-pagerun-btree-target \
  cargo test -p fsqlite-btree \
  test_table_append_after_last_position_repeated_after_existing_rows_crosses_split -- --nocapture

TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-nonempty-pagerun-target \
  cargo test -p fsqlite-core prepared_direct_insert_page_run -- --nocapture
```

Both focused gates passed before the benchmark run. After the benchmark
rejected the connection-level candidate, the `connection.rs` changes were
removed and the original three page-run tests still passed.

## Measurement

Candidate build:

```bash
TMPDIR=/data/tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-nonempty-pagerun-perf-target \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

Candidate benchmark:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-nonempty-pagerun-perf-target/release-perf/comprehensive-bench \
  --quick \
  --filter transaction \
  --json-out tests/artifacts/perf/nonempty-pagerun-purpleotter-20260507T1030Z/candidate-transaction.json \
  --no-html
```

Raw stdout and stderr are in this directory.

## Result

Rejected. The target row worsened versus the retained append-hint transaction
profile:

| Scenario | Baseline F ms | Baseline ratio | Candidate F ms | Candidate ratio |
| --- | ---: | ---: | ---: | ---: |
| 10000 rows / batched (1000/txn) | `4.289494` | `1.3290062724722278` | `4.76` | `1.4408254018260138` |

The candidate did reduce per-row cursor setup on the target row
(`cursor_setup_ns` fell from about `410210` to `14856`) and lowered the profiled
`btree_insert_ns`, but the work moved into commit-time replay:

- `commit_us` rose to `3524.8`.
- `btree_cell_assembly_calls` rose to `9000`.
- `btree_leaf_full_cell_appends` rose to `8943`.
- `btree_leaf_payload_appends` dropped to `0`.

This means the buffering shape lost the cheap payload-append path and replayed
nearly the whole batch as full-cell appends at commit. Do not retry this as
append-hint-started non-empty buffering plus row-at-a-time commit replay.

## Retained B-tree Guard Check

After removing the rejected `connection.rs` changes, only the B-tree
cursor-stack guard remained. It was rebuilt into the same release-perf target
and measured separately:

```bash
/data/tmp/frankensqlite-nonempty-pagerun-perf-target/release-perf/comprehensive-bench \
  --quick \
  --filter transaction \
  --json-out tests/artifacts/perf/nonempty-pagerun-purpleotter-20260507T1030Z/btreeguard-transaction.json \
  --no-html

/data/tmp/frankensqlite-nonempty-pagerun-perf-target/release-perf/comprehensive-bench \
  --quick \
  --json-out tests/artifacts/perf/nonempty-pagerun-purpleotter-20260507T1030Z/btreeguard-full.json \
  --no-html
```

Focused transaction result with only the guard:

- `10000 rows / batched (1000/txn)`: FSQLite `4.251529 ms`, ratio
  `1.2777334488590038`.
- Transaction-section primary weighted score: `0.8663945593492034`.

Full quick matrix with only the guard:

- Total scenarios: `93`.
- Franken faster / comparable / C faster: `76 / 3 / 14`.
- Geomean ratio: `0.27868765456581224`.
- Primary weighted score: `0.36394897123082987`.

The guard did not reproduce the rejected candidate's target-row regression.

## Retry Condition

Reconsider non-empty monotonic batching only if the flush path has a real
non-empty page builder or direct payload writer that preserves the payload-append
kernel. A retry must show, before full-matrix work, that the target row improves
in absolute FrankenSQLite median time and that the profile does not replace
cursor setup with commit-time full-cell assembly.
