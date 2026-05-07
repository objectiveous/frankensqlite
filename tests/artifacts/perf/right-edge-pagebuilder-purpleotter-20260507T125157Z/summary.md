# Non-Empty Page-Run Writer Flush Rejection

- Date: 2026-05-07T12:51:57Z
- Baseline HEAD: `7660b8da docs(perf): refresh full matrix gap map`
- Baseline worktree: `/data/tmp/frankensqlite-baseline-pagebuilder-20260507T125157Z`
- Candidate patch: `crates/fsqlite-core/src/connection.rs`

## Candidate

Allowed direct INSERT page-runs to start on a non-empty right edge when the
next explicit rowid was greater than the table's last rowid. On flush, replayed
records through the existing `table_append_after_last_position_with_writer`
payload-writer kernel before falling back to the byte-slice append path.

Focused proof passed:

```text
cargo fmt -p fsqlite-core
env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-pagebuilder-coretest-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core prepared_direct_insert_page_run -- --nocapture
```

## Transaction Keep Gate

Command shape:

```text
env FSQLITE_BENCH_PROFILE_INSERT=1 <bin>/comprehensive-bench --quick --filter transaction --json-out <artifact>.json --no-html
```

Summary:

| Metric | Baseline | Candidate | Result |
|---|---:|---:|---|
| primary weighted score | 0.9403114839 | 0.9287374158 | improves |
| geomean ratio | 0.9786733776 | 1.0159826210 | regresses |
| write-bulk geomean | 1.0104867009 | 1.0916437903 | regresses |
| write-single geomean | 0.9180199506 | 0.8800289112 | improves |
| 10000 rows / batched FSQLite | 4.548586 ms | 4.686143 ms | rejects |
| 10000 rows / batched ratio | 1.3666644733 | 1.4288776944 | rejects |
| 10000 rows / single txn FSQLite | 2.637083 ms | 3.482457 ms | rejects |

Profile signal on the target row:

- Baseline `fs_insert_txn_batched_small_3col_10000`:
  `insert_us=8438.1`, `commit_us=160.0`, `cursor_setup_ns=393556`,
  `btree_insert_ns=1455606`, `btree_leaf_payload_appends=8934`,
  `btree_leaf_full_cell_appends=9`.
- Candidate:
  `insert_us=5724.9`, `commit_us=3186.4`, `cursor_setup_ns=12906`,
  `btree_insert_ns=300381`, `btree_leaf_payload_appends=8943`,
  `btree_leaf_full_cell_appends=0`.

The candidate moved work out of the per-row insert body but paid it back at
commit, and the benchmark row that motivated the change got slower.

## Decision

Rejected. Do not retry non-empty page-run buffering plus writer-flush replay as
a standalone `connection.rs` optimization. Revisit only with a true page builder
that lays out the whole non-empty right-edge run and its parent updates in one
batch, with an absolute improvement on `10000 rows / batched (1000/txn)` before
any full-matrix repeat.
