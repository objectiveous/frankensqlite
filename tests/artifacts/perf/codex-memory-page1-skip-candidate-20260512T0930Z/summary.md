# Normal Private-Memory Page-1 Skip Candidate

- Date: 2026-05-12
- Git: `05e8736383c8982c552635785edf91bddb66d716` plus local pager candidate
- Target: `UPDATE/DELETEThroughput` transaction/release envelope

## Candidate

In `crates/fsqlite-pager/src/pager.rs`, the candidate changed normal
`SimpleTransaction::commit()` so private in-memory databases did not stage
page 1 unless the freelist was dirty or page 1 was explicitly dirty. This
mirrored the narrower retained-transaction condition in `commit_and_retain()`.

## Commands

```bash
cargo fmt -p fsqlite-pager --check
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-page1-skip CARGO_BUILD_JOBS=8 cargo test -p fsqlite-pager memory -- --nocapture --test-threads=1
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-page1-skip CARGO_BUILD_JOBS=8 FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --json-out tests/artifacts/perf/codex-memory-page1-skip-candidate-20260512T0930Z/candidate-update.json --no-html
```

## Result

Correctness smoke passed, but the focused update/delete matrix rejected the
candidate. Compared with
`tests/artifacts/perf/codex-update-fixed-overhead-20260512T091030Z/baseline-update.json`,
the update-filter geomean worsened from `1.2075x` to `1.7033x` F/C.

Key row movements:

- `100 rows / update 10 rows`: `1.378x` to `1.911x`.
- `100 rows / delete 5 rows`: `1.512x` to `3.927x`.
- `1000 rows / delete 50 rows`: `1.862x` to `2.113x`.
- `10000 rows / delete 500 rows`: `1.642x` to `1.915x`.

The candidate was unwound. Do not retry normal private-memory page-1 commit
skipping as a standalone transaction-envelope optimization.
