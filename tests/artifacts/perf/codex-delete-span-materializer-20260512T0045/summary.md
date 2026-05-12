# Compact DELETE Span Materializer Rejection

- Date: 2026-05-12.
- Source: `6b4f3f873cf7a7abac7860ad6a5060de1c5691f8` plus a reverted
  `crates/fsqlite-btree/src/cursor.rs` candidate.
- Build: local release-perf,
  `CARGO_TARGET_DIR=/tmp/frankensqlite-codex-span-materializer-target`.
- Artifact:
  `tests/artifacts/perf/codex-delete-span-materializer-20260512T0045/`.

## Candidate

The candidate replaced the compact descending `TableLeafDeleteRun`
materializer's per-live-cell copy loop with contiguous live-span copies between
deleted cells. The first probe also lowered `COMPACT_DELETE_SINGLE_PASS_MIN`
from `6` to `2`; the second probe restored the existing threshold and measured
only the span materializer on the old one-pass path. The source patch was
manually unwound after the focused gate failed.

## Proof Before Measurement

- `cargo fmt -p fsqlite-btree --check`
- `env CARGO_TARGET_DIR=/tmp/frankensqlite-codex-span-materializer-target CARGO_BUILD_JOBS=4 cargo test -p fsqlite-btree test_table_leaf_delete_run -- --nocapture --test-threads=1`
- `env CARGO_TARGET_DIR=/tmp/frankensqlite-codex-span-materializer-target CARGO_BUILD_JOBS=4 cargo test -p fsqlite-core test_prepared_direct_delete_leaf_run -- --nocapture --test-threads=1`

After a SIGKILL left the first debug target with stale linker objects, the
restored-threshold B-tree test was rerun successfully in
`/tmp/frankensqlite-codex-span-materializer-threshold6-target` without deleting
the poisoned target directory.

## Focused Results

| Probe | Scenario | FSQLite median | C SQLite median | Ratio |
|---|---|---:|---:|---:|
| threshold 2 | 100 rows / delete 5 rows | 0.009498 ms | 0.004658 ms | 2.03907x |
| threshold 2 | 1000 rows / delete 50 rows | 0.028533 ms | 0.015909 ms | 1.79351x |
| threshold 2 | 10000 rows / delete 500 rows | 0.257392 ms | 0.160179 ms | 1.60690x |
| threshold 6 | 100 rows / delete 5 rows | 0.006923 ms | 0.002324 ms | 2.97892x |
| threshold 6 | 1000 rows / delete 50 rows | 0.030497 ms | 0.017032 ms | 1.79057x |
| threshold 6 | 10000 rows / delete 500 rows | 0.260628 ms | 0.159889 ms | 1.63006x |

## Decision

Rejected. The span materializer reduced the large DELETE materialization
counter (`delete_leaf_materialize` dropped to about `40-42 us` for the 500-row
DELETE), but the focused medians did not clear the keep gate. The threshold-2
probe worsened the 100-row DELETE absolute median, and the restored-threshold
probe left the 1000/10000-row DELETE medians flat to slightly worse versus the
current frontier repeat.

Do not retry live-span compact DELETE materialization as a standalone
`TableLeafDeleteRun` optimization. Reconsider only inside the broader
transaction-local DML mutation operator if it removes more of the page-local
publication path and wins all focused DELETE medians in a same-window A/B.
