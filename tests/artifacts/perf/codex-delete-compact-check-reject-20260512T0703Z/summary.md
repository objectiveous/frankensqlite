# Prepared Direct DELETE Compact-Check Candidate Rejection

- Date: 2026-05-12T07:03Z
- Base commit: `3437480e5fe256a884bbb6e590ffe6e273b9a962`
- Worktree: `/data/tmp/frankensqlite-perf-frontier-20260512-0710`
- Target dir: `/data/tmp/frankensqlite-target-frontier-0710`
- Candidate touched: `crates/fsqlite-btree/src/cursor.rs`

## Candidate

Move the `TableLeafDeleteRun::has_compact_cell_area` check from every accepted
row in `delete_rowid_with_reason` to `BtCursor::table_leaf_delete_run_current`,
leaving the existing flush-time materialization validation in place.

Hypothesis: the retained leaf image is private until flush, so repeatedly
rescanning its cell-pointer array for compactness on every accepted same-leaf
DELETE is redundant work on the 10k-row DELETE profile.

## Commands

Baseline profile at current HEAD:

```bash
rch exec -- env FSQLITE_BENCH_PROFILE_DML=1 CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --no-html
```

Candidate profile:

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --no-html
```

Candidate narrow compare:

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 10000 40 delete compare standard
```

Same-target baseline narrow compare after unwinding the candidate:

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 10000 40 delete compare standard
```

## Results

The quick profile initially looked promising:

| Run | 10k DELETE FSQLite | `delete_leaf_active_ns` | `delete_leaf_materialize` | `delete_leaf_flush_ns` |
| --- | ---: | ---: | ---: | ---: |
| Baseline | `410.8 us` | `49854 ns` | `64/54643 ns` | `69190 ns` |
| Candidate | `329.3 us` | `46502 ns` | `64/57550 ns` | `71645 ns` |

The focused same-target compare did not confirm a reliable keep:

| Run | FSQLite per deleted row | C SQLite per deleted row | Delete ratio |
| --- | ---: | ---: | ---: |
| Candidate | `506 ns` | `345 ns` | `1.46x` |
| Baseline after unwind | `529 ns` | `368 ns` | `1.43x` |

## Verdict

Rejected and unwound. The candidate's intended active-path counter moved only
about 3.4 us on the 10k-row profile, materialization/flush got slightly worse,
and the narrow compare showed only a 4% absolute FSQLite movement while the
ratio failed to improve. That is within the local noise envelope and below the
bar for carrying another retained-leaf micro-special case.

Retry only if a future profile proves compactness rescans dominate a larger
fraction of the retained DELETE path and a same-window A/B improves the
absolute FSQLite 10k DELETE median by more than 10% without worsening the
focused compare ratio.
