# DML Profile After Active-Probe Attribution Fix - 2026-05-17

## Command

```bash
FSQLITE_BENCH_PROFILE_DML=1 rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-dml-profile-after-active-probe-fix-20260517 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-dml-profile-after-active-probe-fix-20260517T1730Z/update-delete-profile.json --no-html
```

The command ran through `rch` local fallback (`no admissible workers:
critical_pressure=6`) on `main @ 6b4181415c1e1a38c013b895cdca5f8ace522aaa`
with a dirty worktree. The benchmark binary was built with `release-perf`.

## Matrix Result

Artifact:
`tests/artifacts/perf/codex-dml-profile-after-active-probe-fix-20260517T1730Z/update-delete-profile.json`

Log:
`tests/artifacts/perf/codex-dml-profile-after-active-probe-fix-20260517T1730Z/run.log`

Summary:

- Total scenarios: `6`
- FrankenSQLite faster / comparable / C SQLite faster: `2 / 0 / 4`
- Average F/C ratio: `1.5000368728153883`
- Geomean weighted score for observed `write_single`: `1.3701115485179365`
- Median ratio: `1.7547436630695066`
- P99 ratio: `2.2553956834532376`

Rows:

| Scenario | C SQLite median | FrankenSQLite median | Ratio | CV C | CV F |
| --- | ---: | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 4.178 us | 6.502 us | 1.556x | 2.0% | 2.8% |
| 100 rows / delete 5 rows | 3.336 us | 7.524 us | 2.255x | 41.9% | 16.9% |
| 1000 rows / update 100 rows | 36.408 us | 29.425 us | 0.808x | 7.2% | 4.2% |
| 1000 rows / delete 50 rows | 15.599 us | 30.277 us | 1.941x | 2.4% | 13.8% |
| 10000 rows / update 1000 rows | 362.338 us | 248.084 us | 0.685x | 12.7% | 10.5% |
| 10000 rows / delete 500 rows | 159.107 us | 279.192 us | 1.755x | 9.3% | 14.2% |

## DELETE Profile Signal

This run was captured after `delete_active_probe_ns` was moved out of the
retained-run flush path. The active-probe counter now cleanly tracks the active
pending-run probe, while `delete_leaf_flush_ns` tracks the later flush.

Key DELETE rows:

- `fs_delete_100`: `mutate_us=6.5`, `commit_us=8.0`,
  `delete_active_probe_ns=1312`, `delete_leaf_flush_ns=1754`,
  `delete_leaf_materialize=1/1202`, `delete_leaf_search=5/441`,
  `delete_memdb_abandon=5/160`, `delete_memory_sync=5/491`,
  `delete_seek_ns=1072`.
- `fs_delete_1000`: `mutate_us=42.9`, `commit_us=14.7`,
  `delete_active_probe_ns=14679`, `delete_leaf_flush_ns=5881`,
  `delete_leaf_materialize=6/4248`, `delete_leaf_search=55/4566`,
  `delete_memdb_abandon=50/1241`, `delete_memory_sync=50/1611`,
  `delete_seek_ns=3867`, `delete_leaf_active=44/49`,
  `delete_leaf_miss=5`, `delete_leaf_miss_out_of_leaf=5`.
- `fs_delete_10000`: `mutate_us=479.4`, `commit_us=44.2`,
  `delete_active_probe_ns=139492`, `delete_leaf_flush_ns=64079`,
  `delete_leaf_materialize=64/50586`, `delete_leaf_search=560/39676`,
  `delete_dupcheck=500/12345`, `delete_compact=497/11699`,
  `delete_cellparse=497/13147`, `delete_memdb_abandon=500/12421`,
  `delete_memory_sync=500/13111`, `delete_seek_ns=35793`,
  `delete_physical_ns=12794`, `delete_leaf_start=64/67`,
  `delete_leaf_active=433/496`, `delete_leaf_miss=63`,
  `delete_leaf_miss_out_of_leaf=60`, `delete_leaf_miss_last_cell=3`.

## Interpretation

The stable red surface is still DELETE. The post-fix attribution shows the cost
is not a single obvious fixed-cost helper; it is spread across active retained
leaf-run probing, leaf materialization/flush, leaf search, duplicate checks,
cell parsing, and repeated per-row maintenance calls.

This rejects another isolated micro-patch from the current signal. The next
viable source-level lever remains a broader transaction-local DML mutation
operator that reduces per-row retained-run ceremony while preserving exact
affected-row counts, read-your-writes, rollback/savepoint behavior, duplicate
and missing rowid semantics, schema/cache invalidation, and MVCC publication.
