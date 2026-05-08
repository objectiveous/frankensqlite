# Next Frontier Refresh - 2026-05-08

## Source Basis

- Clean benchmark worktree: `/data/projects/frankensqlite-rusticgrove-clean-next-20260508T1630Z`
- Benchmark commit: `f749770ccc32857cf936ae8ce9f48f15e00ca233`
- Main checkout caveat: the shared checkout had dirty page-builder edits in:
  - `crates/fsqlite-btree/src/cursor.rs`
  - `crates/fsqlite-btree/src/lib.rs`
  - `crates/fsqlite-core/src/connection.rs`
- Coordination caveat: `SwiftGate` held exclusive reservations on `cursor.rs` and `connection.rs`, so this pass did not edit or stage those files.
- CASS status: unhealthy/stale lexical index, semantic unavailable. A targeted search for same-leaf DML/page-builder history returned no fresh hits, so this decision is based on the negative ledger plus current benchmark artifacts.

## Build

```bash
env CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=12 \
  cargo build --profile release-perf \
  -p fsqlite-e2e \
  --bin comprehensive-bench \
  --bin mt-mvcc-bench \
  --bin perf-update-delete
```

Result: passed.

## Commands

```bash
env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/cargo-target/release-perf/comprehensive-bench \
  --quick --filter update \
  --json-out tests/artifacts/perf/rusticgrove-next-frontier-20260508T1630Z/dml-profile.json \
  --html tests/artifacts/perf/rusticgrove-next-frontier-20260508T1630Z/dml-profile.html
```

```bash
/data/tmp/cargo-target/release-perf/comprehensive-bench \
  --quick --filter concurrent \
  --json-out tests/artifacts/perf/rusticgrove-next-frontier-20260508T1630Z/concurrent-profile.json \
  --html tests/artifacts/perf/rusticgrove-next-frontier-20260508T1630Z/concurrent-profile.html
```

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/cargo-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/rusticgrove-next-frontier-20260508T1630Z/insert-profile.json \
  --html tests/artifacts/perf/rusticgrove-next-frontier-20260508T1630Z/insert-profile.html
```

```bash
/data/tmp/cargo-target/release-perf/comprehensive-bench \
  --quick \
  --json-out tests/artifacts/perf/rusticgrove-next-frontier-20260508T1630Z/full-quick.json \
  --html tests/artifacts/perf/rusticgrove-next-frontier-20260508T1630Z/full-quick.html
