# Prepared Root PageNumber Probe

- Date: 2026-05-08
- Agent: CrimsonGorge
- Base commit: `de62f313`
- Candidate source diff: `candidate-connection.diff`
- Benchmark isolation: clean `HEAD` archive under
  `/data/tmp/frankensqlite-prepared-root-page-20260508T035302Z`, with only the
  candidate `connection.rs` patch applied for candidate timing.

## Candidate

Prepared direct INSERT, UPDATE, and DELETE metadata kept the legacy
`root_page: i32` for existing cache identity and tests, but also decoded a
`PageNumber` once at prepare time. Direct DML execution then used the prepared
`PageNumber` instead of calling `page_number_from_schema_root(...)` on every
row.

The hypothesis was that a tiny prepare-time conversion might remove repeated
checked conversion/error-formatting ceremony from the hot direct DML cursor
setup path.

## Correctness Proof

- `cargo fmt -p fsqlite-core --check` passed.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-prepared-root-target CARGO_BUILD_JOBS=12 cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture`
  passed: 28 matching tests.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-prepared-root-target CARGO_BUILD_JOBS=12 cargo test -p fsqlite-core update_delete -- --nocapture`
  passed: 9 core tests plus matching integration/conformance tests.

## Focused UPDATE/DELETE Gate

Same-window `comprehensive-bench --quick --filter update` rejected the
candidate:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | 1.1015072810860902 | 1.2277880043578617 |
| Average ratio | 1.1138399195630244 | 1.2418121222187701 |
| Geomean ratio | 1.1015072810860902 | 1.2277880043578617 |
| p90 ratio | 1.3975804587368041 | 1.5230680435137203 |
| p99 ratio | 1.3975804587368041 | 1.5230680435137203 |
| C-faster rows | 2 | 4 |
| FrankenSQLite-faster rows | 0 | 0 |

Row movement:

| Row | Baseline ratio | Candidate ratio | Baseline F ms | Candidate F ms |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 1.3062074100816823 | 1.4052339040411757 | 0.115777 | 0.119584 |
| 100 rows / delete 5 rows | 1.3975804587368041 | 1.5230680435137203 | 0.108824 | 0.121387 |
| 1000 rows / update 100 rows | 0.9786963691203022 | 1.0479210979438873 | 0.375103 | 0.413845 |
| 1000 rows / delete 50 rows | 0.9679242672780094 | 0.9881983055515411 | 0.342832 | 0.362399 |
| 10000 rows / update 1000 rows | 1.0214739318027262 | 1.2422802276180076 | 3.552385 | 4.419679 |
| 10000 rows / delete 500 rows | 1.0111570803586227 | 1.2441711546442895 | 3.233468 | 4.053002 |

## Decision

Rejected and reverted from source. Do not retry prepare-time root
`PageNumber` caching as a standalone prepared direct DML optimization. The
conversion was too small to dominate, and the extra prepared metadata made the
focused UPDATE/DELETE gate materially worse.

The negative-results ledger was exclusively reserved by CalmThrush at capture
time, so this artifact is the canonical evidence and a patch-ready ledger entry
was sent over Agent Mail instead of editing through the lock.
