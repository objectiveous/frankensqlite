# Direct INSERT Single-Pass Target Check

Date: 2026-05-08 02:45Z
Agent: CalmThrush
Head: `808fe4a9 docs(legal): adopt MIT License with OpenAI/Anthropic restricted-parties rider`

## Purpose

This artifact prevents a repeated optimization attempt on the remaining
100-row direct INSERT gaps. The obvious candidate was to make the prepared
direct INSERT record builder size fields and emit cells from one captured
layout. That change is already present in current `HEAD`.

## Evidence

- `ba8e9dae refactor(prepared): stream record-cell layout sizes during column
  iteration` changed `crates/fsqlite-core/src/connection.rs`.
- `PreparedDirectInsertRecordCell` already carries `value`, `serial_type`, and
  `payload_len` (`connection.rs:3237`).
- `try_serialize_prepared_direct_simple_insert_record()` already builds the
  cell list while accumulating `header_content_size` and `body_size`, then calls
  `serialize_prepared_direct_insert_record_cells_into()` (`connection.rs:18575`).
- The keep artifact is
  `tests/artifacts/perf/direct-insert-layout-crimsongorge-20260507T1950Z/summary.md`.
  It moved the full quick primary score from `0.35304311937129634` to
  `0.3445386401431955`, the average ratio from `0.46648821322957423` to
  `0.45557973340836866`, and the geomean from `0.26988400673068275` to
  `0.2635206749084158`.

## Current Target Map

Use `tests/artifacts/perf/calmthrush-clean-noprofile-20260508T0219Z/summary.md`
as the clean no-profile full matrix target order. Remaining rows above `1.05x`
are still dominated by:

- `UPDATE/DELETE 100 rows / delete 5 rows`: ratio `1.401515`
- `UPDATE/DELETE 100 rows / update 10 rows`: ratio `1.398978`
- 100-row direct INSERT variants: ratios `1.097814` to `1.192042`
- `2 writers x 1000 rows`: ratio `1.088204`

Use
`tests/artifacts/perf/head53367-clean-insert-profile-tanbear-20260508T0145Z/summary.md`
for the focused INSERT profile. Its relevant profile lines report
`serialize_ns=0`, so the remaining 100-row INSERT gaps are not
record-serializer limited. Visible costs are direct INSERT row expression/build
work plus fixed schema validation, MemDatabase, and change-tracking bookkeeping.

## Decision

Do not attempt another standalone record layout or `SmallVec` staging cleanup
for prepared direct INSERT. The next viable INSERT candidate needs to remove
several fixed costs together, for example a broader prepared row-template path
or bookkeeping amortization that proves a focused INSERT win and then passes the
same-window full quick gate.

At capture time, `crates/fsqlite-core/src/connection.rs` and
`docs/progress/perf-negative-results.md` were still reserved by CrimsonGorge, so
this artifact records target selection only and intentionally makes no source or
negative-ledger edits.

## Scratch Candidate Rejected

Scratch worktree:
`/data/tmp/frankensqlite-record-column-plan-calmthrush-20260508T0250Z`

Candidate shape:

- Add a prepared `PreparedDirectInsertRecordColumn` plan carrying
  `is_rowid_alias`, `notnull`, and decoded `TypeAffinity`.
- Add a prepared `has_strict_columns` flag.
- Use those prepared fields in
  `try_serialize_prepared_direct_simple_insert_record()` instead of rechecking
  strictness, rowid-alias status, and schema affinity on every direct INSERT
  execute.

Correctness proof:

```text
env TMPDIR=/data/tmp/frankensqlite-calmthrush-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-record-column-plan-target \
  CARGO_BUILD_JOBS=8 \
  cargo fmt -p fsqlite-core --check

env TMPDIR=/data/tmp/frankensqlite-calmthrush-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-record-column-plan-target \
  CARGO_BUILD_JOBS=8 \
  cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture
```

Result: `28` matching prepared-direct INSERT tests passed.

Focused A/B evidence:

- Baseline:
  `baseline-insert-clean-head.json`, built from clean detached worktree
  `/data/tmp/frankensqlite-record-column-plan-baseline-calmthrush-20260508T0300Z`
  at `808fe4a9`.
- Candidate:
  `candidate-insert-record-column-plan.json`, built from the scratch worktree
  above.

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Primary score | `0.7709957061652553` | `0.7984675558054796` |
| Average ratio | `0.8545592431377232` | `0.7874561591829939` |
| Geomean ratio | `0.7931548501371006` | `0.7675398476763968` |
| P90 ratio | `1.1371824859452584` | `1.0927267051662244` |
| P99 ratio | `2.3087027663511384` | `1.104941173169072` |
| C SQLite faster rows | `6` | `3` |

The non-primary aggregate numbers improved, but absolute FSQLite medians were
mixed: `medium_6col` 100-row INSERT improved `0.156242 ms -> 0.103534 ms`,
while `small_3col` 100-row INSERT worsened `0.081953 ms -> 0.089378 ms` and
the 100-row transaction-strategy rows also worsened slightly in absolute
FSQLite time. Several ratio gains came from C SQLite noise rather than a
uniform FSQLite improvement. The keep gate rejects the candidate because the
primary weighted score moved the wrong way.

Do not land or retry prepared direct-INSERT record-column metadata as a
standalone row-build optimization. Reconsider only as part of a broader
row-template execution plan that improves the focused INSERT primary score and
then passes the full quick matrix.
