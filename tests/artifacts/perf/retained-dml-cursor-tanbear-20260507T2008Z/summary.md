# Retained direct-DML cursor handoff

Date: 2026-05-07
Agent: TanBear
Source head: `506711e2 docs(perf): publish direct insert layout keep gate`

## Status

No source patch was applied in this pass. The next allowed measured lever needs
`crates/fsqlite-core/src/connection.rs` and probably
`crates/fsqlite-btree/src/cursor.rs`, but both are under CrimsonGorge's active
exclusive Agent Mail reservation until `2026-05-07T22:02:20Z`. I sent a
coordination note and did not edit through the lock.

## Current Matrix Signal

Latest full quick artifact used as the current matrix:
`tests/artifacts/perf/direct-insert-layout-crimsongorge-20260507T1950Z/full-quick-layout.json`.

Summary:

- Scenarios: `93`
- FrankenSQLite faster / comparable / C SQLite faster: `79 / 5 / 9`
- Primary weighted score: `0.3445386401431955`
- Average ratio: `0.45557973340836866`
- Geomean ratio: `0.2635206749084158`
- p90 / p99 ratio: `1.0486389503409195 / 1.4470359808264062`

Top remaining C-faster rows are in `current-cfast-rows.tsv`. The head of that
ranking is:

| Ratio | Section | Scenario | C SQLite ms | FrankenSQLite ms |
| ---: | --- | --- | ---: | ---: |
| `1.4470359808264062` | `UPDATE/DELETEThroughput` | `100 rows / delete 5 rows` | `0.082405` | `0.119243` |
| `1.3849202643674583` | `UPDATE/DELETEThroughput` | `100 rows / update 10 rows` | `0.096381` | `0.133480` |
| `1.1537959907659803` | `INSERTThroughput - Single Transaction - small_3col` | `100 rows` | `0.076673` | `0.088465` |
| `1.1491347201118747` | `INSERTThroughput - Transaction Strategy Comparison (small_3col)` | `100 rows / single txn` | `0.074369` | `0.085460` |
| `1.1429855164393699` | `INSERTThroughput - Transaction Strategy Comparison (small_3col)` | `100 rows / batched (100/txn)` | `0.075672` | `0.086492` |

## Negative Ledger Gate

I checked the ledger and artifacts before considering source edits. The following
nearby ideas are already rejected and must not be repeated as standalone
optimizations:

- direct UPDATE/DELETE scratch-reset removal,
- fixed-width REAL leaf-payload patch,
- direct UPDATE/DELETE schema-proof microbatch carry,
- staged table-leaf delete mutation,
- delete scratch-reset narrowing.

The allowed retry condition in those ledger entries is a true retained-cursor or
batch direct-DML kernel that removes per-row cursor construction/root descent and
wins the same-window `UPDATE/DELETEThroughput` gate.

## Legacy SQLite Comparison

The legacy C path supports the retained-cursor direction:

- `legacy_sqlite_code/sqlite/src/vdbe.c` keeps a VDBE cursor open across
  repeated `OP_SeekRowid`, `OP_Delete`, `OP_RowData`, and `OP_Insert` work.
- `legacy_sqlite_code/sqlite/src/btree.c::sqlite3BtreeTableMoveto` first checks
  whether the cursor is already on the target key before doing a root seek.
- `sqlite3BtreeDelete` supports `BTREE_SAVEPOSITION`, leaving the cursor in a
  reusable state when no rebalance forces a full restore.

FrankenSQLite currently creates a fresh `BtCursor` for each prepared direct
UPDATE/DELETE execution, then calls `table_move_to` for each rowid. That makes
the benchmark's monotone rowid loops pay repeated cursor setup/root descent
that SQLite can amortize through an already-open cursor.

## Candidate Contract

Recommendation: implement the retained direct-DML cursor kernel from
`tests/artifacts/perf/retained-dml-cursor-plan-tanbear-20260507T1722Z/summary.md`
once the source reservation clears.

Start with the explicit-transaction concurrent direct UPDATE/DELETE lane because
that is the benchmark default (`BEGIN` promotes to concurrent mode). Keep the
first patch one lever wide:

1. Add a retained direct-DML cursor slot keyed by root page, page-size shape,
   schema generation, concurrent session id, and expected `total_changes`.
2. Reuse the actual `BtCursor<SharedTxnPageIo>` shell when the key matches.
3. Use `BtCursor::advance_to` in the helper for retained cursors; fresh cursors
   keep the current `table_move_to` path.
4. Drain the transaction back to `active_txn` before returning on every path.
5. Clear retained state on mismatch, error, memdb-mirror abandon, schema drift,
   or total-change drift.

Proof obligations:

- absent rowid remains `Ok(0)`;
- issue #73 fail-closed rowid skew still trips through the fallback seek path;
- no reuse after external writes or schema/session drift;
- transaction ownership stays single and explicit;
- concurrent-writer defaults remain unchanged.

## Keep Gate

Use same-window A/B, not stored C timings, because the remaining rows are small
and noisy:

1. Focused correctness:
   - direct UPDATE repeated prepared statement test,
   - direct DELETE repeated prepared statement test,
   - `cargo test -p fsqlite-btree test_table_seek_fails_closed_when_successor_contains_missed_rowid -- --nocapture`.
2. Focused isolated performance:
   - `perf-update-delete 1000 500 both compare isolated`,
   - `perf-update-delete 10000 100 both compare isolated`.
3. Section gate:
   - `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update`.
4. Full gate:
   - `comprehensive-bench --quick`.

Keep only if the candidate improves the isolated 1K/10K UPDATE and DELETE rows,
improves the section geomean/weighted score, and does not add C-faster rows in
the full quick matrix.
