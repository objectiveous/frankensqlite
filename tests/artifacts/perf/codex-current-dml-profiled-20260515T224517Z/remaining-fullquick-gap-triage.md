# Remaining Full-Quick Gap Triage

- Date: 2026-05-15
- Matrix artifact:
  `tests/artifacts/perf/codex-current-fullquick-1c7f5b33-20260515T002530Z/full-quick.json`
- Matrix source commit:
  `1c7f5b33faee761fdb50cf556e97610a2f93ae4c`
- Current profile artifact:
  `tests/artifacts/perf/codex-current-dml-profiled-20260515T224517Z/summary.md`
- Superseded current-matrix artifact:
  `tests/artifacts/perf/codex-current-fullquick-refresh-20260515T003425Z/summary.md`

Fresh-eyes note: this file ranks the older `1c7f5b33` full-quick artifact. Keep
it as evidence for that run only. The later `06a37f61` full-quick refresh
reports the concurrent-writer rows as green, so the low-thread concurrent rows
listed below are not current source targets.

That older full-quick matrix reports `93` scenarios, with
FrankenSQLite faster / comparable / C-SQLite-faster at `78 / 2 / 13`, geomean
F/C `0.2795`, and primary weighted score `0.3852`.

## Ranked Red Rows

Rows with F/C ratio above `1.0` in the current full-quick matrix:

| Rank | Section | Scenario | Category | F/C | C median | F median | Current conclusion |
|---:|---|---|---|---:|---:|---:|---|
| 1 | UPDATE/DELETE | 100 rows / delete 5 rows | write_single | `3.2855x` | `0.002354 ms` | `0.007734 ms` | DML mutation operator, not retained-run micro-patch |
| 2 | UPDATE/DELETE | 1000 rows / delete 50 rows | write_single | `1.9567x` | `0.015750 ms` | `0.030818 ms` | DML mutation operator |
| 3 | UPDATE/DELETE | 10000 rows / delete 500 rows | write_single | `1.6791x` | `0.162334 ms` | `0.272580 ms` | DML mutation operator |
| 4 | UPDATE/DELETE | 100 rows / update 10 rows | write_single | `1.6280x` | `0.004228 ms` | `0.006883 ms` | transaction/DML lifecycle redesign |
| 5 | INSERT single txn medium_6col | 100 rows | write_bulk | `1.4271x` | `0.103283 ms` | `0.147396 ms` | 100-row fixed-cost/fused builder boundary; high F CV |
| 6 | Concurrent writers | 2 writers x 1000 rows | concurrent_writers | `1.1570x` | `11.888853 ms` | `13.755807 ms` | MVCC publication/retry representation, not wait-slice tweak |
| 7 | INSERT single txn small_3col | 100 rows | write_bulk | `1.1555x` | `0.079649 ms` | `0.092032 ms` | 100-row fixed-cost/fused builder boundary |
| 8 | INSERT transaction strategy small_3col | 100 rows / batched (100/txn) | write_bulk | `1.1350x` | `0.076113 ms` | `0.086391 ms` | 100-row fixed-cost/fused builder boundary |
| 9 | INSERT transaction strategy small_3col | 100 rows / single txn | write_bulk | `1.1043x` | `0.078397 ms` | `0.086572 ms` | 100-row fixed-cost/fused builder boundary |
| 10 | INSERT single txn large_10col | 100 rows | write_bulk | `1.0957x` | `0.150081 ms` | `0.164438 ms` | below 1.1x; fixed-cost/noise band |
| 11 | INSERT single txn tiny_1col | 100 rows | write_bulk | `1.0804x` | `0.068619 ms` | `0.074138 ms` | below 1.1x; high F CV |
| 12 | INSERT record-size large_10col | 10K rows | write_bulk | `1.0685x` | `9.742295 ms` | `10.409494 ms` | below 1.1x; fused row/page builder only |
| 13 | INSERT single txn large_10col | 10000 rows | write_bulk | `1.0650x` | `9.817947 ms` | `10.456512 ms` | below 1.1x; fused row/page builder only |
| 14 | Concurrent writers | 4 writers x 1000 rows | concurrent_writers | `1.0160x` | `20.522332 ms` | `20.851518 ms` | comparable/noise-sensitive |

