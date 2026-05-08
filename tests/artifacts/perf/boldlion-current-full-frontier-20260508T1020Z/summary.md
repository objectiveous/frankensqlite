# Current Full Frontier Snapshot

Date: 2026-05-08 10:20Z
Commit: `9f5d4ac92a7e26d5acbc1e7e739ab29b64dae991`
Binary: `/data/tmp/frankensqlite-boldlion-delete-tail-target/release-perf/comprehensive-bench`

The benchmark binary was built before the artifact-only commit above. The source tree did not change between the build and this snapshot, but the benchmark reports correctly warn that the binary predates Git HEAD.

## Runs

- `current-full-quick.json`: full quick matrix, `--quick --no-html`
- `current-insert-repeat.json`: focused INSERT repeat, `--quick --filter INSERT --no-html`
- `current-insert-profile.json`: focused INSERT repeat with `FSQLITE_BENCH_PROFILE_INSERT=1`
- `current-concurrent-repeat.json`: focused concurrent-writer repeat, `--quick --filter Concurrent --no-html`

Stdout and stderr captures are under `stdout/`.

## Full Quick Result

- Scenarios: 93
- Faster / comparable / slower: 83 / 3 / 7
- Average ratio: 0.4618x
- Geomean ratio: 0.2739x
- Median ratio: 0.2978x
- P90 / P99 ratio: 0.9979x / 1.5258x
- Primary weighted score: 0.3554x

Rows over 1.05x in the full run:

| Section | Scenario | Ratio | C SQLite | FrankenSQLite | F CV |
| --- | --- | ---: | ---: | ---: | ---: |
| INSERT single txn small_3col | 100 rows | 1.175x | 0.076904 ms | 0.090369 ms | 5.80% |
| INSERT single txn large_10col | 100 rows | 1.127x | 0.149150 ms | 0.168074 ms | 4.05% |
| INSERT txn strategy small_3col | 100 rows / batched | 1.094x | 0.076272 ms | 0.083416 ms | 3.36% |
| INSERT txn strategy small_3col | 100 rows / single txn | 1.526x | 0.074650 ms | 0.113904 ms | 33.44% |
| Concurrent writers | 2 writers x 1000 rows | 1.171x | 11.378463 ms | 13.324446 ms | 27.06% |
| UPDATE/DELETE | 100 rows / update 10 rows | 1.472x | 0.085340 ms | 0.125625 ms | 20.40% |
| UPDATE/DELETE | 100 rows / delete 5 rows | 1.365x | 0.083346 ms | 0.113753 ms | 6.08% |

## Focused Repeats

The full-run `INSERT txn strategy small_3col / 100 rows / single txn` spike did not reproduce at the same magnitude. In the INSERT profile run, the comparable 100-row INSERT tails were 1.056x to 1.135x, with `small_3col / 100 rows / single txn` at 1.114x.

The one larger INSERT tail in the profiled repeat was `large_10col / 10000 rows` at 1.098x. The profile for that row showed direct insert remained on the fast path for every row:

- rows: 10000
- direct / fast / slow: 10000 / 10000 / 0
- insert: 8655.2 us
- commit: 3974.7 us
- page-pool misses: 2006
- row build: 3955583 ns
- btree insert: 664139 ns
- memdb apply / schema validation / change tracking: 240120 ns / 310329 ns / 237848 ns

The concurrent repeat also reduced the full-run spike:

| Scenario | Ratio | C SQLite | FrankenSQLite | F CV |
| --- | ---: | ---: | ---: | ---: |
| 2 writers x 1000 rows | 1.076x | 12.953572 ms | 13.940479 ms | 7.01% |
| 4 writers x 1000 rows | 0.978x | 20.292521 ms | 19.852558 ms | 7.82% |
| 8 writers x 1000 rows | 0.446x | 92.088845 ms | 41.086922 ms | 42.36% |

## No-Source Decision

This snapshot did not produce a keepable standalone source candidate.

The remaining DML tails are covered by the dedicated delete-tail profile in `../boldlion-delete-tail-profile-20260508T1000Z/` and by prior ledger-fenced attempts around private `SharedTxnPageIo` bypass, retained DML cursors, direct DELETE clone removal, non-max/no-rebalance delete primitives, page-one cleanup caching, staged-page overwrite probing, hard-disabled dormant QF, and lazy DML fallback.

The INSERT tails are either small fixed-cost rows with noisy C/F variance or in already-fenced families: direct INSERT schema lookup, row template executor, no-FK guard cache, fixed cell staging, direct row-values plan, append-hint active bit, param text cache, param arithmetic variant, page-run thresholds/admission, record-cell layout reuse, repeated-record page-run loading, page-run arena bands, arena-backed page-run buffers, record-template serialization, preserialize-when-dirty, and global page-buffer recycle growth.

The concurrent-writer 2-thread tail did not reproduce strongly enough to justify another narrow code change. Previous focused profiles already point at shared fixed ceremony across WAL checksum/frame preparation, serial type computation, direct-insert dispatch, page loading, and page-state lookup. The next credible attempt should be a broader prepared-frame/WAL payload pipeline or a larger direct-INSERT/`SharedTxnPageIo` ceremony reduction, not a one-symbol microprobe.

## Next Viable Targets

- Design a true batch or leaf-run DML operator and gate it on the full DML plus full quick matrix.
- Design a broader WAL/prepared-frame pipeline for concurrent inserts and gate it against 2/4/8-writer repeats plus full quick.
- Treat small 100-row INSERT tails as low priority unless a new profile shows a source-local fixed cost that is not already fenced.
