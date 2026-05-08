# SilverAnchor DML CPU profile

- Repo: clean detached worktree at `3f8aa91fe66378f01492f9940fcd860604708d3a`
- Binary: `/data/tmp/frankensqlite-silveranchor-dml-profile-target/release-perf/comprehensive-bench`
- Valid command:
  `env FSQLITE_BENCH_PROFILE_DML=1 perf record -F 997 -g --call-graph dwarf -o /data/projects/frankensqlite/tests/artifacts/perf/silveranchor-dml-cpu-profile-20260508T1320Z/perf-dml.data -- /data/tmp/frankensqlite-silveranchor-dml-profile-target/release-perf/comprehensive-bench --quick --filter update --json-out /data/projects/frankensqlite/tests/artifacts/perf/silveranchor-dml-cpu-profile-20260508T1320Z/dml-profile.json --no-html`
- Report command:
  `perf report --stdio --no-children -i tests/artifacts/perf/silveranchor-dml-cpu-profile-20260508T1320Z/perf-dml.data > tests/artifacts/perf/silveranchor-dml-cpu-profile-20260508T1320Z/perf-dml-report.txt`

## Result

The clean 100-row DML rows are still slower than C SQLite:

- `update-deletethroughput__100-rows-update-10-rows`: `1.2736x` (`91.792 us` C, `116.909 us` F)
- `update-deletethroughput__100-rows-delete-5-rows`: `1.5562x` (`90.048 us` C, `140.132 us` F)

The larger rows are already faster than C SQLite in this same run:

- `update-deletethroughput__1000-rows-update-100-rows`: `0.8551x`
- `update-deletethroughput__1000-rows-delete-50-rows`: `0.8635x`
- `update-deletethroughput__10000-rows-update-1000-rows`: `0.8080x`
- `update-deletethroughput__10000-rows-delete-500-rows`: `0.7237x`

## Interpretation

The remaining small-row DML gap is setup dominated, not direct DML dominated.
The instrumented 100-row FrankenSQLite lanes report:

- Update: `setup_us=66.9`, `mutate_us=12.8`, `commit_us=6.2`, `mutations=10`
- Delete: `setup_us=52.9`, `mutate_us=9.2`, `commit_us=5.5`, `mutations=5`

The top FrankenSQLite perf samples are also in setup/insert work, especially:

- `Connection::try_serialize_prepared_direct_simple_insert_record`: `7.00%`
- `Connection::execute_precompiled_prepared_insert_fast`: `1.77%`
- `Connection::eval_prepared_direct_simple_insert_expr`: `1.34%`
- `BtCursor::bulk_table_leaf_groups`: `1.01%`
- `BtCursor::delete`: only `0.67%`

This does not support another standalone fixed-width UPDATE payload patch or
DELETE leaf-run patch. The next credible lever for this row is the same broader
prepared direct INSERT setup/row-template/page-run work identified by the insert
profile, and it should stay fenced by the existing negative ledger entries.