The matrix has 13 C-SQLite-faster rows by the report's own classification; the
table includes the additional `4 writers x 1000 rows` row because it is above
`1.0x` but within the noise/comparable band.

## Source-Work Ranking

| Lever | Covers rows | Impact | Confidence | Effort | Score | Status |
|---|---|---:|---:|---:|---:|---|
| Transaction-local DML mutation operator with grouped leaf flush | DELETE rows; possibly small UPDATE after lifecycle integration | 5 | 3 | 5 | 3.0 | next viable DML source lever |
| Fused row/body/page construction and MVCC publication for INSERT | 100-row INSERT tails and 10K large row construction | 3 | 3 | 5 | 1.8 | below implementation gate until a sharper profile appears |
| MVCC publication/retry representation change for low-thread concurrent writers | 2-writer row, maybe 4-writer row | 3 | 2 | 5 | 1.2 | below implementation gate |
| Standalone retained DELETE run micro-optimization | DELETE rows | 1 | 2 | 2 | 1.0 | rejected by ledger |
| Standalone INSERT serializer/template/page-run tweak | INSERT tails | 1 | 2 | 2 | 1.0 | rejected by ledger |
| Standalone concurrent wait-slice/retry tweak | low-thread concurrent rows | 1 | 2 | 2 | 1.0 | rejected by ledger |

Score formula: `impact * confidence / effort`. The only current source lever
that clears the `>= 2.0` gate is the transaction-local DML mutation operator.

## Non-Repeat Boundaries

- DELETE: do not retry retained-run search/admission/materialization,
  direct-flush/publication wrappers, cancellation polling weakening,
  per-connection synced-write caches, tombstone-only overlays, or
  affected-count-only logical buffers. The current profile shows direct DELETE
  with `slow=0`, `vdbe_opcodes=0`, `delete_leaf_active=433/496`,
  `delete_leaf_miss=63`, and `delete_leaf_flush=64/64`.
- UPDATE: do not retry standalone fixed-width leaf-patch continuation,
  guard/flush prelude reordering, or exact transaction-control bypass. The
  remaining 100-row update row is fixed transaction/statement ceremony
  amortized over ten mutations.
- INSERT: do not retry standalone serializer, concat/param-one/template,
  scratch-borrow, page-run threshold/arena, prebuilt empty-root page builder,
  owned-record borrowed flush, direct page-image, parser/background wrapper, or
  setup-only PRAGMA/schema shortcuts.
- Concurrent writers: do not retry wait-slice tuning, active-holder
  preemption, retry-loop reshaping, standalone file-backed page-run admission,
  preserialized-record widening, witness-summary reuse, exact read-witness
  dedupe, or MVCC commit page-set container tweaks.

## Next Code Slice

The next code slice should be a proof-first transaction-local DML mutation
operator prototype, not another one-function trim. Minimum first milestone:

1. Define a transaction-owned logical DML buffer keyed by table root and rowid,
   with savepoint checkpoints and rollback truncation.
2. Add oracle tests for duplicate DELETE, missing DELETE, DELETE then UPDATE,
   rollback-to-savepoint, and read-boundary flushing against `rusqlite`.
3. Route only the safest private `:memory:` prepared direct-simple DELETE shape
   into the buffer after exact row-existence proof.
4. Flush the buffer through the existing physical path at every read/savepoint /
   commit boundary before attempting grouped leaf mutation.

That first milestone is expected to be correctness-heavy and may not improve the
benchmark yet. It should not be kept as a performance patch unless the second
milestone groups physical deletes by leaf and improves the focused DELETE
medians plus full-quick primary score in the same measurement window.
