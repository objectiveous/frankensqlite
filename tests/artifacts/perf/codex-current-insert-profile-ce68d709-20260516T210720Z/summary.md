# Current INSERT Profile Boundary

- Date: 2026-05-16 21:14:26 UTC.
- Source: `main @ ce68d7097d3da6f79bf66b5b8bf1e6cda2251757`.
- Command:
  `rch exec -- env FSQLITE_BENCH_PROFILE_INSERT=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert --json-out tests/artifacts/perf/codex-current-insert-profile-ce68d709-20260516T210720Z/insert.json --no-html`
- Raw evidence: `run.log`.
- Note: RCH reported `insert.json` on the worker, but only `run.log` was retained locally.

## Matrix Summary

- Total scenarios: 25.
- FrankenSQLite faster / comparable / C SQLite faster: 15 / 1 / 9.
- Average F/C time ratio: 0.94x.

Remaining red rows:

| Section | Scenario | C SQLite | FrankenSQLite | Ratio |
| --- | --- | ---: | ---: | ---: |
| Single transaction tiny_1col | 100 rows | 48.3 us | 55.9 us | 1.16x slower |
| Single transaction small_3col | 100 rows | 50.9 us | 76.1 us | 1.49x slower |
| Single transaction medium_6col | 100 rows | 75.7 us | 93.6 us | 1.24x slower |
| Single transaction large_10col | 100 rows | 150.2 us | 175.2 us | 1.17x slower |
| Single transaction large_10col | 10000 rows | 10.33 ms | 15.01 ms | 1.45x slower |
| Transaction strategy small_3col | 100 rows / autocommit | 113.4 us | 129.7 us | 1.14x slower |
| Transaction strategy small_3col | 100 rows / batched | 70.7 us | 82.3 us | 1.16x slower |
| Transaction strategy small_3col | 100 rows / single txn | 69.5 us | 76.2 us | 1.10x slower |
| Record size comparison | large_10col 10000 rows | 10.59 ms | 13.75 ms | 1.30x slower |

Representative hot counters:

| Scenario | Insert | Commit | Key profile counters |
| --- | ---: | ---: | --- |
| `fs_insert_record_size_large_10col_10000` | 12.2228 ms | 6.7113 ms | `row_build_ns=6190021`, `preserialize_ns=5523982`, `preserialize_cell_ns=3510571`, `direct_flush_ns=2901657`, `commit_roundtrip_ns=3480600`, `pager_mem_flush_ns=1436832`, `pager_cache_finish_ns=1996177`, `page_pool_misses=2004`, `btree_bulk_leaf_build=2000/842270` |
| `fs_insert_single_txn_large_10col_10000` | 14.4643 ms | 8.7619 ms | `row_build_ns=7078210`, `preserialize_ns=6333637`, `direct_flush_ns=4957353`, `commit_roundtrip_ns=3380008`, `pager_mem_flush_ns=1360498`, `pager_cache_finish_ns=1975474`, `page_pool_misses=2004` |
| `fs_insert_record_size_medium_6col_10000` | 9.8412 ms | 1.4447 ms | Green row despite `row_build_ns=4270506`, `preserialize_ns=3585652`, `direct_flush_ns=813415`, `page_pool_misses=455` |

## Decision

No source patch was attempted from this profile. The 100-row red rows are fixed-cost tails with high variance in several cells, and the large-row red rows are the already-known row/body/page construction frontier. The negative-results ledger has many rejected standalone attempts around direct INSERT serialization, row scratch/template tweaks, owned-record flushes, page-image construction, capacity tuning, and direct page-run mechanics.

The next INSERT attempt should be a broader fused row/body/page construction design that proves both this focused INSERT profile and the full quick matrix. Standalone helper tweaks are not justified by this artifact.
