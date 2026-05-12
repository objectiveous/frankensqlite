# Current DML / Vendored SQLite DELETE Boundary

Date: 2026-05-12

Commit under review: `bacda26169793a3645f10bece385e75d231740b2`

Reservation note: Agent Mail reservation could not be obtained for this narrow
docs/artifact update because `macro_start_session` and `health_check` both
timed out under database contention.

## Commands

Current DML profile:

```bash
env FSQLITE_BENCH_PROFILE_DML=1 \
  /tmp/frankensqlite-codex-span-materializer-target/release-perf/comprehensive-bench \
  --quick --filter update \
  --json-out tests/artifacts/perf/codex-current-dml-profile-20260512T0125/update-delete-profile.json \
  --no-html \
  2>&1 | tee tests/artifacts/perf/codex-current-dml-profile-20260512T0125/stdout.txt
```

Vendored C SQLite comparison:

```bash
rg -n "OP_Delete|sqlite3BtreeDelete|BTREE_SAVEPOSITION|CURSOR_SKIPNEXT" \
  legacy_sqlite_code/sqlite/src/vdbe.c \
  legacy_sqlite_code/sqlite/src/btree.c
```

## Profile Highlights

`UPDATE/DELETEThroughput` still has four red rows:

- `100 rows / update 10 rows`: C `4.2 us`, F `5.8 us`, `1.39x` slower.
- `100 rows / delete 5 rows`: C `2.2 us`, F `6.6 us`, `3.01x` slower.
- `1000 rows / delete 50 rows`: C `15.5 us`, F `28.1 us`, `1.81x` slower.
- `10000 rows / delete 500 rows`: C `184.9 us`, F `259.9 us`, `1.41x` slower.

All DELETE rows stayed on the prepared direct path (`slow=0`). The 500-row
DELETE profile still reports:

```text
delete_seek_ns=33703
delete_physical_ns=12945
delete_leaf_start=64/67
delete_leaf_active=433/496
delete_leaf_miss=63
delete_leaf_flush=64/64
delete_leaf_flush_ns=75450
delete_leaf_materialize=64/62093
delete_leaf_write=64/7915
commit_us=40.6
```

The benchmark binary warned that it predates Git HEAD. The intervening commit
`bacda261` was docs/artifact-only, so this profile is useful as a source-frontier
screen, but a final keep gate for any future Rust patch must rebuild.

## C SQLite Comparison

Vendored C SQLite routes `OP_Delete` to `sqlite3BtreeDelete`. When the delete
can avoid a rebalance, `BTREE_SAVEPOSITION` lets the cursor remain positioned in
`CURSOR_SKIPNEXT` state for the next step. That explains why cursor-position
preservation remains a tempting lead.

FrankenSQLite has already measured the comparable local family: retained direct
DML cursor shells, `BtCursor::advance_to`, next-cell hints, rowid bounds,
cursorless/direct flush, retained leaf-run materializers, and linear multi-leaf
backlogs. Those are fenced in the negative ledger. The next credible source
shape is not another cursor-preservation tweak; it is the broader
transaction-local DML mutation operator that removes the page-local
mutation/publication ceremony while preserving correctness.
