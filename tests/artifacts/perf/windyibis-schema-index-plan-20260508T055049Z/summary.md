# Page Buffer Cross-Pool Recycle Verification

Date: 2026-05-08

Commit under test:

- `c65caada perf(pager): cross-pool global recycle for dropped page buffers (zero-fill on reuse)`

Candidate:

- Add a bounded process-local recycle cache for page buffers drained from dropped
  `PageBufPoolInner` instances.
- Reuse only buffers with the same page size.
- Zero-fill the aligned page window before handing the buffer to a new pool, so
  first-acquire semantics remain unchanged.
- Scope is intentionally limited to short-lived pool/connection churn. Long-lived
  pools keep their existing per-pool free-list behavior.

Validation:

- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-pagebuf-test-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-pager page_buf -- --nocapture`
- `env TMPDIR=/data/tmp/frankensqlite-windyibis-tmp CARGO_TARGET_DIR=.rch-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
- `env FSQLITE_BENCH_PROFILE_DML=1 .rch-target/release-perf/comprehensive-bench --quick --filter update --no-html --json-out tests/artifacts/perf/windyibis-schema-index-plan-20260508T055049Z/pagebuf-update-profile.json`
- `env FSQLITE_BENCH_PROFILE_INSERT=1 .rch-target/release-perf/comprehensive-bench --quick --filter insert --no-html --json-out tests/artifacts/perf/windyibis-schema-index-plan-20260508T055049Z/pagebuf-insert-profile.json`
- `.rch-target/release-perf/comprehensive-bench --quick --no-html --json-out tests/artifacts/perf/windyibis-schema-index-plan-20260508T055049Z/pagebuf-full-quick.json`

Full quick gate:

| Run | Weighted score | Avg ratio | Geomean | P90 | P99 | Faster / Comparable / C faster |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Clean baseline (`calmthrush-clean-noprofile-20260508T0219Z`) | 0.345939 | 0.454261 | 0.267475 | 0.981159 | 1.401515 | 80 / 5 / 8 |
| Prior landed state (`windyibis-current-dirty-20260508T051120Z`) | 0.351285 | 0.471355 | 0.267868 | 1.045104 | 1.607150 | 78 / 6 / 9 |
| Page-buffer recycle candidate | 0.335899 | 0.442035 | 0.259341 | 1.057587 | 1.242234 | 81 / 2 / 10 |

Decision:

- Keep. The primary metric improved from `0.351285` to `0.335899` versus the
  immediately prior state and from `0.345939` to `0.335899` versus the clean
  baseline.
- The full-matrix P99 also improved materially versus both references.
- P90 is slightly worse than both references, and C-fast rows increased from 8
  on the clean baseline to 10, so the change is not a total closeout.

Focused profiles:

- Insert focused run: 25 scenarios, 19 FrankenSQLite faster, 2 comparable, 4 C
  SQLite faster, average ratio `0.771463`, weighted score `0.776732` over the
  observed write categories.
- Update/delete focused run: 6 scenarios, 3 FrankenSQLite faster, 1 comparable,
  2 C SQLite faster, average ratio `1.094475`, weighted score `1.064187` over
  the observed write-single category.

Largest remaining full-matrix gaps after this commit:

| Ratio | Category | Scenario |
| ---: | --- | --- |
| 1.242234 | write_single | 100 rows / delete 5 rows |
| 1.155391 | write_bulk | small_3col, 100 rows / single txn |
| 1.120684 | concurrent_writers | 2 writers x 1000 rows |
| 1.114018 | write_bulk | large_10col, 10K rows / single txn |
| 1.112973 | write_single | small_3col, 100 rows / autocommit |

Notes:

- This is a memory-retention tradeoff: the global recycle cache keeps at most
  256 backing buffers per process. It is deliberately bounded and only stores
  page-sized aligned buffers.
- The unit test added in the commit verifies that a buffer recycled from one
  pool into another is returned zeroed.
