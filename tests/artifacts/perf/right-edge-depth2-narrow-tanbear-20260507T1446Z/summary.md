# Narrow depth-2 right-edge admission A/B

Date: 2026-05-07
Agent: TanBear
Base commit: `5786202abaee8c4f07730f872cdea2dbd8ba9279`
Detached worktree: `/data/tmp/frankensqlite-tanbear-narrow-20260507T1446Z`

## Candidate shape

This run started from the peer broad depth-2 right-edge page-builder candidate
in `crates/fsqlite-btree/src/cursor.rs` and
`crates/fsqlite-core/src/connection.rs`, then narrowed the non-empty-tree
admission in the detached worktree.

The narrow gate kept `table_bulk_append_depth2_right_edge_sorted_records` only
for arena-backed prepared direct insert page-run records when:

- `records.len() >= 512`
- `record_bytes.len() <= records.len() * 64`

Owned and repeated-record flushes kept the normal path. The intent was to keep
the `small_3col` 10K batched/single-transaction win while avoiding the broad
candidate's 100-row, autocommit, repeated-record, and large-record regressions.

No source changes from this detached candidate were committed in this pass
because the shared checkout still has peer-owned dirty edits in the same two
source files.

## Validation

- `git diff --check` in detached worktree: pass
- `cargo fmt --check` in detached worktree: pass
- `cargo test -p fsqlite-btree test_table_bulk_append_depth2_right_edge_sorted_records_extends_tree -- --nocapture`: pass
- `cargo test -p fsqlite-core test_prepared_direct_insert_page_run_buffers_nonempty_right_edge_batch -- --nocapture`: pass
- `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`: pass

The RCH wrapper could not offload the detached `/data/tmp` worktree because the
worker root normalizer rejected the path, so the build/test commands ran
locally with `/data/tmp` target directories.

## Benchmark artifacts

- `narrow-insert.json`: `--quick --filter insert`
- `narrow-fullquick.json`: first full quick matrix
- `narrow-fullquick-repeat.json`: repeat full quick matrix

## Result summary

Insert-only primary score:

| Run | Score | Faster | C faster | Avg ratio | Geomean |
| --- | ---: | ---: | ---: | ---: | ---: |
| clean insert | 0.902771 | 13 | 8 | 0.924466 | 0.893070 |
| broad dirty insert | 0.921972 | 14 | 9 | 0.924235 | 0.890727 |
| narrow insert | 0.831343 | 19 | 5 | 0.867729 | 0.847539 |

Full quick primary score:

| Run | Score | Faster | C faster | Avg ratio | Geomean | write_bulk avg | write_single avg |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| clean full quick | 0.370335 | 74 | 16 | 0.516276 | 0.286622 | 1.004592 | 1.080311 |
| broad dirty full quick | 0.368076 | 75 | 14 | 0.495578 | 0.280117 | 0.940940 | 1.105308 |
| narrow full quick | 0.376311 | 76 | 12 | 0.507167 | 0.281418 | 0.923585 | 1.222773 |
| narrow full quick repeat | 0.356426 | 79 | 11 | 0.484110 | 0.273391 | 0.884842 | 1.033396 |

The first full quick run missed the clean primary score, mostly because
untouched UPDATE/DELETE rows were noisier. The repeat cleared both clean and
broad. Averaging the two narrow full quick scores gives `0.366369`, slightly
better than clean `0.370335` and broad `0.368076`.

## Decision

Promising, but not source-landed here. The narrowed admission is the first
right-edge depth-2 shape that preserves the insert win while making the full
quick matrix look plausibly better than clean. It should still get a clean
same-run A/B before landing in source, because the first full quick run missed
the keep gate and the dirty source files are owned by another agent.

Do not revive the broad admission. It is already fenced in
`docs/progress/perf-negative-results.md`.
