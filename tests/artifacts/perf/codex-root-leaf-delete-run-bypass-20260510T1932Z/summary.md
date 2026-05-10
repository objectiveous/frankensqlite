# Root-Leaf DELETE Run Bypass Screen

Date: 2026-05-10 UTC

Source baseline: `4b0546d6c0e00545738633f19605b29b242d5f11`

Candidate shape: temporarily made `BtCursor::table_leaf_delete_run_current`
decline root-leaf table deletes (`tree_depth == 1`), so tiny root-leaf DELETEs
used the ordinary cursor path while non-root retained same-leaf DELETE runs
remained enabled.

The source patch was reverted after the focused gate failed.

## Commands

```bash
cargo fmt -p fsqlite-btree --check
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-rootleaf-delete-target CARGO_BUILD_JOBS=8 \
  cargo test -p fsqlite-btree table_leaf_delete_run -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-rootleaf-delete-target CARGO_BUILD_JOBS=8 \
  cargo test -p fsqlite-core test_prepared_direct_delete_leaf_run -- --nocapture --test-threads=1
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-rootleaf-delete-target CARGO_BUILD_JOBS=8 \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
/data/tmp/frankensqlite-codex-dml-perf-20260510T191902Z/release-perf/comprehensive-bench \
  --quick --filter UPDATE \
  --json-out tests/artifacts/perf/codex-root-leaf-delete-run-bypass-20260510T1932Z/baseline-update-delete.json \
  --no-html
/data/tmp/frankensqlite-codex-rootleaf-delete-target/release-perf/comprehensive-bench \
  --quick --filter UPDATE \
  --json-out tests/artifacts/perf/codex-root-leaf-delete-run-bypass-20260510T1932Z/candidate-update-delete.json \
  --no-html
```

## Result

Rejected. The candidate improved focused geomean only slightly, from
`1.6169050307` to `1.6039644124`, but it worsened the intended tiny DELETE row
and the p90/p99 tail.

| Scenario | Baseline ratio | Candidate ratio | Baseline F ms | Candidate F ms |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 1.647439 | 1.553327 | 0.006883 | 0.006583 |
| 100 rows / delete 5 rows | 3.470310 | 4.462775 | 0.008065 | 0.010550 |
| 1000 rows / update 100 rows | 0.847608 | 0.821713 | 0.030758 | 0.031078 |
| 1000 rows / delete 50 rows | 2.103442 | 2.011915 | 0.033613 | 0.032251 |
| 10000 rows / update 1000 rows | 0.722487 | 0.760674 | 0.274814 | 0.281787 |
| 10000 rows / delete 500 rows | 2.426463 | 1.953315 | 0.388598 | 0.314349 |

The `fsqlite-core` pending delete-run tests failed under the candidate because
they correctly assert that root-leaf same-leaf direct deletes stay buffered
until read/rollback boundaries. Since the benchmark target row also regressed,
the source and temporary test edits were unwound.
