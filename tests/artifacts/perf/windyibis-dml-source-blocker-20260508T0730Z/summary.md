# Direct DML Source Blocker

Date: 2026-05-08
Agent: WindyIbis
Base evidence:

- `tests/artifacts/perf/windyibis-dirty-pagebuf256-timing-full-20260508T0710Z/full-quick.json`
- `tests/artifacts/perf/windyibis-dirty-pagebuf256-timing-20260508T0705Z/update-profile.json`
- `tests/artifacts/perf/windyibis-insert-profile-log-20260508T0715Z/insert-profile.json`
- `docs/progress/perf-negative-results.md`

## Current State

The dirty page-buffer/timing integration remains a small weighted improvement
over the prior keeper:

| Metric | Dirty integration |
| --- | ---: |
| Primary weighted score | 0.3348866468 |
| Average ratio | 0.4461065430 |
| Geomean ratio | 0.2575107741 |
| P90 ratio | 0.9757275559 |
| P99 ratio | 1.4291042441 |
| Faster / comparable / slower | 81 / 3 / 9 |

The current tail is concentrated in setup-heavy 100-row DML and small INSERT
rows, not in the larger DML rows:

| Ratio | Scenario | C SQLite | FrankenSQLite |
| ---: | --- | ---: | ---: |
| 1.4291 | 100 rows / update 10 rows | 0.079899 ms | 0.114184 ms |
| 1.3893 | tiny_1col / 100 rows | 0.062557 ms | 0.086913 ms |
| 1.1390 | medium_6col / 100 rows | 0.097743 ms | 0.111329 ms |
| 1.1378 | 100 rows / batched (100/txn) | 0.071164 ms | 0.080972 ms |
| 1.1266 | 2 writers x 1000 rows | 13.281167 ms | 14.963211 ms |
| 1.1105 | 100 rows / delete 5 rows | 0.108303 ms | 0.120275 ms |

Focused UPDATE/DELETE evidence from the dirty profile shows the section is now
near parity overall:

| Metric | Focused DML |
| --- | ---: |
| Weighted / geomean score | 1.0467937415 |
| Average ratio | 1.0617534252 |
| P90 / P99 ratio | 1.3307941900 |
| Faster / comparable / slower | 3 / 1 / 2 |

The remaining slower focused DML rows are the 100-row cases:

- `100 rows / update 10 rows`: C `86.2 us`, F `113.0 us`, ratio `1.31x`.
- `100 rows / delete 5 rows`: C `78.9 us`, F `105.0 us`, ratio `1.33x`.

The profile attributed most of those rows to setup, not row mutation:

- Update setup `51.2 us`, mutate `12.2 us`.
- Delete setup `52.3 us`, mutate `8.6 us`.

## Rejected Source Shapes

The negative ledger fences the obvious direct-DML source moves:

- Retained direct UPDATE/DELETE cursor shell, including `BtCursor::advance_to`.
- Retained table-seek hint across fresh direct-DML cursors.
- Direct UPDATE fixed-width REAL leaf payload patch.
- Direct DELETE no-rebalance leaf primitive.
- Direct UPDATE/DELETE per-row scratch reset removal.
- Direct UPDATE/DELETE schema-proof microbatch carry.
- Lazy VDBE fallback compilation for direct UPDATE/DELETE.
- Certified direct UPDATE/DELETE logical buffering.
- Certified direct UPDATE/DELETE scan-merge flushing.
- Connection-level pending fixed-width REAL same-leaf UPDATE run.

Several of these improved isolated microbenchmarks, but failed the actual
`comprehensive-bench --quick --filter update` section or the same-window full
matrix. The current code already contains the small primitives those candidates
needed, including `BtCursor::advance_to`,
`table_insert_prechecked_absent`, and
`table_overwrite_current_payload_same_size_no_overflow`; reusing them per row
has not moved the gate.

## Blocker

The source files that still matter for the dirty integration are owned by
another active reservation:

- `crates/fsqlite-e2e/src/fsqlite_executor.rs`
- `crates/fsqlite-pager/src/page_buf.rs`
- `crates/fsqlite-wal/src/group_commit.rs`
- `crates/fsqlite-wal/src/lib.rs`

I did not edit those files. The unreserved DML source path is blocked because
all narrow standalone variants I could justify are already rejected by the
ledger.

## Next Viable Shape

A new direct-DML attempt should be a different operator, not another retained
cursor or one-row helper:

1. Accept a sorted rowid/value run at the B-tree layer.
2. Walk each touched leaf once.
3. Decode/project only the columns required for correctness.
4. Apply all same-leaf mutations while the leaf page is already staged.
5. Emit one page write per dirty leaf where possible.
6. Prove an isolated UPDATE/DELETE win before any full-matrix run.

Do not start from connection-local cursor retention, same-size REAL patching,
or standalone no-rebalance DELETE. Those paths already failed keep gates.
