# Focused INSERT Profile Log

Date: 2026-05-08
Command:
`env FSQLITE_BENCH_PROFILE_INSERT=1 .rch-target/release-perf/comprehensive-bench --quick --filter insert --no-html --json-out tests/artifacts/perf/windyibis-insert-profile-log-20260508T0715Z/insert-profile.json`

This run captured the stderr `insert_profile` lines that were missing from the earlier focused INSERT JSON bundle. It used the current dirty integration binary from the pagebuf256 + commit-timing experiment.

## Summary

| Metric | Value |
| --- | ---: |
| Focused INSERT weighted score | 0.7367848148 |
| Average ratio | 0.7872805081 |
| Geomean ratio | 0.7514995346 |
| P90 ratio | 1.0588928456 |
| P99 ratio | 1.7150147712 |
| Faster / comparable / slower | 18 / 4 / 3 |

The 100-row INSERT rows are noisy across same-source quick runs. In this rerun, `tiny_1col / 100 rows` moved from slower in the full matrix to faster (`0.7689x`), while `large_10col / 100 rows` became the worst focused row (`1.7150x`). Do not choose a source lever from a single 100-row quick sample without same-window A/B confirmation.

## Profile Signals

Selected `insert_profile` rows:

| Row | setup_us | begin_us | prepare_us | insert_us | commit_us | row_build_ns | btree_insert_ns | page_pool_misses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| tiny_1col 100 | 20.8 | 10.1 | 8.3 | 48.0 | 10.7 | 3,585 | 3,207 | 1 |
| small_3col 100 | 15.6 | 9.3 | 9.2 | 55.7 | 10.6 | 11,329 | 3,266 | 1 |
| medium_6col 100 | 17.5 | 9.2 | 11.6 | 65.0 | 19.7 | 19,737 | 3,576 | 7 |
| large_10col 100 | 21.0 | 10.1 | 17.9 | 85.2 | 40.5 | 38,337 | 5,639 | 22 |
| large_10col 10000 | 49.3 | 16.0 | 34.5 | 8,630.5 | 4,603.9 | 3,802,281 | 793,742 | 2006 |

Row-building remains the largest in-row CPU counter for concat-heavy large INSERT records, while B-tree insertion is smaller. However, the negative-results ledger already rejects the obvious standalone source families for this surface:

- prepared direct INSERT row-template executor
- direct INSERT header-size and fixed cell-array micro paths
- `?1 op literal` and param-one concat/text-cache specializations
- direct-record layout reuse / row-value pooling / owned-text moving
- standalone page-run and bulk page-builder variants that failed full or focused gates

## Decision

Evidence-only profile artifact. The next source attempt on direct INSERT row-building should be broader than another local expression specialization and must win a same-window focused INSERT gate plus a full quick weighted gate. The safer near-term path is to wait for the pagebuf256/timing source owner to land cleanly, then re-run the full quick matrix from a clean tree before choosing another source lever.
