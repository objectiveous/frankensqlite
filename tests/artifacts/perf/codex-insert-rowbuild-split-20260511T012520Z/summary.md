# Direct INSERT Row-Build Split Profile

- Run date: 2026-05-11T01:25:20Z
- Base commit: `cee9bbbecd321e163c728b4ecfdfc6f1b0e5c3c6`
- Candidate: profiling-only working tree that splits prepared direct INSERT row-build counters.
- Command:
  `FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-insert-rowbuild-profile-bench-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/codex-insert-rowbuild-split-20260511T012520Z/insert-profile.json --no-html`

## Section Result

- Scenarios: 25
- FSQLite faster / comparable / C SQLite faster: 17 / 2 / 6
- Average ratio: 0.842541798676873
- Geomean ratio: 0.8149083849321554
- Median ratio: 0.7639278680008946
- p90 ratio: 1.1371324574103894
- p99 ratio: 1.2978791516606643
- Weighted score: 0.8262273960738257

## C-Side Faster Rows

| Section | Scenario | Ratio | FSQLite ms | C SQLite ms |
|---|---:|---:|---:|---:|
| Single Transaction tiny_1col | 100 rows | 1.2978791516606643 | 0.097302 | 0.074970 |
| Single Transaction small_3col | 100 rows | 1.106674420107256 | 0.085640 | 0.077385 |
| Single Transaction medium_6col | 100 rows | 1.0607748507334567 | 0.111218 | 0.104846 |
| Single Transaction large_10col | 100 rows | 1.1371324574103894 | 0.168075 | 0.147806 |
| Single Transaction large_10col | 10000 rows | 1.0431958901891945 | 9.717449 | 9.315076 |
| Transaction Strategy small_3col | 100 rows / batched | 1.1586203301929818 | 0.088916 | 0.076743 |
| Transaction Strategy small_3col | 100 rows / single txn | 1.1092243077880328 | 0.085570 | 0.077144 |
| Record Size large_10col | 10000 rows | 1.0472266272946282 | 9.881066 | 9.435461 |

## Row-Build Split

All 10K non-tiny INSERT profiles use the direct preserialized-record path; `row_value_build_ns=0` throughout. The dominant subphase is cell/value/layout construction, not byte encoding.

| Profile Row | row_build_ns | preserialize_ns | preserialize_cell_ns | preserialize_encode_ns | row_value_build_ns |
|---|---:|---:|---:|---:|---:|
| `fs_insert_single_txn_small_3col_10000` | 2749552 | 2166050 | 882714 | 311239 | 0 |
| `fs_insert_single_txn_medium_6col_10000` | 3570116 | 2989851 | 1496225 | 553566 | 0 |
| `fs_insert_single_txn_large_10col_10000` | 5553059 | 4968148 | 3209521 | 823277 | 0 |
| `fs_insert_record_size_small_3col_10000` | 2707463 | 2111791 | 853019 | 321643 | 0 |
| `fs_insert_record_size_medium_6col_10000` | 3489821 | 2909265 | 1436683 | 536538 | 0 |
| `fs_insert_record_size_large_10col_10000` | 5549525 | 4965089 | 3226389 | 802153 | 0 |

## Interpretation

The rejected page-run families should stay rejected as standalone patches. This split points the next viable INSERT design at reducing direct-record cell/value/layout work over a run, not at another owned/arena/page-run publication tweak. Byte encoding is a smaller target than expression evaluation, affinity, serial-type layout, and scratch-text assembly.
