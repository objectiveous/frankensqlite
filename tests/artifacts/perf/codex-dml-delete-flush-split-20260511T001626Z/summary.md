# Direct DELETE Leaf-Run Flush Split Profile

Date: 2026-05-11 00:16:36 UTC

Base commit: `50191116e9bd31c7071bc3ffbcc6edfc9ad808f3`

Source state: base commit plus working-tree instrumentation delta in:

- `crates/fsqlite-btree/src/instrumentation.rs`
- `crates/fsqlite-btree/src/cursor.rs`
- `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`

Command:

```bash
FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-delete-flush-split-target/release-perf/comprehensive-bench --quick --filter update --json-out tests/artifacts/perf/codex-dml-delete-flush-split-20260511T001626Z/update-profile.json --no-html
```

## Result

The retained same-leaf DELETE flush is mostly page materialization, not the pager write call.

| Scenario | DELETE leaf flush | Materialize | Pager write | Unattributed |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 1.993 us | 1.052 us | 0.611 us | 0.330 us |
| 1000 rows / delete 50 rows | 14.047 us | 10.069 us | 2.736 us | 1.242 us |
| 10000 rows / delete 500 rows | 120.148 us | 80.862 us | 27.668 us | 11.618 us |

For the largest quick-matrix DELETE row, FrankenSQLite measured 315.210 us median versus C SQLite 161.943 us median, or 1.946x slower. The retained leaf-run flush accounted for 120.148 us of profiled time; 80.862 us of that was materializing retained runs into page images and 27.668 us was handing those page images to the pager.

This points the next optimization away from the pager write call itself and toward the retained-run page image materialization shape or a broader mutation representation that avoids rebuilding whole page images per retained leaf run.
