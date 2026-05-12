# Prepared Direct DELETE Borrowed-Write Candidate Rejection

- Date: 2026-05-12T07:16Z
- Base commit: `0881d3960d52bccd63d469dd567cd6b1515d2e32`
- Worktree: `/data/tmp/frankensqlite-perf-frontier-20260512-0710`
- Target dir: `/data/tmp/frankensqlite-target-frontier-0710`
- Candidate touched: `crates/fsqlite-btree/src/cursor.rs`

## Candidate

Change `BtCursor::flush_table_leaf_delete_run_in_place` from:

```rust
self.pager.write_page_data(cx, leaf_page, run.entry.page_data.clone())?;
```

to:

```rust
self.pager.write_page(cx, leaf_page, run.entry.page_data.as_bytes())?;
```

Hypothesis: writing the materialized page through the borrowed slice path would
avoid cloning a 4 KiB `PageData` for each retained DELETE leaf-run flush.

## Commands

Focused proof:

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 cargo fmt --check
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 cargo test -p fsqlite-btree table_leaf_delete_run -- --nocapture
```

Candidate narrow compare:

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 10000 40 delete compare standard
```

Candidate focused DML profile:

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --no-html
```

## Results

Focused tests passed and the narrow compare looked positive in isolation:

| Run | FSQLite per deleted row | C SQLite per deleted row | Delete ratio |
| --- | ---: | ---: | ---: |
| Candidate narrow compare | `451 ns` | `338 ns` | `1.33x` |
| Prior same-target baseline from compact-precheck rejection | `529 ns` | `368 ns` | `1.43x` |

The profiling counters rejected the hypothesis:

| Run | 10k DELETE FSQLite | `delete_leaf_write` | `delete_leaf_flush_ns` | `page_pool_misses` |
| --- | ---: | ---: | ---: | ---: |
| Candidate profile | `424.3 us` | `64/23945 ns` | `81933 ns` | `62` |
| Earlier same-worktree baseline profile | `410.8 us` | `64/8484 ns` | `69190 ns` | `1` |

## Verdict

Rejected and unwound. The borrowed `write_page` route lost the owned
`PageData` adoption path used by `write_page_data`; the page-pool miss count
rose from roughly one miss to one miss per flushed leaf. That made the intended
write/flush counters substantially worse despite one favorable narrow compare.

Do not retry borrowed-slice publication for retained DELETE leaf runs as a
standalone optimization. Reconsider only if `write_page` grows an owned-buffer
steal/adoption equivalent, or if a same-window DML profile proves the write
counter and page-pool misses improve together.
