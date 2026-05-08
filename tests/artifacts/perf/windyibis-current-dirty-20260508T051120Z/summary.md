# Landed Schema/Env Probe Verification

Date: 2026-05-08
Agent: WindyIbis
Commit verified: `a6418c20487918c72fa03e6d703f393205843db3`

## Context

The shared checkout initially had an uncommitted candidate diff:

- `Connection::schema_index_of()` skipped `to_ascii_lowercase()` when the
  incoming table name was already lowercase.
- `fsqlite-pager::resolve_page_buffer_max()` cached
  `FSQLITE_PAGE_BUFFER_MAX` env parsing with `OnceLock`.

While the release-perf build was running, another agent landed the exact change
as:

```text
a6418c20 perf(core,pager): skip lowercase alloc in schema lookup; cache page-buffer env probe
```

This bundle is therefore verification for the landed peer commit, not a new
WindyIbis code change.

## Build

Primary local build:

```text
env TMPDIR=/data/tmp/frankensqlite-windyibis-tmp \
  CARGO_TARGET_DIR=.rch-target \
  CARGO_BUILD_JOBS=16 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

RCH build also completed successfully, but took 25m due to a cold remote
release-perf build.

## Full Quick Gate

Baseline:
`tests/artifacts/perf/calmthrush-clean-noprofile-20260508T0219Z/full-quick-clean-noprofile.json`.

Candidate:
`tests/artifacts/perf/windyibis-current-dirty-20260508T051120Z/full-quick.json`.

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Primary weighted score | 0.345939 | 0.351285 |
| Average ratio | 0.454261 | 0.471355 |
| Geomean ratio | 0.267475 | 0.267868 |
| Median ratio | 0.292509 | 0.319661 |
| P90 ratio | 0.981159 | 1.045104 |
| P99 ratio | 1.401515 | 1.607150 |
| FrankenSQLite faster rows | 80 | 78 |
| Comparable rows | 5 | 6 |
| C-SQLite-faster rows | 8 | 9 |

Lower is better for score and ratios. The full quick matrix did not confirm a
broad win.

## Current Slow Rows

Top rows above `1.05x` in the candidate full quick run:

| Ratio | Category | Scenario |
| ---: | --- | --- |
| 1.607150 | write_single | UPDATE/DELETE: 100 rows / update 10 rows |
| 1.529465 | write_bulk | INSERT small_3col: 100 rows |
| 1.353182 | write_single | UPDATE/DELETE: 100 rows / delete 5 rows |
| 1.305860 | write_bulk | INSERT medium_6col: 100 rows |
| 1.162849 | write_bulk | INSERT large_10col: 100 rows |
| 1.145683 | write_bulk | INSERT strategy: 100 rows / batched |
| 1.134839 | concurrent_writers | 2 writers x 1000 rows |
| 1.130955 | write_bulk | INSERT strategy: 100 rows / single txn |
| 1.068926 | write_bulk | record-size large_10col 10K rows |

## Focused Profiles

`update-delete-profile.json`:

- 6 scenarios; average ratio `1.089074`, geomean `1.077395`.
- Mixed result: 100-row delete improved in the focused run, but 100-row update
  and 10K rows still lagged, and the full matrix remained the keeper gate.

`insert-profile.json`:

- 25 scenarios; average ratio `0.87`.
- Remaining insert losses are 100-row fixed-cost rows; larger insert rows are
  usually faster than C SQLite.

## Disposition

Do not retry the landed schema-lowercase plus page-buffer-env-cache pair as a
standalone setup-cost optimization. It may be behaviorally fine, but this
verification did not confirm a broad end-to-end matrix win.

I did not edit `docs/progress/perf-negative-results.md` because it was reserved
by `CrimsonGorge`. I sent a patch-ready ledger entry on Agent Mail thread
`perf-negative-results`.

I also did not edit `crates/fsqlite-core/src/connection.rs` for the next small
write probe because it was reserved by `CalmThrush`. I sent the next concrete
lead on Agent Mail thread `perf-small-write-followup`: use the now-fast
`schema_index_of()` in `prepared_direct_simple_insert_plan()` instead of the
current linear `schema.iter().find(... eq_ignore_ascii_case ...)` table lookup,
then measure the 100-row INSERT rows and full quick matrix.
