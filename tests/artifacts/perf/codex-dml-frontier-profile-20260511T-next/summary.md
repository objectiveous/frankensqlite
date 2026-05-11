# Current DML Frontier Profile - 2026-05-11

## Command

Remote build/profile command:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/codex-frankensqlite-dml-profile-target CARGO_BUILD_JOBS=8 FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --json-out tests/artifacts/perf/codex-dml-frontier-profile-20260511T-next/update-delete-head-profile.json --no-html
```

The ignored JSON artifact was then materialized locally with the current
release-perf binary produced by that command:

```bash
env FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/codex-frankensqlite-dml-profile-target/release-perf/comprehensive-bench --quick --filter update --json-out tests/artifacts/perf/codex-dml-frontier-profile-20260511T-next/update-delete-head-profile.json --no-html
```

## Result

Current `HEAD`: `35a51b26a10846a96e25825043d565eff2e3c80c`.

| Scenario | C SQLite median | FrankenSQLite median | F/C ratio |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.004168 ms | 0.005590 ms | 1.341x |
| 100 rows / delete 5 rows | 0.002174 ms | 0.006462 ms | 2.972x |
| 1000 rows / update 100 rows | 0.036569 ms | 0.027321 ms | 0.747x |
| 1000 rows / delete 50 rows | 0.015590 ms | 0.027842 ms | 1.786x |
| 10000 rows / update 1000 rows | 0.357189 ms | 0.242133 ms | 0.678x |
| 10000 rows / delete 500 rows | 0.156172 ms | 0.261549 ms | 1.675x |

The focused slice reports 2 FSQLite-faster rows, 0 comparable rows, and 4
C-SQLite-faster rows. Write-single geomean is `1.3494470715628746`.

## Profile Boundary

The remaining DELETE tail stayed on the prepared direct path (`slow=0`):

- 5-row DELETE: `direct_delete=5`, `delete_leaf_flush=1/1`,
  `delete_leaf_materialize=1/1173`, `delete_leaf_write=1/260`.
- 50-row DELETE: `direct_delete=50`, `delete_leaf_flush=6/6`,
  `delete_leaf_materialize=6/9738`, `delete_leaf_write=6/863`.
- 500-row DELETE: `direct_delete=500`, `delete_leaf_flush=64/64`,
  `delete_leaf_materialize=64/78827`, `delete_leaf_write=64/7434`.

This repeats the current negative-ledger boundary: standalone retained
`TableLeafDeleteRun` admission, materialization, direct-writer, hint, and
wrapper tweaks are exhausted. The credible next source target remains the
broader transaction-local DML mutation operator with row-level semantics,
read/rollback/savepoint/MVCC proof, and focused plus full-quick gates.
