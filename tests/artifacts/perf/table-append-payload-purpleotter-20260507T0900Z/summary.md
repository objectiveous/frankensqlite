# Table Append Payload Candidate

- Agent: PurpleOtter
- Date: 2026-05-07
- Source state: dirty local candidate on top of the current performance branch;
  source was manually removed before this summary was written.
- Target: transaction-strategy and write-single rows where the insert profile
  showed autocommit staying on `btree_leaf_full_cell_appends` while explicit
  paths used the cheaper payload append kernel.

## Candidate

The candidate added a byte-slice variant of the existing right-edge leaf payload
append path in `crates/fsqlite-btree/src/cursor.rs` and routed
`table_append_after_last_position` through it before falling back to the normal
full-cell insert path.

## Evidence

- Baseline insert section:
  `baseline-insert.json`
- Candidate insert section:
  `candidate-insert.json`
- Clean current full quick baseline:
  `../current-full-quick-purpleotter-20260507T0857Z/report.json`
- Candidate full quick first run:
  `candidate-full.json`
- Baseline full repeat:
  `baseline-full-repeat.json`
- Candidate full repeats:
  `candidate-full-repeat.stderr` and `candidate-full-repeat2.stderr`

The insert section looked directionally good:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Primary score | 1.187148 | 1.166217 |
| Geomean ratio | 0.945056 | 0.911234 |
| Write-single geomean | 1.299599 | 1.286171 |
| Write-bulk geomean | 0.904881 | 0.869402 |

The first full quick run also looked promising:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Primary score | 0.380185 | 0.358372 |
| Geomean ratio | 0.284254 | 0.277993 |
| C-faster rows | 18 | 16 |
| Write-single geomean | 1.219092 | 0.999125 |
| Write-bulk geomean | 0.932556 | 0.869539 |

## Rejection

The candidate failed the repeat full-matrix gate twice. Both candidate repeat
runs panicked in the 8-writer concurrent benchmark before producing JSON:

```text
fsqlite COMMIT tid=3 failed: database is busy (snapshot conflict on pages: ) (retry_count=64)
fsqlite COMMIT tid=2 failed: database is busy (snapshot conflict on pages: ) (retry_count=64)
```

Because the full benchmark matrix did not complete under repeat, the candidate
is rejected/abandoned despite the initial insert and full-score improvement.

Retry only if the right-edge byte-slice append path is redesigned with an
explicit concurrent-writer correctness proof, a focused multi-writer stress
gate, and a same-window full quick repeat that completes successfully.
