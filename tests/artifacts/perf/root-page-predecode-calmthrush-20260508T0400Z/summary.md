# Prepared Direct DML Root PageNumber Predecode A/B

Date: 2026-05-08
Agent: CalmThrush
Status: rejected after reverse-order full quick repeat.

## Scope

This pass measured the uncommitted `crates/fsqlite-core/src/connection.rs`
candidate already present in the shared checkout. The file was under
CrimsonGorge's exclusive Agent Mail reservation, so this pass stayed read-only
with respect to source and produced artifacts only.

Candidate diff:

- `candidate-connection.diff`

Candidate shape:

- Add cached `PageNumber root` to prepared direct INSERT, UPDATE, and DELETE
  metadata.
- Decode schema `root_page` once during prepare.
- Use the cached `PageNumber` in direct execution instead of calling
  `page_number_from_schema_root(...)` on each prepared direct DML execution.

## Build

Baseline was a clean `git archive HEAD` copy at:

```text
/data/tmp/frankensqlite-root-predecode-baseline-20260508T0400Z
```

Baseline build:

```text
env TMPDIR=/data/tmp/frankensqlite-root-predecode-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-root-predecode-baseline-target \
  CARGO_BUILD_JOBS=16 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Candidate build:

```text
env TMPDIR=/data/tmp/frankensqlite-root-predecode-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-root-predecode-candidate-target \
  CARGO_BUILD_JOBS=16 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Both release-perf builds passed.

## Focused Gates

No-profile INSERT filter:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | 0.8792619192210196 | 0.7564531538440045 |
| Average ratio | 0.8847076669540895 | 0.7835914696483758 |
| Geomean ratio | 0.852553889692823 | 0.751832965799122 |
| p90 ratio | 1.1874296074499355 | 1.111115317061353 |
| p99 ratio | 1.3391167876740364 | 1.2593949243655458 |
| C-faster rows | 8 | 5 |

No-profile UPDATE/DELETE filter:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | 1.060212072301748 | 1.102200241422575 |
| Average ratio | 1.0723272238583177 | 1.1293366342154212 |
| Geomean ratio | 1.060212072301748 | 1.102200241422575 |
| p90 ratio | 1.316175176056338 | 1.6281457733137534 |
| p99 ratio | 1.316175176056338 | 1.6281457733137534 |
| C-faster rows | 2 | 3 |

The focused update filter was noisy/mixed and not sufficient alone as a keep
signal, so this pass continued to full quick gates.

## Full Quick Gate

No-profile full quick:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | 0.3516581964554835 | 0.3432531882821518 |
| Average ratio | 0.47113616234279637 | 0.4401260353994725 |
| Geomean ratio | 0.2727000762937826 | 0.26612738048754864 |
| p90 ratio | 1.1047400320195166 | 1.0317359734438676 |
| p99 ratio | 1.4684348104855613 | 1.3017792558254113 |
| C-faster rows | 12 | 8 |
| FrankenSQLite-faster rows | 79 | 81 |

Rows above `1.05x` after the candidate:

| Ratio | Section | Scenario | FSQLite ms | C SQLite ms |
| ---: | --- | --- | ---: | ---: |
| 1.3017792558254113 | UPDATE/DELETEThroughput | 100 rows / delete 5 rows | 0.116258 | 0.089307 |
| 1.1501454995484497 | INSERTThroughput - Single Transaction - medium_6col | 1000 rows | 0.687718 | 0.597940 |
| 1.094302789321772 | INSERTThroughput - Single Transaction - small_3col | 100 rows | 0.091371 | 0.083497 |
| 1.0857198167387236 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | 0.084127 | 0.077485 |
| 1.073267098853044 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / batched (100/txn) | 0.083937 | 0.078207 |
| 1.0670745609035346 | UPDATE/DELETEThroughput | 1000 rows / update 100 rows | 0.486761 | 0.456164 |
| 1.066496469743508 | UPDATE/DELETEThroughput | 100 rows / update 10 rows | 0.133680 | 0.125345 |
| 1.0539440836524196 | INSERTThroughput - Single Transaction - medium_6col | 100 rows | 0.118633 | 0.112561 |

This first full quick run looked like a keep signal. Because CrimsonGorge's
focused UPDATE/DELETE artifact for the same shape showed a rejection, this pass
reran full quick in reverse order with the same binaries.

## Full Quick Repeat

Reverse-order no-profile full quick:

| Metric | Baseline repeat | Candidate repeat |
| --- | ---: | ---: |
| Weighted score | 0.344815755555221 | 0.3498281962295187 |
| Average ratio | 0.4497896322365449 | 0.4546914761817618 |
| Geomean ratio | 0.2653974056811448 | 0.2667304935254383 |
| p90 ratio | 1.0229615071185465 | 1.0513673719630703 |
| p99 ratio | 1.448494024100538 | 1.4059811268387454 |
| C-faster rows | 9 | 10 |
| FrankenSQLite-faster rows | 81 | 79 |

Rows above `1.05x` after the candidate repeat:

| Ratio | Section | Scenario | FSQLite ms | C SQLite ms |
| ---: | --- | --- | ---: | ---: |
| 1.4059811268387454 | UPDATE/DELETEThroughput | 100 rows / update 10 rows | 0.121578 | 0.086472 |
| 1.4041208723660594 | UPDATE/DELETEThroughput | 100 rows / delete 5 rows | 0.114214 | 0.081342 |
| 1.264814899933246 | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 2 writers x 1000 rows | 14.886989 | 11.770093 |
| 1.231920756684293 | INSERTThroughput - Single Transaction - tiny_1col | 100 rows | 0.082705 | 0.067135 |
| 1.1802015093019815 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / batched (100/txn) | 0.087735 | 0.074339 |
| 1.0985093885670507 | INSERTThroughput - Single Transaction - large_10col | 100 rows | 0.163456 | 0.148798 |
| 1.0921316660243718 | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | 0.082184 | 0.075251 |
| 1.0759020569073432 | INSERTThroughput - Single Transaction - small_3col | 100 rows | 0.084057 | 0.078127 |
| 1.0721192194704383 | INSERTThroughput - Single Transaction - medium_6col | 100 rows | 0.110216 | 0.102802 |
| 1.0513673719630703 | INSERTThroughput - Record Size Comparison (10K rows, single txn) | large_10col - 10 cols | 9.948712 | 9.462641 |

## Correctness

Candidate checks:

- `cargo fmt -p fsqlite-core --check` passed.
- `cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture --test-threads=1`
  passed: 28 tests.
- `cargo test -p fsqlite-core direct_simple_update -- --nocapture --test-threads=1`
  passed: 5 tests.
- `cargo test -p fsqlite-core direct_simple_delete -- --nocapture`
  passed: 1 test.

Two earlier parallel test invocations produced false failures:

- The first `prepared_direct_simple_insert` run failed two tests because
  `TMPDIR=/data/tmp/frankensqlite-root-predecode-tmp` did not exist yet.
- The first broad `direct_simple_update` run failed a hot-path profile assertion
  while other filtered test binaries were running against the same target.

Both affected tests passed when rerun serially after creating the temp
directory.

## Decision

Rejected and not a keep. The first full quick run improved, but the
reverse-order full quick repeat worsened the primary weighted score,
average/geomean ratios, p90, and C-faster row count. Combined with
CrimsonGorge's focused UPDATE/DELETE rejection, prepare-time root `PageNumber`
caching is too small/noisy to stand as an isolated optimization.

Do not retry this shape as a standalone prepared direct DML optimization.
