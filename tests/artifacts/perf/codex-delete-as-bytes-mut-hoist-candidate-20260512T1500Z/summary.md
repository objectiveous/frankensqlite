# DELETE materializer `as_bytes_mut` hoist candidate

Date: 2026-05-12
Base commit: `ed0c79130a4781f6268ce222167ae1e5bb2dc0c0`
Candidate file: `crates/fsqlite-btree/src/cursor.rs`

## Candidate

Tested a one-lever source variant in
`TableLeafDeleteRun::materialize_deletions_incremental_descending`: hoist
`self.entry.page_data.as_bytes_mut()` out of the per-deleted-cell loop and reuse
the same mutable page byte slice through final header/pointer writes.

The candidate was not kept. The source patch was reverted after measurement.

## Commands

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-hoist-target \
  CARGO_BUILD_JOBS=4 \
  cargo test -p fsqlite-btree test_table_leaf_delete_run -- \
  --nocapture --test-threads=1

env CARGO_TARGET_DIR=/data/tmp/frankensqlite-delete-hoist-target \
  CARGO_BUILD_JOBS=4 \
  cargo build --profile release-perf -p fsqlite-e2e \
  --bin comprehensive-bench --bin perf-update-delete

env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-delete-hoist-target/release-perf/comprehensive-bench \
  --quick --filter update \
  --json-out tests/artifacts/perf/codex-delete-as-bytes-mut-hoist-candidate-20260512T1500Z/update-delete-candidate.json

env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-delete-hoist-target/release-perf/comprehensive-bench \
  --quick --filter update --no-html \
  --json-out tests/artifacts/perf/codex-delete-as-bytes-mut-hoist-candidate-20260512T1500Z/update-delete-candidate-repeat.json
```

## Results

Committed repeat baseline:
`tests/artifacts/perf/codex-current-dml-refresh-after-insert-frontier-20260512T1615Z/dml-repeat.json`.

| Scenario | Baseline C ms | Baseline F ms | Candidate F ms | Repeat F ms | Verdict |
|---|---:|---:|---:|---:|---|
| `100 rows / delete 5 rows` | `0.003647` | `0.008967` | `0.006933` | `0.006993` | improved F median, but repeat F CV was `132.9%` |
| `1000 rows / delete 50 rows` | `0.015990` | `0.028714` | `0.029415` | `0.028724` | flat to slightly worse |
| `10000 rows / delete 500 rows` | `0.159789` | `0.258423` | `0.277640` | `0.276948` | worse |

The repeat candidate run reported geomean F/C `1.3974486211` across the six
UPDATE/DELETE rows, versus the committed repeat baseline geomean
`1.3229800457`. The 500-delete row is the key failure: it regressed from
`0.258423ms` to roughly `0.277ms`.

## Verdict

Rejected. The hoist removes repeated method calls locally, but does not move the
focused DELETE section in the intended direction. Do not retry this exact
borrow-hoist micro-patch unless a new profile proves `PageData::as_bytes_mut`
entry overhead, rather than allocation/page materialization, is the dominant
remaining cost.
