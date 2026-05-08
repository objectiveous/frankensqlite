# Current INSERT Frontier Reprofile

Date: 2026-05-08
Agent: WindyIbis
Source commit: `536dc300512ff37adb47e36b6ed9150dea3f8e1d`

## Purpose

The previous full-quick artifact
`tests/artifacts/perf/boldlion-small-record-append-20260508T0800Z/candidate-full-quick.json`
showed the largest non-DML C-relative row as:

- `INSERTThroughput - Single Transaction - medium_6col / 100 rows` at
  `1.4215818419x`, with FSQLite CV `39.38%`.

This pass rebuilt `comprehensive-bench` from the current clean source and
reprofiled INSERT before considering source changes. The goal was to determine
whether the `medium_6col` 100-row spike was a stable optimization target or a
noisy fixed-cost row.

## Build

```text
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-windyibis-medium100-profile-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

RCH built successfully on worker `vmi1152480`. The retrieved binary was:

```text
/data/tmp/frankensqlite-windyibis-medium100-profile-target/release-perf/comprehensive-bench
```

## Artifacts

- `current-insert-profile.json`: focused `--filter INSERT` with
  `FSQLITE_BENCH_PROFILE_INSERT=1`.
- `current-insert-profile.stdout`
- `current-insert-profile.stderr`
- `current-full-quick.json`: full `--quick` matrix without profiling.
- `current-full-quick.stdout`
- `current-full-quick.stderr`
- `current-insert-repeat.json`: focused `--filter INSERT` repeat without
  profiling.
- `current-insert-repeat.stdout`
- `current-insert-repeat.stderr`

## Current Full Quick

`current-full-quick.json`:

| Metric | Value |
| --- | ---: |
| Average ratio | `0.4579682579` |
| Geomean ratio | `0.2656030248` |
| Primary weighted score | `0.3473581467` |
| P90 ratio | `1.0219971036` |
| P99 ratio | `1.4000453363` |
| Faster / comparable / slower | `80 / 4 / 9` |

Rows above `1.10x`:

| Row | Ratio | FSQLite median | C SQLite median | F CV% |
| --- | ---: | ---: | ---: | ---: |
| UPDATE/DELETE 100 rows / update 10 rows | `1.400045` | `0.117349 ms` | `0.083818 ms` | `7.87` |
| UPDATE/DELETE 100 rows / delete 5 rows | `1.367213` | `0.110788 ms` | `0.081032 ms` | `6.18` |
| INSERT single txn medium_6col / 1000 rows | `1.260124` | `0.681025 ms` | `0.540443 ms` | `14.07` |
| INSERT small_3col / 100 rows batched | `1.141845` | `0.084919 ms` | `0.074370 ms` | `4.20` |
| INSERT small_3col / 100 rows single txn | `1.135437` | `0.084078 ms` | `0.074049 ms` | `2.40` |
| INSERT single txn small_3col / 100 rows | `1.109678` | `0.084138 ms` | `0.075822 ms` | `3.25` |

## Focused INSERT Repeat

`current-insert-repeat.json`:

| Metric | Value |
| --- | ---: |
| Average ratio | `0.8466318835` |
| Geomean ratio | `0.8255254525` |
| Primary weighted score | `0.8219659600` |
| P90 ratio | `1.1295845166` |
| P99 ratio | `1.1615123606` |
| Faster / comparable / slower | `16 / 4 / 5` |

The focused repeat did not reproduce the full-quick `medium_6col` 1000-row
slowdown:

| Row | Ratio | FSQLite median | C SQLite median | F CV% |
| --- | ---: | ---: | ---: | ---: |
| medium_6col / 100 rows | `1.077553` | `0.108293 ms` | `0.100499 ms` | `8.63` |
| medium_6col / 1000 rows | `0.824269` | `0.446246 ms` | `0.541384 ms` | `4.29` |
| medium_6col / 10000 rows | `0.651373` | `3.606847 ms` | `5.537303 ms` | `14.45` |

## Profile Counters

The profiled `medium_6col` 100-row run also did not reproduce the older spike:

```text
ratio=1.081416
setup_us=17.1 begin_us=9.6 prepare_us=11.4 insert_us=65.8 commit_us=19.3
rows=100 row_build_ns=21579 btree_insert_ns=3576 schema_validation_ns=3268
memdb_apply_ns=2423 change_tracking_ns=2395 page_pool_misses=7
```

For comparison, the focused repeat's stable small fixed-cost rows are only
about `8-12 us` slower than C SQLite at 100 rows. That is below the threshold
where the existing negative ledger supports a standalone direct-INSERT source
change: recent row-template, concat, FK-guard, schema-lookup, root-predecode,
page-run admission, and fixed-cell staging attempts all failed broader gates.

## Decision

No source candidate was kept or attempted from this INSERT profile. The
published `medium_6col` 100-row spike was not stable on the current binary, and
the remaining INSERT slow rows are small fixed-cost tails already covered by
multiple rejected standalone direct-INSERT ideas.

The next source work should either target the still-stable 100-row
UPDATE/DELETE tails with a true broader DML batch/leaf-run design, or target
INSERT only through a broader row/page-builder design that explicitly protects
large-row full-quick behavior.
