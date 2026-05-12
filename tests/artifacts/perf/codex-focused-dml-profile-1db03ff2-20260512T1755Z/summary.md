# Focused DML Profile After Profile-Env Commit

- Date: 2026-05-12 17:55 UTC
- Commit: `1db03ff260e81604e9fd9564473e52f42a119e13`
- Command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-next-target FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 10000 20 delete compare standard`
- Evidence:
  `delete-standard.stderr.txt` and `delete-standard.stdout.txt`.

## Summary

This run used the lightweight `perf-update-delete` profiler introduced in
`1db03ff2` to re-check the current 10k-row, 500-row standard DELETE gap without
the full comprehensive-bench ceremony.

| Engine | Total | Populate | DELETE | Per Deleted Row |
| --- | ---: | ---: | ---: | ---: |
| FrankenSQLite | `73 ms` | `53 ms` | `6 ms` | `667 ns` |
| C SQLite | `70 ms` | `65 ms` | `3 ms` | `358 ns` |

DELETE ratio: `1.86x` F/C. Total ratio: `1.03x` F/C.

## Profile Shape

All measured DELETE statements stayed on the prepared direct fast path:

| Counter | Representative Value |
| --- | ---: |
| Direct DELETE rows | `500` per iteration |
| Fast / slow | `500 / 0` per iteration |
| Same-leaf run starts | `64 / 67` |
| Active same-leaf hits | `433 / 496` |
| Active misses | `63` |
| Out-of-leaf misses | `60` |
| Non-root-last-cell misses | `3` |
| Dirty leaf-run flushes | `64 / 64` |
| Page pool hits / misses | `65 / 1` |

Warm iterations were concentrated in the same four buckets already seen in the
full DML refresh:

| Bucket | Warm-Iteration Range |
| --- | ---: |
| `delete_seek_ns` | about `32-49 us` |
| `delete_leaf_active_ns` | about `49-51 us` |
| `delete_leaf_materialize` | about `49-66 us` |
| `commit_roundtrip_ns` | about `20-22 us` |

The first iteration was colder (`545.6 us`) and reported larger seek and
materialization costs, but the steady-state runs still retained the same
physical same-leaf delete-run profile.

## Decision

No source patch was attempted from this profile. The run confirms that the
remaining standard DELETE tail is not a missed direct-dispatch path: `slow=0`,
parser work is absent, background checks remain small, and the direct DELETE
path is already using retained same-leaf runs.

The negative-results ledger already fences the plausible standalone variants:
compactness prechecks, monotone search floors, direct-writer/borrowed-write
publication, freeblock-chain materialization, single-pass threshold tuning,
live-span materialization, transaction-envelope trimming, and prepared-lookup
or background-wrapper trimming.

The next credible source lever remains the broader transaction-local DML
mutation/read-view operator: logical rowid deltas need to become visible to
B-tree reads inside the same transaction before commit-side physical page
publication can be removed from the benchmark-critical path.
