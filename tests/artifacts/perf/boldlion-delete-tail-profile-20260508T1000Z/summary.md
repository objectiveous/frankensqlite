# Current DML Delete Tail Profile

Date: 2026-05-08
Agent: BoldLion
Source commit: `638e93f9cf34da9872fee6e2a05a98936cc0c1dc`

## Scope

This pass refreshed the remaining `UPDATE/DELETEThroughput` frontier after the
direct REAL assignment rejection. No source candidate was attempted: the fresh
delete-side profile points at mechanisms already fenced by the negative-results
ledger as standalone changes.

Agent Mail coordination was attempted first, but `macro_start_session`,
`register_agent`, and `file_reservation_paths` all timed out or were cancelled
under database contention. The write surface was kept to this unique artifact
directory only.

## Build

```text
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-delete-tail-target CARGO_BUILD_JOBS=8 cargo build -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete --profile release-perf
```

The rebuilt binaries used for these artifacts were:

```text
/data/tmp/frankensqlite-boldlion-delete-tail-target/release-perf/comprehensive-bench
/data/tmp/frankensqlite-boldlion-delete-tail-target/release-perf/perf-update-delete
```

The benchmark stdout reports `Git: main @ 638e93f9...`, binary modified
`2026-05-08 10:06:06 UTC`, and no binary-predates-HEAD warning.

## Focused DML Profile

Command:

```text
env FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-boldlion-delete-tail-target/release-perf/comprehensive-bench --quick --filter update --no-html --json-out tests/artifacts/perf/boldlion-delete-tail-profile-20260508T1000Z/current-dml-profile.json
```

`current-dml-profile.json` summary:

| Metric | Value |
| --- | ---: |
| Scenarios | `6` |
| Average ratio | `1.0931318868` |
| Geomean ratio | `1.0748888467` |
| Primary weighted score | `1.0748888467` |
| P90/P99 ratio | `1.3900732265` |
| Faster / comparable / slower | `1 / 3 / 2` |

Rows:

| Row | Ratio | C SQLite | FrankenSQLite | F CV% |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | `1.390073` | `0.086171 ms` | `0.119784 ms` | `7.71` |
| 100 rows / delete 5 rows | `1.351892` | `0.082565 ms` | `0.111619 ms` | `5.96` |
| 1000 rows / update 100 rows | `1.009511` | `0.404597 ms` | `0.408445 ms` | `2.96` |
| 1000 rows / delete 50 rows | `0.976656` | `0.384510 ms` | `0.375534 ms` | `1.89` |
| 10000 rows / update 1000 rows | `0.988785` | `3.604418 ms` | `3.563993 ms` | `0.57` |
| 10000 rows / delete 500 rows | `0.841874` | `3.407480 ms` | `2.868670 ms` | `0.79` |

The profiled stdout confirms the direct fast path stayed active for all rows:
`fast == mutations`, `slow == 0`, `vdbe_opcodes == 0`, and
`vdbe_statements == 0`.

## Isolated Delete Sample

Command:

```text
/data/tmp/frankensqlite-boldlion-delete-tail-target/release-perf/perf-update-delete 100 20000 delete fsqlite isolated
```

Result:

```text
fsqlite: total=211ms populate=38ms update=0ms delete=171ms | per-row-delete=1720ns
```

The delete-only `perf record` run used `100` rows, `60000` iterations, and the
same rebuilt binary. The text report is `perf-delete-100-flat.txt`.

Top flat self-time symbols:

| Self | Symbol |
| ---: | --- |
| `29.55%` | `TransactionKind::get_page` |
| `11.39%` | `TransactionKind::write_page_data` |
| `5.76%` | `BtCursor<SharedTxnPageIo>::table_seek_for_insert` |
| `5.56%` | `BtCursor<SharedTxnPageIo> as BtreeCursorOps>::delete` |
| `4.44%` | `__memmove_avx_unaligned_erms` |
| `2.99%` | `_int_malloc` |
| `1.98%` | `read_cell_pointers_into` |
| `1.52%` | `Connection::execute_prepared_direct_simple_delete` |

## No Source Candidate

The obvious standalone source moves are already rejected in
`docs/progress/perf-negative-results.md`:

- private-memory direct `SharedTxnPageIo` bypass,
- retained direct UPDATE/DELETE cursor shell using `advance_to`,
- `SharedTxnPageIo` concurrent-context borrow-vs-clone cleanup,
- page-one synthetic cleanup negative cache,
- direct DELETE top-stack clone removal,
- non-max/no-rebalance table-leaf DELETE primitive,
- staged-page same-size overwrite probing,
- hard-disabled dormant QF consultation,
- lazy UPDATE/DELETE fallback compilation.

The fresh delete profile reinforces the existing decision: the next keepable
source attempt needs to be a true batch/leaf-run DML operator that amortizes
page I/O and mutation ceremony across several row changes. It should prove an
isolated UPDATE/DELETE win first, then pass repeated focused DML gates and a
same-window full quick matrix. Another local cursor/page-I/O microprobe is not
justified by this profile.

## Artifacts

- `current-dml-profile.json`
- `stdout/current-dml-profile.stdout`
- `stdout/current-dml-profile.stderr`
- `stdout/isolated-delete-100.stdout`
- `stdout/isolated-delete-100.stderr`
- `stdout/perf-delete-100.stdout`
- `stdout/perf-delete-100.stderr`
- `perf-delete-100-flat.txt`
- `perf-delete-100-flat.stderr`
