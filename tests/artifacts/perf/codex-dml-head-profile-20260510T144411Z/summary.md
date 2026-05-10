# DML Head Profile Refresh

Date: 2026-05-10

Command:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-dml-head-target CARGO_BUILD_JOBS=4 FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-dml-head-profile-20260510T144411Z/update-delete.json --no-html
```

The remote benchmark completed successfully, but local target-directory
retrieval was stopped after the run finished. The JSON report was not retained
locally; the raw stderr capture remains local and ignored.

Summary from the completed run:

- Total scenarios: 6
- FrankenSQLite faster / comparable / C SQLite faster: `1 / 0 / 5`
- Average time ratio: `2.19x`

Key profile lines:

- `fs_delete_100`: `mutate_us=6.1`, `commit_us=10.8`,
  `direct_delete=5`, `delete_leaf_start=1/1`, `delete_leaf_active=4/4`,
  `delete_leaf_miss=0`, `delete_leaf_flush=1/1`,
  `delete_leaf_flush_ns=2845`, `direct_flush_ns=3266`.
- `fs_delete_1000`: `mutate_us=43.6`, `commit_us=12.6`,
  `direct_delete=50`, `delete_leaf_start=6/6`,
  `delete_leaf_active=44/49`, `delete_leaf_miss=5`,
  `delete_leaf_miss_out_of_leaf=5`, `delete_leaf_flush=6/6`,
  `delete_leaf_flush_ns=17054`.
- `fs_delete_10000`: `mutate_us=443.8`, `commit_us=54.3`,
  `direct_delete=500`, `delete_leaf_start=64/67`,
  `delete_leaf_active=433/496`, `delete_leaf_miss=63`,
  `delete_leaf_miss_out_of_leaf=60`, `delete_leaf_miss_last_cell=3`,
  `delete_leaf_flush=64/64`, `delete_leaf_flush_ns=151339`,
  `pager_mem_flush_ns=14287`, `pager_cache_finish_ns=20238`.

Conclusion:

The current DML tail is still not a safe one-lever source-code optimization.
Small DELETE is already a single same-leaf retained run, and large DELETE is
dominated by many leaf-run flushes at out-of-leaf boundaries. The next credible
change is a transaction-level multi-leaf DELETE buffering design with
read-your-writes, savepoint/rollback, and MVCC publication proof, followed by a
same-window focused UPDATE/DELETE A/B and full quick gate.
