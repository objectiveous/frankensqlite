# e644bd64 Current Frontier Refresh

Date: 2026-05-13

Commit:
`e644bd64eefea85d67e0eb9a813eacee3b2790de`
(`fix(mvcc): lock cell-delta-only commit pages`)

## Commands

```bash
/data/tmp/cargo-target/release-perf/comprehensive-bench \
  --quick \
  --no-html \
  --json-out tests/artifacts/perf/codex-e644bd64-frontier-refresh-20260513T0248Z/full-quick.json
```

```bash
FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/cargo-target/release-perf/comprehensive-bench \
  --quick \
  --filter update \
  --no-html \
  --json-out tests/artifacts/perf/codex-e644bd64-frontier-refresh-20260513T0248Z/update-delete-profile.json
```

Both commands were run from a clean canonical worktree using the `release-perf`
profile binary built under `/data/tmp/cargo-target`.

## Full Quick Result

- Scenarios: `93`
- FrankenSQLite faster / comparable / C SQLite faster: `78 / 6 / 9`
- Average F/C time ratio: `0.4964158116`
- Geomean F/C time ratio: `0.2752616803`
- Median F/C time ratio: `0.3086122222`
- p90 F/C time ratio: `1.0490572078`
- p99 F/C time ratio: `3.0527225583`
- Primary weighted score: `0.3710116820`

## Remaining Red Rows

| Category | Scenario | F/C |
|---|---|---:|
| write_single | `100 rows / delete 5 rows` | `3.0527x` |
| write_single | `1000 rows / delete 50 rows` | `1.8557x` |
| write_single | `10000 rows / delete 500 rows` | `1.6418x` |
| write_single | `100 rows / update 10 rows` | `1.4313x` |
| write_bulk | 100-row INSERT shapes | `1.09x` to `1.16x` |
| concurrent_writers | `2 writers x 1000 rows` | `1.0491x` |
| concurrent_writers | `4 writers x 1000 rows` | `1.0402x` |

Medium and large UPDATE rows remain faster than C SQLite in the same matrix.

## Focused DML Profile

Focused UPDATE/DELETE scenarios: `2 / 0 / 4` faster/comparable/slower, with
geomean F/C `1.3423972979` and p99 F/C `3.0859002169`.

DELETE rows stayed on the prepared direct path (`slow=0`) but remained red:

| Scenario | C median ms | F median ms | F/C |
|---|---:|---:|---:|
| `100 rows / delete 5 rows` | `0.002305` | `0.007113` | `3.0859x` |
| `1000 rows / delete 50 rows` | `0.015930` | `0.028924` | `1.8157x` |
| `10000 rows / delete 500 rows` | `0.158206` | `0.276939` | `1.7505x` |

Representative `fs_delete_10000` counters:

- `direct_delete=500`, `fast=500`, `slow=0`
- `delete_leaf_active=433/496`, `delete_leaf_miss=63`
- `delete_leaf_flush=64/64`
- `delete_leaf_materialize=64/39933ns`
- `delete_leaf_write=64/7453ns`
- `delete_leaf_search=560/39508ns`
- `execute_body_ns=41287`
- `commit_roundtrip_ns=20789`

## Decision

No source patch was kept or attempted from this refresh. The current evidence
still fences off another retained DELETE search, duplicate-check, compactness,
materialization, direct-flush, publication, or low-thread concurrent
micro-patch. The next source frontier remains the broader transaction-local DML
mutation operator: logical rowid/key messages, read-boundary flushing or a
logical read-view overlay, rollback/savepoint ownership, row-count oracle tests,
focused DELETE wins, and full-quick primary-score neutrality or better.
