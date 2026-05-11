# Current INSERT Profile Frontier

This artifact captures a profiled quick `INSERTThroughput` run with
`FSQLITE_BENCH_PROFILE_INSERT=1` and `--quick --filter insert`.

## Scope Caveat

The benchmark stdout reports `Git: main @
6e9a5b95c14d8db1e93738c19d399cdfcb35c20c`, `Git dirty: yes`, and warns that
the benchmark binary predates Git HEAD. The later commits on the current branch
are docs/test-only relative to the retained perf source path, so this is useful
as a source-frontier profile, but it is not a fresh full gate for current HEAD.

## Result

- Scenarios: 25
- FSQLite faster / comparable / C SQLite faster: 16 / 1 / 8
- Average ratio: 0.8919133739782801
- Geomean ratio: 0.8667666365038209
- Median ratio: 0.8137349629711796
- P90 ratio: 1.1648027083819752
- P99 ratio: 1.2913387913895782
- Observed weighted score: 0.863967906049364

## Red Rows

| Section | Scenario | Ratio F/C | FSQLite ms | C SQLite ms |
| --- | --- | ---: | ---: | ---: |
| Single Transaction - large_10col | 10000 rows | 1.2913387913895782 | 12.471772000000001 | 9.658017 |
| Transaction Strategy Comparison - small_3col | 100 rows / single txn | 1.168898748390984 | 0.08535999999999999 | 0.07302600000000001 |
| Single Transaction - small_3col | 100 rows | 1.1648027083819752 | 0.099777 | 0.08566 |
| Single Transaction - large_10col | 100 rows | 1.1642095072908054 | 0.173175 | 0.148749 |
| Transaction Strategy Comparison - small_3col | 100 rows / batched (100/txn) | 1.1599399261253704 | 0.08573 | 0.073909 |
| Single Transaction - medium_6col | 100 rows | 1.1317334872795317 | 0.11370300000000001 | 0.100468 |
| Single Transaction - tiny_1col | 100 rows | 1.1199897302685233 | 0.074159 | 0.066214 |
| Record Size Comparison | large_10col - 10 cols (~600B: includes long text fields) | 1.0721716391782885 | 10.696931 | 9.976883 |

## Profile Attribution

The large 10-column 10K single-transaction profile stayed on the prepared direct
INSERT path (`direct_insert=10000`, `slow=0`) with one owned empty-root page run
(`page_run_flushes=1`, `page_run_owned=1`, `page_run_empty_root=1`). The visible
FSQLite time is split across row construction and page publication:

- `row_build_ns=5472592`
- `preserialize_ns=4891849`
- `preserialize_cell_ns=3166275`
- `preserialize_encode_ns=791104`
- `direct_flush_ns=3304715`
- `btree_bulk_leaf_build=2000/1172428`
- `btree_bulk_leaf_write=2000/697188`
- `pager_mem_flush_ns=1094398`
- `pager_cache_finish_ns=1118514`

This supports the existing negative-ledger boundary: standalone concat,
record-template, page-run threshold/arena, borrowed owned-record flush, eager
restore-clone, and empty-root layout-cache tweaks are still the wrong next
move. The next credible source shape remains a fused record-body/page-layout
builder that removes duplicate large-row construction and page publication work
and then proves the focused INSERT slice plus full quick primary score in the
same measurement window.