```

## Full Quick Result

- Generated: `2026-05-08 16:35:38 UTC`
- Scenarios: `93`
- Faster / comparable / slower: `80 / 4 / 9`
- Average ratio: `0.4574106763`
- Geomean ratio: `0.2665951426`
- Weighted score: `0.3347931621`
- P90 / P99 ratio: `1.0460722171 / 1.4474956173`

Rows above `1.0x` in the full quick run:

| Ratio | Category | Section | Scenario | C ms | F ms | C CV | F CV |
| ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| `1.4475` | write_single | UPDATE/DELETE | 100 rows / delete 5 rows | `0.079860` | `0.115597` | `4.2` | `48.3` |
| `1.3795` | write_bulk | INSERT single txn medium_6col | 100 rows | `0.099816` | `0.137698` | `5.1` | `14.1` |
| `1.1693` | write_bulk | INSERT record size | large_10col 10K | `9.883759` | `11.557161` | `6.6` | `12.8` |
| `1.1404` | write_bulk | INSERT single txn large_10col | 10K rows | `9.512725` | `10.848134` | `0.5` | `11.8` |
| `1.1305` | write_bulk | INSERT single txn medium_6col | 1000 rows | `0.577691` | `0.653073` | `0.9` | `21.6` |
| `1.1192` | write_bulk | INSERT txn strategy small_3col | 100 rows / single txn | `0.072756` | `0.081432` | `9.4` | `3.1` |
| `1.1165` | write_bulk | INSERT single txn small_3col | 100 rows | `0.073037` | `0.081543` | `6.8` | `8.3` |
| `1.1082` | write_single | UPDATE/DELETE | 100 rows / update 10 rows | `0.106720` | `0.118271` | `48.0` | `33.9` |
| `1.0639` | write_bulk | INSERT single txn tiny_1col | 100 rows | `0.064731` | `0.068869` | `12.1` | `6.3` |
| `1.0461` | concurrent_writers | 2 writers x 1000 rows | 2 writers x 1000 rows | `11.993649` | `12.546223` | `2.8` | `8.8` |
| `1.0283` | write_bulk | INSERT txn strategy small_3col | 100 rows / batched | `0.081482` | `0.083787` | `21.9` | `8.6` |
| `1.0067` | write_bulk | INSERT single txn large_10col | 100 rows | `0.160610` | `0.161693` | `23.3` | `5.7` |

## Focused DML Result

- Scenarios: `6`
- Faster / comparable / slower: `1 / 4 / 1`
- Average ratio: approximately `1.02x`
- 100-row update: `0.0818 ms` C SQLite vs `0.1162 ms` FSQLite, ratio `1.42x`, F CV `9.6%`
- 100-row delete: `0.1403 ms` C SQLite vs `0.1101 ms` FSQLite, faster in this focused run
- Larger 1000-row and 10K-row DML rows were comparable or faster.

100-row DML counters:

| Row | setup_us | begin_us | prepare_us | mutate_us | commit_us | direct ops | page misses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| update 10/100 | `83.6` | `9.8` | `19.1` | `20.6` | `9.4` | `10` | `0` |
| delete 5/100 | `50.0` | `5.0` | `11.3` | `8.2` | `5.4` | `5` | `0` |

## Focused Insert Result

- Scenarios: `25`
- Faster / comparable / slower: `19 / 3 / 3`
- Average ratio: approximately `0.78x`
- The focused insert run did not reproduce the full-run large-row gap:
  - record-size `large_10col` 10K: `9.38 ms` C SQLite vs `9.31 ms` FSQLite
  - single-txn `large_10col` 10K: `9.38 ms` C SQLite vs `9.12 ms` FSQLite
- The remaining focused slower rows were small 100-row fixed-cost cases:
  - `small_3col` 100 rows: `1.06x`, with high C/F variance
  - 100-row batched/single transaction small_3col: around `1.11x`

## Focused Concurrent Result

- Scenarios: `3`
- Faster / comparable / slower: `2 / 1 / 0`
- Average ratio: approximately `0.78x`
- 2 writers x 1000 rows: `1.03x` focused, `1.046x` full quick
- 4 and 8 writers were faster than C SQLite.

## Negative-Ledger Fence

The apparent gaps sit inside already-fenced families unless a broader operator changes the shape:

- DML: private-memory `SharedTxnPageIo` bypass, retained DML cursor shell, retained seek hints, direct DELETE clone removal, no-rebalance delete, fixed-width REAL assignment shortcuts, lazy scratch borrow, staged overwrite probes, page-one cleanup caches.
- INSERT: direct schema lookup, root predecode, row-template executor, concat encoder, no-FK guard cache, param text/arithmetic caches, fixed cell staging, record-cell layout reuse, page-run threshold/admission variants, retained page-run widening, global page-buffer growth.
- WAL/concurrent: standalone checksum coefficient precompute already lost the concurrent quick gate.

## Decision

No source patch was attempted.

The current clean frontier is better than several earlier snapshots and does not expose a stable independent source lever:

1. Focused DML says only 100-row update is slower; full quick says 100-row delete is the worst row, but with `48%` FSQLite CV.
2. Focused INSERT says large_10col 10K is comparable/faster; full quick says it is slower, again with high FSQLite variance.
3. Concurrent writers are comparable or faster, so a WAL pipeline patch is not justified by this run.
4. The one plausible source family, a true page-builder / leaf-run operator, is already being edited in the shared checkout under another agent's reservations. A read-only `cargo check -p fsqlite-core --lib` on that dirty slice passed, but it emitted a `dead_code` warning for `BulkTableLeafPageBuilder::reset_current_page`, which would need cleanup before a `-D warnings` closeout.

Next keep gate: wait for the reserved page-builder slice to become reviewable, or require a new same-window A/B candidate that improves repeated focused INSERT plus DML and then the full quick weighted score. The current measurements do not justify another standalone micro-optimization.
