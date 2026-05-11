# Post Free-Page Dispatch DML Profile

Date: 2026-05-11

Commit under test: `fc84f8a3` (`perf(pager): statically dispatch free page`)

Purpose: refresh the focused `UPDATE/DELETEThroughput` profile after the
`TransactionKind::free_page` static-dispatch patch, then decide whether the
remaining DELETE tail exposes a fresh standalone source lever.

## Build

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-post-free-page-perf \
  CARGO_BUILD_JOBS=12 \
  cargo build --profile release-perf -p fsqlite-e2e \
  --bin perf-update-delete --bin comprehensive-bench
```

The rebuilt release-perf binaries postdate `fc84f8a3`; this is the authoritative
post-patch profile artifact. The earlier sibling artifact
`codex-post-free-page-dml-profile-20260511T112754Z` was produced before the
binary rebuild and reported that the benchmark binary predated `HEAD`.

## Focused Quick Slice

Command:

```bash
env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-post-free-page-perf/release-perf/comprehensive-bench \
  --quick --filter update --no-html \
  --json-out tests/artifacts/perf/codex-post-free-page-dml-profile-20260511T113225Z/update-delete-profile-quick.json
```

Rows:

| Scenario | C SQLite median | FSQLite median | Ratio |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.005119 ms | 0.006703 ms | 1.3094x |
| 100 rows / delete 5 rows | 0.002254 ms | 0.008225 ms | 3.6491x |
| 1000 rows / update 100 rows | 0.035676 ms | 0.031899 ms | 0.8941x |
| 1000 rows / delete 50 rows | 0.015188 ms | 0.032130 ms | 2.1155x |
| 10000 rows / update 1000 rows | 0.348232 ms | 0.274914 ms | 0.7895x |
| 10000 rows / delete 500 rows | 0.153979 ms | 0.293349 ms | 1.9051x |

Section summary: 2 FSQLite-faster rows, 4 C-SQLite-faster rows, geomean ratio
1.5449, median ratio 1.9051, p99 ratio 3.6491.

## DELETE Profile Counters

The 500-row DELETE case still takes the prepared direct path and spends the
visible time in the retained leaf-run machinery:

- `direct_delete=500`
- `delete_seek_ns=35971`
- `delete_leaf_start=64/67`, `delete_leaf_start_ns=12327`
- `delete_leaf_active=433/496`, `delete_leaf_active_ns=52240`
- `delete_leaf_miss=63` (`60` rowid-not-in-leaf, `3` nonroot-last-cell)
- `delete_leaf_flush=64/64`, `delete_leaf_flush_ns=113993`
- `delete_leaf_materialize=64/83642`
- `delete_leaf_write=64/23157`
- `commit_us=44.7`

## Sampled Perf

Heavy isolated DELETE command:

```bash
perf record -F 997 --call-graph dwarf \
  -o tests/artifacts/perf/codex-post-free-page-dml-profile-20260511T113225Z/perf-delete-isolated-heavy.data \
  -- /data/tmp/frankensqlite-post-free-page-perf/release-perf/perf-update-delete \
  10000 10000 delete fsqlite isolated
```

Run output: total 6565 ms, populate 1720 ms, delete 4288 ms, 858 ns per deleted
row. The capture wrote 6797 samples. Kernel symbols were restricted by the host,
but user-space symbols resolved.

Top no-children self-time:

| Symbol | Self |
| --- | ---: |
| `TransactionKind::get_page` | 25.63% |
| `TransactionKind::write_page_data` | 5.57% |
| `Connection::try_serialize_prepared_direct_simple_insert_record` | 4.55% |
| `_int_malloc` | 4.10% |
| `__memmove_avx_unaligned_erms` | 3.59% |
| `TableLeafDeleteRun::delete_rowid_with_reason` | 3.10% |
| `TransactionKind::free_page` | 2.89% |
| `serialize_freelist_to_write_set` | 2.86% |
| `return_pages_to_freelist` | 2.81% |
| `SimpleTransaction::durable_freelist_pages_with_inner` | 2.77% |

## Decision

No source patch was attempted from this profile. The remaining visible levers
are the same fenced families already recorded in
`docs/progress/perf-negative-results.md`: standalone `get_page`/freed-page
lookup changes, direct leaf-run flush wrappers, retained-cursor shells,
next-cell hints, and `TableLeafDeleteRun` materializer changes.

The next credible source direction remains the broader `bd-db300.11.1`
transaction-local DML mutation operator. A keepable candidate needs to improve
the focused 5-row, 50-row, and 500-row DELETE rows while keeping the full quick
matrix primary score neutral or better in the same A/B window.
