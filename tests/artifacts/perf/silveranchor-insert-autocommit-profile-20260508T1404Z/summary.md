# SilverAnchor Insert Autocommit Profile - 2026-05-08

## Scope

Focused transaction-strategy profile captured while triaging the remaining
write-side gaps after `953959cbb2b495700c0737d155e6f7c84ce20acc`
(`fix(e2e): stagger concurrent benchmark retries`).

Command:

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-silveranchor-concurrent-retry-bench-target/release-perf/comprehensive-bench \
  --quick --filter transaction \
  --json-out tests/artifacts/perf/silveranchor-insert-autocommit-profile-20260508T1404Z/transaction-profile.json \
  --no-html
```

Note: the JSON environment marks the benchmark binary as older than Git `HEAD`.
The source-relevant code was the same as `953959cb`; a later local artifact-only
commit moved `HEAD` after this run. A fresh rebuild was started with:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-current-target \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

The remote compile finished successfully. RCH then stalled during target
artifact retrieval, so the local helper was stopped after the fresh binary had
arrived at `/data/tmp/frankensqlite-silveranchor-current-target/release-perf/comprehensive-bench`.

## Same-Window Transaction Results

| Scenario | C SQLite median ms | FrankenSQLite median ms | F/C ratio |
|---|---:|---:|---:|
| 100 rows / autocommit | 0.174847 | 0.186089 | 1.064x |
| 100 rows / batched | 0.077715 | 0.089517 | 1.152x |
| 100 rows / single txn | 0.077034 | 0.089026 | 1.156x |
| 1000 rows / autocommit | 0.852768 | 0.671888 | 0.788x |
| 1000 rows / batched | 0.395221 | 0.266790 | 0.675x |
| 1000 rows / single txn | 0.370400 | 0.264200 | 0.713x |
| 10000 rows / autocommit | 8.450000 | 6.070000 | 0.718x |
| 10000 rows / batched | 3.310000 | 2.010000 | 0.607x |
| 10000 rows / single txn | 3.260000 | 2.010000 | 0.617x |

The 100-row autocommit gap from the preceding full quick run did not reproduce
strongly in this focused profile. The remaining slower rows are tiny absolute
differences in the 100-row transaction tail; larger transaction rows are already
faster than C SQLite.

## Hot Counters

For `100 rows / autocommit`:

| Counter | Value |
|---|---:|
| `execute_body_ns` | 57,558 |
| `row_build_ns` | 13,532 |
| `btree_insert_ns` | 17,982 |
| `autocommit_begin_ns` | 15,712 |
| `autocommit_resolve_ns` | 23,004 |
| `btree_leaf_payload_appends` | 98 |
| `btree_cell_assembly_calls` | 2 |

For `100 rows / single txn`:

| Counter | Value |
|---|---:|
| `execute_body_ns` | 46,793 |
| `row_build_ns` | 11,143 |
| `btree_insert_ns` | 3,104 |
| `commit_roundtrip_ns` | 1,503 |
| `schema_validation_ns` | 3,228 |

The profile does not justify another right-edge payload append, page-run
admission, retained-autocommit threshold, transaction-control bypass, schema
lookup, FK guard, root predecode, row-template, fixed-cell staging, or arithmetic
expression micro-specialization. Those shapes are already rejected in
`docs/progress/perf-negative-results.md`, and this run did not show a new
dominant self-time outside those fenced areas.

## Superseding Evidence

The later local artifact commit `488176a2` publishes the current full quick
matrix at:

- `tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/full-quick.json`
- `tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/summary.md`

That matrix supersedes this focused transaction profile for target selection.
It shows the current C-faster rows are all write-side:

| Scenario | F/C ratio |
|---|---:|
| UPDATE/DELETE 100 rows / delete 5 rows | 1.381x |
| UPDATE/DELETE 100 rows / update 10 rows | 1.334x |
| INSERT single transaction large_10col / 10000 rows | 1.268x |
| INSERT single transaction small_3col / 100 rows | 1.222x |
| INSERT transaction strategy small_3col / 100 rows batched | 1.124x |
| INSERT record-size large_10col / 10000 rows | 1.102x |

It also shows the previous concurrent-writer tail is stale on current `main`.

## Decision

No source patch was attempted from this profile. The remaining 100-row
transaction-strategy gaps are noisy and small in absolute time, and the fresher
full quick matrix points at broader fixed write setup and large-row page-builder
work. A keepable next source change should remove a shared fixed write setup
cost across INSERT and UPDATE/DELETE, or implement a true large-row page-builder
design that improves both large-row medians and the full quick weighted score in
the same A/B window.
