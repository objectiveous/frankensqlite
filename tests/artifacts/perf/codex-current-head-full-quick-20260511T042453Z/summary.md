# Current HEAD Full Quick Baseline

This run is a fresh rebuilt current-HEAD `--quick` benchmark matrix with no
hot-path profiling flags enabled.

## Source

- Source commit reported by benchmark:
  `b21d4675f141bcc441fbdcdb84319a445df81119`
- Build command:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-head-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
- Run command:
  `/data/tmp/frankensqlite-current-head-target/release-perf/comprehensive-bench --quick --json-out tests/artifacts/perf/codex-current-head-full-quick-20260511T042453Z/full-quick.json --no-html`

The benchmark stdout reports `Git dirty: yes` because the checkout has
pre-existing untracked `.rch-*` target directories. No source changes were
present in this run.

## Result

- Scenarios: 93
- FSQLite faster / comparable / C SQLite faster: 80 / 2 / 11
- Average ratio: 0.5103530692713387
- Geomean ratio: 0.2788517574143525
- Median ratio: 0.29467029889607793
- P90 ratio: 1.0985644359316655
- P99 ratio: 3.5544130248500427
- Primary weighted score: 0.38010348528188176

## Category Ratios

| Category | n | Geomean F/C | Median F/C | P90 F/C |
| --- | ---: | ---: | ---: | ---: |
| read_aggregate | 25 | 0.07828040504815573 | 0.12631181577953945 | 0.504013914905004 |
| mixed | 1 | 0.18428163798916297 | 0.184281637989163 | 0.184281637989163 |
| read_single | 33 | 0.21787346692891146 | 0.21884368308351176 | 0.30748672593298815 |
| concurrent_writers | 3 | 0.778662451082928 | 1.0464937671974768 | 1.048070311225398 |
| write_bulk | 22 | 0.8226314586782784 | 0.8121059932457865 | 1.1496349426705743 |
| write_single | 9 | 1.2408927630655788 | 0.871912359864167 | 3.5544130248500427 |

## Remaining Rows Above 1.05x

| Section | Scenario | Ratio F/C | FSQLite ms | C SQLite ms | Category |
| --- | --- | ---: | ---: | ---: | --- |
| UPDATE/DELETEThroughput | 100 rows / delete 5 rows | 3.5544130248500427 | 0.008296 | 0.002334 | write_single |
| UPDATE/DELETEThroughput | 1000 rows / delete 50 rows | 2.071393188854489 | 0.033453 | 0.01615 | write_single |
| UPDATE/DELETEThroughput | 10000 rows / delete 500 rows | 1.7947715222654694 | 0.29280799999999996 | 0.163145 | write_single |
| UPDATE/DELETEThroughput | 100 rows / update 10 rows | 1.6290248390064397 | 0.007083 | 0.0043479999999999994 | write_single |
| INSERTThroughput - Single Transaction - tiny_1col | 100 rows | 1.1844831511839709 | 0.083236 | 0.07027199999999999 | write_bulk |
| INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / batched (100/txn) | 1.1521044011750332 | 0.08863599999999999 | 0.076934 | write_bulk |
| INSERTThroughput - Single Transaction - small_3col | 100 rows | 1.1496349426705743 | 0.089437 | 0.077796 | write_bulk |
| INSERTThroughput - Single Transaction - large_10col | 100 rows | 1.1485437405930659 | 0.173986 | 0.15148399999999998 | write_bulk |
| INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | 1.1308265556035781 | 0.08634199999999999 | 0.076353 | write_bulk |
| INSERTThroughput - Single Transaction - medium_6col | 100 rows | 1.0985644359316655 | 0.113563 | 0.10337400000000001 | write_bulk |
| INSERTThroughput - Record Size Comparison (10K rows, single txn) | large_10col - 10 cols (~600B: includes long text fields) | 1.063369336188658 | 9.883431 | 9.294448000000001 | write_bulk |

## Next Frontier

The largest current gaps remain the explicit-transaction prepared DML rows.
Prior same-window artifacts in `docs/progress/perf-negative-results.md` reject
standalone retained DELETE leaf-run tweaks, global leaf-run disabling,
transaction-control parser/background trimming, prepared-cache last-hit changes,
and root-`Cx` reuse. The next source candidate should therefore be either the
broader transaction-local DML mutation operator described in the ledger, or a
new non-overlapping 100-row INSERT fixed-cost lever that does not repeat the
rejected page-run admission, arena, concat, or prepared-cache families.
