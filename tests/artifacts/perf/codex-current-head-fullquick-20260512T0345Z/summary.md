# Current-HEAD Fullquick Frontier Refresh

Date: 2026-05-12

Commit under review: `fc14dbb5e02d5ed2bf36d7528fce76f9b1a058a8`

## Command

```bash
mkdir -p tests/artifacts/perf/codex-current-head-fullquick-20260512T0345Z

/data/tmp/frankensqlite-target/release-perf/comprehensive-bench --quick \
  --json-out tests/artifacts/perf/codex-current-head-fullquick-20260512T0345Z/full.json \
  --no-html \
  2>&1 | tee tests/artifacts/perf/codex-current-head-fullquick-20260512T0345Z/stdout.txt
```

The benchmark reported `Git dirty: yes` because this artifact directory and the
existing untracked RCH scratch directories were present. It also warned that
the benchmark binary predates Git HEAD; the intervening HEAD commit was the
artifact-only `fc14dbb5`, with no Rust source changes after the rebuilt
`0c016144` binary. Treat this as a current source-frontier run, not as a fresh
release rebuild proof.

## Summary

- Total scenarios: `93`.
- FrankenSQLite faster / comparable / C SQLite faster: `79 / 3 / 11`.
- Average F/C: `0.4847690929`.
- Geomean F/C: `0.2645495519`.
- Median F/C: `0.2991679386`.
- p90 / p99 F/C: `1.0561467448 / 2.9870188004`.
- Primary metric `per_category_weighted.score`: `0.3608218364`.

Category geomeans:

- `read_single`: `0.2054214721`.
- `read_aggregate`: `0.0724778915`.
- `write_bulk`: `0.8011727614`.
- `write_single`: `1.1604018572`.
- `concurrent_writers`: `0.8181208600`.
- `mixed`: `0.1856942385`.

## Rows Above 1.05x F/C

| Section | Scenario | Category | C median ms | F median ms | F/C |
| --- | --- | --- | ---: | ---: | ---: |
| `UPDATE/DELETEThroughput` | `100 rows / delete 5 rows` | `write_single` | `0.002234` | `0.006673` | `2.987x` |
| `UPDATE/DELETEThroughput` | `1000 rows / delete 50 rows` | `write_single` | `0.016040` | `0.029265` | `1.825x` |
| `UPDATE/DELETEThroughput` | `10000 rows / delete 500 rows` | `write_single` | `0.178595` | `0.271849` | `1.522x` |
| `UPDATE/DELETEThroughput` | `100 rows / update 10 rows` | `write_single` | `0.008456` | `0.011532` | `1.364x` |
| `INSERTThroughput - Single Transaction - small_3col` | `100 rows` | `write_bulk` | `0.075792` | `0.085249` | `1.125x` |
| `INSERTThroughput - Single Transaction - large_10col` | `100 rows` | `write_bulk` | `0.143178` | `0.159078` | `1.111x` |
| `Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC` | `2 writers x 1000 rows` | `concurrent_writers` | `11.824826` | `12.947959` | `1.095x` |
| `INSERTThroughput - Transaction Strategy Comparison (small_3col)` | `100 rows / single txn` | `write_bulk` | `0.074500` | `0.080731` | `1.084x` |
| `INSERTThroughput - Single Transaction - medium_6col` | `100 rows` | `write_bulk` | `0.099185` | `0.106569` | `1.074x` |
| `INSERTThroughput - Transaction Strategy Comparison (small_3col)` | `100 rows / autocommit` | `write_single` | `0.116338` | `0.122870` | `1.056x` |
| `INSERTThroughput - Single Transaction - tiny_1col` | `100 rows` | `write_bulk` | `0.066444` | `0.069911` | `1.052x` |

## Frontier Read

The durable red surface is still DML DELETE. The fullquick rows match the
focused DML refresh: DELETE is red at all row counts while UPDATE is only red
on the smallest fixed-cost row. Prior current-HEAD artifacts have already
closed standalone retained leaf-run, scratch reset, exact transaction-control,
direct writer, and one-line cell-log hook attempts.

The remaining INSERT rows are all 100-row fixed-cost tails. Current focused
INSERT artifacts fenced off another standalone row serializer or page-run
patch; larger INSERT rows are already green enough that a narrow patch is more
likely to move noise than the primary score.

The remaining concurrent writer row is the 2-writer row. The 8-writer row is
green in the focused concurrent artifact, so the next concurrent lever would
need a broader file-backed page construction plus MVCC publication change, not
another low-thread retry tuning patch.

## Next Source Shape

The next credible performance lever is still the broader transaction-local DML
mutation operator: an RLU/B-epsilon-tree-style write-log path built on the
existing cell delta scaffolding, with a materialized transaction read view and
MVCC publication integration. It must prove:

- read-your-writes for INSERT/UPDATE/DELETE in the same transaction;
- rollback and savepoint ownership for logical cell deltas;
- duplicate and missing rowid behavior matching the direct DML path;
- schema drift and prepared statement invalidation behavior;
- quotient-filter and count-cache invalidation;
- logical-page `commit_index` publication for delta-only transactions;
- focused DELETE wins plus fullquick primary-score neutrality or better.

## Source-Surface Recheck

A follow-up source read covered `crates/fsqlite-mvcc/src/cell_visibility.rs`,
`crates/fsqlite-mvcc/src/lifecycle.rs`,
`crates/fsqlite-mvcc/src/materialize.rs`, and
`crates/fsqlite-mvcc/src/cell_routing.rs`. The existing `CellVisibilityLog`
can record deltas, bulk-commit them, roll them back on full transaction abort,
and materialize explicit delta lists. That is still not a safe narrow hot-path
patch: uncommitted deltas are not a general transaction read view, savepoints do
not own cell-delta positions, and live B-tree reads/writes are not wired through
the materialized logical page surface. A preparatory patch that only records
DELETE deltas or only updates one commit-side counter would therefore be a
correctness trap rather than a measured optimization.
