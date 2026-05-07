# Depth-2 Right-Edge Page-Builder Insert Repeat

- Agent: TanBear
- Date: 2026-05-07
- Baseline binary: `/data/tmp/frankensqlite-tanbear-clean-target/release-perf/comprehensive-bench`
- Candidate binary: `/data/tmp/frankensqlite-tanbear-target/release-perf/comprehensive-bench`
- Candidate source state: dirty shared worktree at source commit
  `60e1434f5d001180ce4258e1dfb34be55c06036f`, with only artifact/docs commit
  `e52bcfa1` added afterward.
- Candidate owner: CrimsonGorge. TanBear did not edit, stage, or revert
  `crates/fsqlite-btree/src/cursor.rs` or
  `crates/fsqlite-core/src/connection.rs`.

Both binaries report that they predate Git HEAD because the artifact-only commit
`e52bcfa1` was created between the original build and this repeat. That commit
does not change benchmarked source code.

## Commands

Clean repeat:

```bash
/data/tmp/frankensqlite-tanbear-clean-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/right-edge-depth2-insert-repeat-tanbear-20260507T1431Z/clean-insert.json \
  --no-html
```

Dirty repeat:

```bash
/data/tmp/frankensqlite-tanbear-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/right-edge-depth2-insert-repeat-tanbear-20260507T1431Z/dirty-insert.json \
  --no-html
```

## Section Result

| Metric | Clean | Dirty |
|---|---:|---:|
| Total scenarios | 25 | 25 |
| FrankenSQLite faster / comparable / C faster | 13 / 4 / 8 | 14 / 2 / 9 |
| Average F/C ratio | 0.924466 | 0.924235 |
| Geomean F/C ratio | 0.893070 | 0.890727 |
| Primary weighted score | 0.902771 | 0.921972 |

The repeat confirms the target row win, but the INSERT-section primary score is
slightly worse. This is not a clean standalone keep signal for the full INSERT
section.

## Target Confirmation

| Row | Clean F ms | Dirty F ms | F delta | Clean F/C | Dirty F/C |
|---|---:|---:|---:|---:|---:|
| `10000 rows / batched (1000/txn)` | 4.336625 | 2.507627 | -42.18% | 1.285520 | 0.792889 |

The depth-2 page-builder candidate again closes the intended
transaction-strategy row.

## Largest Dirty FSQLite Regressions

| Row | Clean F ms | Dirty F ms | F delta | Note |
|---|---:|---:|---:|---|
| `1000 rows / autocommit` | 0.728394 | 1.012487 | +39.00% | high dirty F CV: 25.89% |
| `tiny_1col` 100 rows | 0.069290 | 0.081132 | +17.09% | small absolute row |
| `large_10col` 100 rows | 0.169909 | 0.184546 | +8.61% | high dirty F CV: 31.74% |
| record-size `large_10col` 10K | 10.815963 | 11.579393 | +7.06% | repeat confirms large-record concern |
| `small_3col` 10K single-txn | 2.895064 | 3.001402 | +3.67% | small but opposite target direction |

## Largest Dirty FSQLite Wins

| Row | Clean F ms | Dirty F ms | F delta |
|---|---:|---:|---:|
| `10000 rows / batched (1000/txn)` | 4.336625 | 2.507627 | -42.18% |
| `medium_6col` 10K single-txn | 5.662639 | 4.259219 | -24.78% |
| `1000 rows / single txn` | 0.370203 | 0.299712 | -19.04% |
| `small_3col` 100 rows | 0.103233 | 0.087133 | -15.60% |
| `tiny_1col` 1000 rows | 0.185337 | 0.164738 | -11.11% |

## Decision

This repeat supports the candidate as a targeted fix for the non-empty
right-edge transaction row, but it does not settle the final landing decision.
The source owner should either:

1. narrow the page-builder admission so the large-record and small-row
   regressions disappear, or
2. publish a fresh source-owned full quick rebuild/repeat showing that the
   full-matrix win is stable enough to outweigh the INSERT-section regression.

Do not treat the target-row win alone as sufficient. The full matrix remains
the keep gate.
