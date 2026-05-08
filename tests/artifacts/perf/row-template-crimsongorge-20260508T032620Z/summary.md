# Prepared Direct INSERT Row-Template Probe

- Date: 2026-05-08
- Agent: CrimsonGorge
- Base commit: `1c5bdcca`
- Candidate source diff: `candidate-connection.diff`
- Benchmark isolation: clean `HEAD` archive under
  `/data/tmp/frankensqlite-row-template-candidate-20260508T032620Z` with only
  the candidate `connection.rs` patch applied. The shared checkout was not used
  for timing after an unrelated peer edit appeared.

## Candidate

Prepared direct INSERT built a per-column record template at prepare time, then
used that template to emit SQLite record bytes directly for literal,
placeholder, numeric binary, and concat expressions. Unsupported expression
shapes fell back to the existing compiled-row serializer.

This was a query-compilation style specialization probe: move expression-shape
branching out of the row loop and keep runtime work to parameter lookup,
affinity, rowid coercion, and byte emission.

## Correctness And Build Proof

- `cargo fmt -p fsqlite-core --check` passed before benchmarking.
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-row-template-check-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture`
  completed successfully with 28 tests passing. RCH artifact retrieval hung
  after the remote command had completed, so the local RCH process group was
  terminated.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-row-template-candidate-target CARGO_BUILD_JOBS=12 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf`
  passed in the clean scratch tree.

## Focused INSERT Result

Same-window `comprehensive-bench --quick --filter insert` improved:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | 0.8030801931161379 | 0.7915138891132704 |
| Average ratio | 0.8290893302098494 | 0.7957192322438452 |
| Geomean ratio | 0.8009900880092378 | 0.7729268013969751 |
| p90 ratio | 1.144713351857072 | 1.1195686003870147 |
| p99 ratio | 1.2884507148302573 | 1.1320620924604214 |
| C-faster rows | 7 | 5 |
| FrankenSQLite-faster rows | 17 | 19 |

Artifacts:

- `baseline-insert.json`
- `candidate-insert.json`
- `baseline-insert.stdout`
- `candidate-insert.stdout`

## Full Quick Gate

The candidate failed the full quick keep gate against the latest clean baseline:

| Metric | Clean baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | 0.34593878641661835 | 0.35679620885171676 |
| Average ratio | 0.4542606463918878 | 0.4850687497684193 |
| Geomean ratio | 0.2674752493298549 | 0.2795497259901094 |
| p90 ratio | 0.9811588214938469 | 1.0870772854107467 |
| p99 ratio | 1.4015153360781543 | 2.091131458001714 |
| C-faster rows | 8 | 11 |
| FrankenSQLite-faster rows | 80 | 80 |

Rows above 1.05x in the candidate full quick run:

- `INSERTThroughput - Single Transaction - small_3col`, 100 rows: 1.0972375975079165
- `INSERTThroughput - Single Transaction - medium_6col`, 100 rows: 1.083375604807242
- `INSERTThroughput - Single Transaction - medium_6col`, 1000 rows: 1.2384622689561697
- `INSERTThroughput - Single Transaction - large_10col`, 100 rows: 1.2688396104027049
- `INSERTThroughput - Single Transaction - large_10col`, 10000 rows: 1.982737271313276
- `INSERTThroughput - Transaction Strategy Comparison (small_3col)`, 100 rows / batched: 1.0870772854107467
- `INSERTThroughput - Transaction Strategy Comparison (small_3col)`, 100 rows / single txn: 1.1096413527026494
- `INSERTThroughput - Record Size Comparison (10K rows, single txn)`, large_10col: 2.091131458001714
- `Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC`, 2 writers x 1000 rows: 1.1753549711944895
- `UPDATE/DELETEThroughput`, 100 rows / update 10 rows: 1.348691160934382
- `UPDATE/DELETEThroughput`, 100 rows / delete 5 rows: 1.3058091386783544

## Decision

Rejected and reverted from source. Do not retry the row-template executor as a
standalone direct INSERT optimization. It is only worth revisiting if paired
with a larger row/page builder design that protects the large-row full quick
rows and wins the full quick weighted score in the same A/B window.
