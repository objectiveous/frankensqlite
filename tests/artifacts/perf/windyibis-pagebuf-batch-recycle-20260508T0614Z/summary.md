# Rejected Page Buffer Batch Recycle Drain

Date: 2026-05-08

Candidate commit:

- `14f67274 perf(pager): batch global recycle pushes under a single Mutex acquisition`

Revert commit:

- `1647b99b Revert "perf(pager): batch global recycle pushes under a single Mutex acquisition"`

Hypothesis:

- After `c65caada`, short-lived page-buffer pools recycle their free buffers
  into a process-global cache on pool drop.
- The candidate changed pool drop to acquire the global recycle mutex once and
  push all eligible buffers in that critical section instead of taking the
  global lock once per drained buffer.
- Expected benefit was lower short-lived connection teardown overhead without
  changing per-buffer reuse semantics.

Correctness:

- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-pagebuf-batch-test-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-pager page_buf -- --nocapture`
- Result: passed, 24 tests.

Benchmark:

- `env FSQLITE_BENCH_PROFILE_INSERT=1 .rch-target/release-perf/comprehensive-bench --quick --filter insert --no-html --json-out tests/artifacts/perf/windyibis-pagebuf-batch-recycle-20260508T0614Z/insert-profile.json`

Focused insert comparison:

| Run | Weighted score | Avg ratio | Geomean | P90 | P99 | Faster / Comparable / C faster |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Prior keeper (`pagebuf-insert-profile.json`) | 0.776732 | 0.771463 | 0.745951 | 1.076081 | 1.098414 | 19 / 2 / 4 |
| Batch recycle candidate | 0.834025 | 0.826022 | 0.803106 | 1.131918 | 1.170809 | 19 / 1 / 5 |

Decision:

- Rejected and reverted before full quick. The focused insert gate worsened on
  the primary score, average ratio, geomean, P90, P99, and C-faster count.
- Do not retry this batch-drain lock-shape as a standalone follow-up to
  `c65caada`.
