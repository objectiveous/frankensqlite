# Lazy Fallback Page-Lock Shard Candidate

Rejected candidate measured on 2026-05-07. Source was reverted after the full
quick matrix failed the keep gate.

## Candidate Shape

- `crates/fsqlite-mvcc/src/core_types.rs`
- Changed `InProcessPageLockTable.shards` from eager
  `Box<[LockShard; LOCK_TABLE_SHARDS]>` to a lazy `OnceLock` table.
- Fast-array page locks stayed allocation-free for the fallback table.
- Page numbers above `FAST_LOCK_ARRAY_SIZE` allocated fallback shards on first
  use.
- Count/holder/release paths skipped fallback shard allocation when no high-page
  lock had ever been acquired.
- Rolling rebuild used `OnceLock::take()` to rotate the fallback table.

## Correctness / Build

- `cargo fmt -p fsqlite-mvcc --check` passed.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-lazy-fallback-shards-target cargo test -p fsqlite-mvcc in_process_lock_table -- --nocapture` passed: 8 passed, 1 ignored.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-lazy-fallback-shards-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete` passed.

## Measurement Caveat

This artifact is retained for audit only. It is not a valid standalone A/B
result.

The intended baseline was the read-only private page-cache shard candidate:
`tests/artifacts/perf/private-page-cache-shards-crimsongorge-20260507T0755Z/candidate-full.json`.
While this candidate was being built, that pager diff was reverted in the shared
tree and a separate dirty `crates/fsqlite-core/src/connection.rs` candidate
appeared. The candidate binary therefore compared different dirty-tree states.

Candidate JSON:
`tests/artifacts/perf/lazy-fallback-lock-shards-crimsongorge-20260507T0820Z/candidate-full.json`.

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Primary weighted score | 0.371643 | 0.382836 |
| Average ratio | 0.489023 | 0.524049 |
| Geomean ratio | 0.277578 | 0.283457 |
| Median ratio | 0.291390 | 0.289615 |
| P90 ratio | 1.121600 | 1.184145 |
| P99 ratio | 1.496608 | 1.584330 |
| FrankenSQLite faster rows | 74 | 71 |
| Comparable rows | 5 | 5 |
| C SQLite faster rows | 14 | 17 |
| Write-bulk geomean | 0.854591 | 0.961601 |
| Write-single geomean | 1.147031 | 1.258971 |

These numbers are not a keep/reject proof for the lazy fallback lock-shard idea.
The source patch was reverted; retry only from a clean worktree or after active
peer-owned dirty source has landed or reverted.
