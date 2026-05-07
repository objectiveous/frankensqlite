# Transaction Strategy Profile After Leaf-Hint Candidate

- Agent: CrimsonGorge
- Date: 2026-05-07
- Source commit measured: `6e13684fd6a95ae9ab55613e5942dbc76b684348`
- Benchmark binary: `/data/tmp/frankensqlite-current-head-leafhint-target/release-perf/comprehensive-bench`
- Worktree: `/data/projects/frankensqlite-clean-head-crimsongorge-20260507T1320Z`
- Command: `FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-current-head-leafhint-target/release-perf/comprehensive-bench --quick --filter transaction --json-out /data/projects/frankensqlite/tests/artifacts/perf/transaction-profile-after-leafhint-crimsongorge-20260507T1345Z/report-transaction.json --no-html`

## Scorecard

- Scenarios: 9
- FrankenSQLite faster: 5
- Comparable: 1
- C SQLite faster: 3
- Primary weighted score: `0.8794821830844206`
- Geomean ratio: `0.9819269749223639`
- Median ratio: `0.9187023715218605`
- P90/P99 ratio: `1.4550766496548666`

## Rows

| Scenario | C SQLite ms | FSQLite ms | F/C ratio |
|---|---:|---:|---:|
| 100 rows / autocommit | 0.154349 | 0.116939 | 0.757627 |
| 100 rows / batched (100/txn) | 0.077014 | 0.089267 | 1.159101 |
| 100 rows / single txn | 0.075150 | 0.087795 | 1.168263 |
| 1000 rows / autocommit | 0.848610 | 0.733133 | 0.863922 |
| 1000 rows / batched (1000/txn) | 0.342101 | 0.314289 | 0.918702 |
| 1000 rows / single txn | 0.339356 | 0.328626 | 0.968381 |
| 10000 rows / autocommit | 8.218002 | 7.004600 | 0.852348 |
| 10000 rows / batched (1000/txn) | 3.107059 | 4.521009 | 1.455077 |
| 10000 rows / single txn | 3.093885 | 2.684729 | 0.867753 |

## 10K Insert Profile Counters

| Strategy | insert_us | commit_us | row_build_ns | cursor_setup_ns | btree_insert_ns | leaf_payload_appends | quick_balance_hits | conservative_reloads |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| autocommit | 13802.0 | 0.0 | 1819076 | 472364 | 1589301 | 9898 | 62 | 63 |
| batched 1000/txn | 8472.3 | 156.1 | 1573074 | 395141 | 1504173 | 8934 | 57 | 57 |
| single txn | 6138.7 | 330.5 | 1428710 | 1052 | 693816 | 0 | 0 | 0 |

## Diagnosis

The worst remaining transaction-strategy row is not explained by commit time:
the 10-batch profile records only `156.1 us` total commit wall time. The gap is
inside the insert loop. Compared with the 10K single transaction path, the 10K
batched path pays roughly `+394 us` cursor setup and `+810 us` B-tree insert
time, with 57 quick-balance/right-edge reload cycles.

That pattern points at a boundary effect. The single transaction can stay on the
empty-table page-run/bulk path, while the batched workload flushes the first
1K-row batch and then the remaining nine batches append into a non-empty B-tree
row-by-row. Autocommit is also faster than C here, so the lever should not be a
generic direct-INSERT row builder reshuffle. The next viable step-change target
is a true non-empty right-edge page-run/bulk append builder, or a transaction
frontier cache that lets consecutive explicit batches resume bulk append layout
after COMMIT without replaying rows one-by-one.

Guardrail: the existing negative ledger already rejects row-at-a-time non-empty
page-run replay. This artifact argues for a different shape: build or splice
whole right-edge pages, preserving correctness at each explicit COMMIT boundary.
