# Small INSERT Ceremony Profile

Date: 2026-05-07
Agent: TanBear
Source: `386bbb10 perf(insert): compact prepared concat segments`

## Scope

This pass refreshes the focused INSERT profile after the compact concat
segment win and after the retained direct-DML cursor shell was rejected. No
source files were changed in this pass.

`crates/fsqlite-core/src/connection.rs` was exclusively reserved by
CrimsonGorge until `2026-05-07T19:17:35Z`, so this artifact intentionally stops
at profiling and patch-ready target selection rather than editing through the
lock.

## Commands

```bash
env TMPDIR=/data/tmp/frankensqlite-small-insert-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-small-insert-target \
  CARGO_BUILD_JOBS=8 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf

env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-small-insert-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/small-insert-ceremony-tanbear-20260507T1741Z/current-insert-profile.json \
  --no-html
```

## Result

Focused INSERT quick matrix:

- Total scenarios: `25`
- FrankenSQLite faster: `18`
- C SQLite faster: `7`
- Average ratio: `0.8374791388898462`
- Geomean ratio: `0.8135700103736332`
- P90 ratio: `1.110406298186012`
- P99 ratio: `1.1521225043613104`
- Insert-only weighted score: `0.8040382465232678`

Remaining C-faster INSERT rows in this same-host run:

| Ratio | Scenario | C SQLite ms | FrankenSQLite ms |
| ---: | --- | ---: | ---: |
| `1.1521` | large_10col, 100 rows | `0.145327` | `0.167431` |
| `1.1104` | small_3col, 100 rows | `0.079769` | `0.088576` |
| `1.0944` | record-size large_10col, 10K rows | `9.994944` | `10.938894` |
| `1.0837` | medium_6col, 100 rows | `0.100445` | `0.108851` |
| `1.0582` | tiny_1col, 100 rows | `0.069733` | `0.073792` |
| `1.0169` | small_3col, 100 rows / single txn strategy | `0.083701` | `0.085113` |
| `1.0029` | small_3col, 100 rows / batched strategy | `0.081046` | `0.081281` |

## Hotspot Read

The remaining INSERT gap is dominated by row-build/record construction rather
than cursor setup:

| Scenario | row_build_ns | btree_insert_ns | schema_validation_ns | commit_roundtrip_ns |
| --- | ---: | ---: | ---: | ---: |
| `small_3col` 100 | `14661` | `3104` | `3206` | `1773` |
| `medium_6col` 100 | `20998` | `3525` | `3233` | `5360` |
| `large_10col` 100 | `43455` | `5327` | `3287` | `13225` |
| `small_3col` 10K single txn | `1374242` | `643351` | `329694` | `37551` |
| `medium_6col` 10K single txn | `2040016` | `351545` | `329893` | `365685` |
| `large_10col` 10K single txn | `4375274` | `616884` | `313676` | `2450250` |

The current B-tree append path is not the primary exposed cost on the small
rows, and the negative-results ledger already fences standalone append hints,
page-run replay, staged-page publication splits, and one-row leaf hints.

## Legacy SQLite Comparison

Legacy SQLite's `OP_MakeRecord` in
`legacy_sqlite_code/sqlite/src/vdbe.c` measures serial types into each input
`Mem.uTemp`, then writes header and payload into a reused output register
buffer. It avoids a per-row side `SmallVec` layout object on the common path
and stores the serial type beside the value it just inspected.

FrankenSQLite's prepared direct insert path currently builds a
`SmallVec<[PreparedDirectInsertRecordValue; 16]>`, then builds a second
`SmallVec<[(serial_type, payload_len); 16]>` inside
`serialize_prepared_direct_insert_record_values_into`, then writes the record.
That is the next high-EV seam because it matches the measured `row_build_ns`
dominance and is not a B-tree-only retry.

## Opportunity Matrix

| Candidate | Impact | Confidence | Effort | Score | Disposition |
| --- | ---: | ---: | ---: | ---: | --- |
| Single-pass prepared direct-insert record builder in `connection.rs`, storing serial layout beside `PreparedDirectInsertRecordValue` and writing without the second layout `SmallVec` | 4 | 4 | 2 | 8.0 | Best next source slice once `connection.rs` is available |
| B-tree cached rightmost leaf tweak only | 2 | 2 | 2 | 2.0 | Do not start here; profile says row-build dominates and ledger fences B-tree-only hint tweaks |
| Retained direct UPDATE/DELETE cursor shell | 3 | 1 | 3 | 1.0 | Rejected by update/delete matrix; see negative-results ledger |

## Patch-Ready Next Step

When `connection.rs` is available, inspect and narrow:

- `PreparedDirectInsertRecordValue`
- `try_serialize_prepared_direct_simple_insert_record`
- `serialize_prepared_direct_insert_record_values_into`

The candidate should be one lever only: carry `(value, serial_type,
payload_len)` in the first row-build pass so serialization does not recompute
layout or allocate a second `SmallVec`. The behavior proof is byte-for-byte
record equivalence against existing prepared direct-insert tests plus the
focused insert matrix above.
