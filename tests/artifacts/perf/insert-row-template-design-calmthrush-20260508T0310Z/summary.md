# Direct INSERT Row-Template Design Notes

Date: 2026-05-08 03:10Z
Agent: CalmThrush
Head: `1ce10b19 docs(perf): reject direct insert column plan`

## Why This Exists

The next source-owned performance pass should not repeat standalone record
serialization work. The current clean matrix and focused profiles point at
prepared direct INSERT row construction and fixed per-execute bookkeeping, but
`crates/fsqlite-core/src/connection.rs` is currently reserved by CrimsonGorge
for a no-FK direct INSERT guard probe through `2026-05-08T04:50:17Z`.

This artifact records the next viable design direction while the source file is
blocked.

## Current Evidence

Clean no-profile target map:
`tests/artifacts/perf/calmthrush-clean-noprofile-20260508T0219Z/summary.md`

Remaining rows above `1.05x` in that run:

- `UPDATE/DELETE 100 rows / delete 5 rows`: ratio `1.401515`
- `UPDATE/DELETE 100 rows / update 10 rows`: ratio `1.398978`
- 100-row direct INSERT variants: ratios `1.097814` to `1.192042`
- `2 writers x 1000 rows`: ratio `1.088204`

Focused DML profile:
`tests/artifacts/perf/current-post-dml-tanbear-20260508T0110Z/summary.md`

That profile shows the small UPDATE/DELETE rows are mostly setup/prepopulation
and statement ceremony, not the direct mutation kernel:

| Row | setup_us | prepare_us | mutate_us | commit_us |
| --- | ---: | ---: | ---: | ---: |
| `fs_update_100` | `56.1` | `13.0` | `12.0` | `6.0` |
| `fs_delete_100` | `55.8` | `11.9` | `8.4` | `5.1` |

Focused INSERT profile:
`tests/artifacts/perf/head53367-clean-insert-profile-tanbear-20260508T0145Z/summary.md`

The INSERT profile reports `serialize_ns=0` on the slow rows. The visible
costs are:

- `row_build_ns`
- `btree_insert_ns`
- `memdb_apply_ns`
- `schema_validation_ns`
- `change_tracking_ns`

## Legacy SQLite Shape

The C SQLite row-write path is still a compact register-to-record-to-btree
pipeline:

- `legacy_sqlite_code/sqlite/src/insert.c:1569` calls
  `sqlite3GenerateConstraintChecks()`.
- `legacy_sqlite_code/sqlite/src/insert.c:1585` calls
  `sqlite3CompleteInsertion(..., appendFlag, bUseSeek)`.
- `legacy_sqlite_code/sqlite/src/insert.c:2714` emits `OP_MakeRecord` for the
  table row.
- `legacy_sqlite_code/sqlite/src/insert.c:2842` emits `OP_Insert` with append
  and seek-result flags.
- `legacy_sqlite_code/sqlite/src/vdbe.c:3467` implements `OP_MakeRecord` by
  applying affinity over contiguous registers, sizing serial types into
  per-register scratch (`Mem.uTemp`), then emitting the record.
- `legacy_sqlite_code/sqlite/src/vdbe.c:5746` implements `OP_Insert` by
  passing the already-built record buffer to `sqlite3BtreeInsert()`.
- `legacy_sqlite_code/sqlite/src/btree.c:9394` keeps cursor state and can turn
  an append/seeking hint into less btree work; `btreeOverwriteCell()` handles
  same-size replacement when the cursor is already on the target row.

FrankenSQLite already has a direct path, but it still evaluates a tree of
`PreparedDirectSimpleInsertExpr` values and separately checks multiple fixed
guards on every execute before btree insertion. The rejected scratch candidate
in `tests/artifacts/perf/single-pass-direct-insert-calmthrush-20260508T0245Z/`
proved that merely precomputing column metadata is not enough.

## No-Retry Fences

Do not retry these as standalone direct INSERT changes:

- record-cell layout sizing; already kept in `ba8e9dae`
- direct serializer one-byte header shortcut
- fixed cell array staging
- lazy param-one text caching
- param-one concat or numeric binary-op specialization
- row-value metadata sparsification
- prepared direct-INSERT record-column metadata
- page-run threshold/admission changes without a true page builder
- file-backed preserialized-record widening
- no-FK direct INSERT guard until CrimsonGorge's current probe lands or is
  rejected

## Next Candidate Shape

The next viable source change should be a prepared row-template executor, not a
single helper tweak.

Required shape:

- Compile the benchmark-shaped direct INSERT expressions into a flat per-column
  template at prepare time.
- The template should emit `PreparedDirectInsertRecordCell` values directly,
  avoiding recursive `eval_prepared_direct_simple_insert_expr()` for common
  segments.
- Keep one execution loop that handles rowid extraction, null propagation,
  concat text append, numeric multiply/modulo for `?1 * K` and `?1 % K`,
  affinity, and serial-type sizing in one pass.
- Leave unsupported expressions on the existing compiled-expression fallback.
- Preserve current bind-error order, rowid-alias NULL storage, NaN-to-NULL
  behavior, NOT NULL errors, strict-table fallback, FK fallback, and OR REPLACE
  fallback.
- Measure with `comprehensive-bench --quick --filter insert` first, then full
  quick only if the focused primary score improves.

Do not keep a candidate that only improves average/geomean while worsening the
focused INSERT primary weighted score. The record-column plan did exactly that
and was rejected.

## CASS Check

Recent 60-day CASS searches for these exact concepts returned no direct prior
attempts:

- `frankensqlite row-template direct INSERT`
- `frankensqlite record-column metadata`
- `frankensqlite row_build_ns schema_validation_ns change_tracking_ns memdb_apply_ns`

Treat that as absence of an indexed exact-match lead, not proof that no adjacent
idea exists. The negative ledger remains the authority before source work.
