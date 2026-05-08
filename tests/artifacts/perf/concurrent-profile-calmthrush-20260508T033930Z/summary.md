# Concurrent Low-Thread Profile and SmallVec Prepared-Commit Probe

Date: 2026-05-08
Agent: CalmThrush
Status: candidate rejected; source restored.

## Baseline

Built current `HEAD` with:

```text
env TMPDIR=/data/tmp/frankensqlite-calmthrush-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-concurrent-calmthrush-target CARGO_BUILD_JOBS=16 cargo build --profile release-perf -p fsqlite-e2e --bin mt-mvcc-bench --bin comprehensive-bench
```

Standalone `mt-mvcc-bench --rows-per-thread=1000 --threads=1,2,4,8 --iters=8`:

| Threads | Baseline throughput ratio | Baseline F ms | Baseline C ms |
| --- | ---: | ---: | ---: |
| 1 | 0.58x | 1.46 | 0.84 |
| 2 | 0.73x | 3.27 | 2.40 |
| 4 | 0.93x | 10.39 | 9.68 |
| 8 | 3.16x | 25.67 | 80.65 |

Comprehensive concurrent filter:

| Scenario | Baseline ratio | Baseline F ms | Baseline C ms |
| --- | ---: | ---: | ---: |
| 2 writers x 1000 rows | 1.0990786735212943 | 13.259783 | 12.064453 |
| 4 writers x 1000 rows | 1.0490765256296313 | 20.304937 | 19.355058 |
| 8 writers x 1000 rows | 0.44206471172925477 | 40.295236 | 91.152347 |

The remaining concurrent gap is concentrated in low-thread rows. The 8-writer
row is already substantially faster than C SQLite.

## Profile

`perf record` on the standalone 2-thread run captured 3,248 samples in
`perf-mt-2t.data`. Kernel symbols were restricted by host perf settings.
Top mixed-process symbols included C SQLite (`sqlite3VdbeExec`,
`sqlite3BtreeTableMoveto`, `sqlite3BtreeInsert`) and FrankenSQLite
(`execute_prepared_direct_simple_insert`,
`execute_prepared_direct_simple_insert_with_cursor`,
`table_seek_for_insert`, `read_cell_pointers_into`, `TransactionKind::get_page`,
`TransactionKind::write_page_data`, `SharedTxnPageIo::with_concurrent`).

## Candidate

`crates/fsqlite-mvcc/src/begin_concurrent.rs` scratch candidate kept prepared
concurrent-commit `write_set_pages` and `held_lock_pages` as
`SmallVec<[PageNumber; 16]>`, avoiding heap `Vec` conversion for the common
small write-set commit plan. This was narrower than the previously rejected
one-pass `page_states` scan: it did not change page-state iteration count or
commit validation semantics.

Correctness and build proof:

- `cargo fmt -p fsqlite-mvcc --check`
- `cargo test -p fsqlite-mvcc commit_updates_commit_index -- --nocapture`
- `cargo test -p fsqlite-mvcc test_prepare_captures_held_lock_pages_separately_from_write_set -- --nocapture`
- `cargo build --profile release-perf -p fsqlite-e2e --bin mt-mvcc-bench --bin comprehensive-bench`

## Result

Standalone candidate:

| Threads | Baseline ratio | Candidate ratio | Baseline F ms | Candidate F ms |
| --- | ---: | ---: | ---: | ---: |
| 1 | 0.58x | 1.11x | 1.46 | 1.17 |
| 2 | 0.73x | 0.70x | 3.27 | 3.33 |
| 4 | 0.93x | 1.01x | 10.39 | 10.04 |
| 8 | 3.16x | 3.22x | 25.67 | 25.16 |

Comprehensive concurrent candidate:

| Scenario | Baseline ratio | Candidate ratio | Baseline F ms | Candidate F ms |
| --- | ---: | ---: | ---: | ---: |
| 2 writers x 1000 rows | 1.0990786735212943 | 1.1265801605368253 | 13.259783 | 14.328216 |
| 4 writers x 1000 rows | 1.0490765256296313 | 0.9645480540965864 | 20.304937 | 19.416062 |
| 8 writers x 1000 rows | 0.44206471172925477 | 0.4146479711003817 | 40.295236 | 38.062030 |

The focused concurrent geomean improved
`0.7988046779013424 -> 0.7666347556689922`, but the actual remaining 2-writer
gap worsened in both the standalone and comprehensive gates. Because the
campaign goal is to close the remaining gaps, this is not a keep.

Source was restored after measurement. Do not retry standalone prepared-commit
page-set `SmallVec` conversion unless a same-window profile proves heap
conversion dominates low-thread commit cost and the 2-writer row improves
without sacrificing the 4/8-writer rows.

