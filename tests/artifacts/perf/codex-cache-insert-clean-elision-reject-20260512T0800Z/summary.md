# Sharded Cache Clean-Mark Elision Rejection - 2026-05-12

## Candidate

Removed the redundant `ShardedPageCache::mark_page_clean()` call after
`insert_buffer()`, relying on `CachedPageEntry::new()` to install clean
replacement entries. Added focused tests for dirty-entry replacement on the
standard sharded path and the single-connection fast-array path.

## Correctness Proof Before Benchmark

- `CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 cargo fmt -p fsqlite-pager --check`
- `CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 cargo test -p fsqlite-pager insert_buffer_replaces_dirty_entry -- --nocapture`
- `CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 cargo test -p fsqlite-pager test_sharded_cache_fast_path_insert_buffer -- --nocapture`

All passed before the benchmark rejection.

## Baseline

Current HEAD `2377512a`, pre-candidate focused run:

- Command: `CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter UPDATE --no-html`
- 50-row DELETE: C `15.4us`, F `28.2us`, `pager_cache_finish_ns=1273ns`
- 500-row DELETE: C `157.1us`, F `269.3us`, `pager_cache_finish_ns=12193ns`

## Candidate Runs

Same command with dirty candidate binary:

- Run 1: 50-row DELETE C `40.4us`, F `58.0us`, `pager_cache_finish_ns=1683ns`; 500-row DELETE C `218.9us`, F `339.5us`, `pager_cache_finish_ns=11822ns`
- Run 2: 50-row DELETE C `29.1us`, F `56.3us`, `pager_cache_finish_ns=1423ns`; 500-row DELETE C `205.2us`, F `437.0us`, `pager_cache_finish_ns=11902ns`

## Decision

Rejected and unwound. The targeted 500-row cache-finish counter moved by only a
few hundred nanoseconds and did not improve the focused DELETE rows. The
remaining DML tail is dominated by retained leaf-run materialization/write work
and pager memory flush/cache insertion at a larger granularity, not this
standalone clean-mark probe.
