# Current HEAD INSERT Refresh - 2026-05-11

Purpose: refresh the focused `INSERTThroughput` profile on current `HEAD` after
the kept pager free-page dispatch patch and the rejected inline-dispatch retry.

Build and run:

- Git: `main @ fdba4a6f4dbe162f9f82698a16e77cbdd60bf924`
- Build: `release-perf`, opt-level 3, LTO
- Command: `FSQLITE_BENCH_PROFILE_INSERT=1 comprehensive-bench --quick --filter insert --no-html`
- Output: `insert.json`, `stdout.txt`, and `stderr.txt` in this directory

## Result

- Scenarios: 25
- FSQLite faster / comparable / C SQLite faster: 17 / 0 / 8
- Average ratio: `0.8488023932937518`
- Geomean ratio: `0.8257312152979909`
- Median ratio: `0.7822753799494335`
- P90 ratio: `1.1369067408487974`
- P99 ratio: `1.148266304063595`
- Observed weighted score: `0.8209692957916329`

## Red Rows

| Section | Scenario | Ratio F/C | FSQLite | C SQLite |
| --- | --- | ---: | ---: | ---: |
| Single Transaction - small_3col | 100 rows | `1.15x` | `87.8 us` | `76.5 us` |
| Transaction Strategy Comparison - small_3col | 100 rows / single txn | `1.14x` | `84.7 us` | `74.0 us` |
| Single Transaction - large_10col | 100 rows | `1.14x` | `167.6 us` | `147.4 us` |
| Transaction Strategy Comparison - small_3col | 100 rows / batched | `1.13x` | `86.1 us` | `76.5 us` |
| Single Transaction - medium_6col | 100 rows | `1.09x` | `110.6 us` | `101.1 us` |
| Single Transaction - tiny_1col | 100 rows | `1.08x` | `74.7 us` | `69.0 us` |
| Single Transaction - large_10col | 10000 rows | `1.07x` | `9.91 ms` | `9.25 ms` |
| Record Size Comparison | large_10col - 10 cols (~600B) | `1.06x` | `9.89 ms` | `9.30 ms` |

## Profile Attribution

The large 10-column 10K record-size row stayed on the prepared direct INSERT
path (`direct_insert=10000`, `slow=0`) with one owned empty-root page run
(`page_run_flushes=1`, `page_run_owned=1`, `page_run_empty_root=1`):

- `row_build_ns=5540370`
- `preserialize_ns=4954413`
- `preserialize_cell_ns=3138843`
- `preserialize_encode_ns=876868`
- `direct_flush_ns=2712187`
- `btree_bulk_leaf_build=2000/874779`
- `btree_bulk_leaf_write=2000/611815`
- `pager_mem_flush_ns=897871`
- `pager_cache_finish_ns=893432`

## Decision

No source patch from this artifact. Current INSERT is mostly green and the
remaining large-row gap is now near the noise band, while the obvious
record/page-run families are already rejected in the negative-results ledger.
The next source work should stay on the broader transaction-local DML mutation
operator for the remaining `UPDATE/DELETEThroughput` DELETE gaps.
