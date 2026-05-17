# Current Full-Quick Frontier - 2026-05-17

## Command

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-fullquick-20260517 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --json-out tests/artifacts/perf/codex-current-fullquick-frontier-20260517T1900Z/full-quick.json --no-html
```

The run used `rch` local fallback on `main @ 6b4181415c1e1a38c013b895cdca5f8ace522aaa`
with the current dirty profiling patch applied.

## Matrix Result

- Total scenarios: `93`
- FrankenSQLite faster / comparable / C SQLite faster: `78 / 3 / 12`
- Average F/C ratio: `0.5048263013032641`
- Geomean F/C ratio: `0.27571024757087464`
- Median F/C ratio: `0.2970987189148455`
- P90 F/C ratio: `1.0855404674241524`
- P99 F/C ratio: `3.326797385620915`
- Weighted score: `0.3792298779519334`

## Remaining C-SQLite-Faster Rows

| Ratio | Category | Section | Scenario | C SQLite | FrankenSQLite |
| ---: | --- | --- | --- | ---: | ---: |
| `3.3268x` | write_single | UPDATE/DELETE Throughput | 100 rows / delete 5 rows | `0.002295 ms` | `0.007635 ms` |
| `1.9020x` | write_single | UPDATE/DELETE Throughput | 1000 rows / delete 50 rows | `0.015829 ms` | `0.030107 ms` |
| `1.7007x` | write_single | UPDATE/DELETE Throughput | 10000 rows / delete 500 rows | `0.164608 ms` | `0.279944 ms` |
| `1.5239x` | write_single | UPDATE/DELETE Throughput | 100 rows / update 10 rows | `0.004188 ms` | `0.006382 ms` |
| `1.1713x` | write_bulk | INSERT Throughput - Record Size Comparison | large_10col - 10 cols | `9.168443 ms` | `10.738633 ms` |
| `1.1375x` | write_bulk | INSERT Throughput - Transaction Strategy Comparison | 100 rows / batched | `0.075952 ms` | `0.086392 ms` |
| `1.1138x` | write_bulk | INSERT Throughput - Single Transaction - small_3col | 100 rows | `0.077014 ms` | `0.085781 ms` |
| `1.1066x` | write_bulk | INSERT Throughput - Transaction Strategy Comparison | 100 rows / single txn | `0.076203 ms` | `0.084328 ms` |
| `1.1060x` | write_bulk | INSERT Throughput - Single Transaction - large_10col | 10000 rows | `9.069137 ms` | `10.030807 ms` |
| `1.0855x` | write_bulk | INSERT Throughput - Single Transaction - large_10col | 100 rows | `0.151554 ms` | `0.164518 ms` |
| `1.0651x` | concurrent_writers | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 4 writers x 1000 rows | `19.986544 ms` | `21.287799 ms` |
| `1.0502x` | concurrent_writers | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 2 writers x 1000 rows | `13.060018 ms` | `13.715084 ms` |

## Targeting Implication

The remaining weighted gap is concentrated in `write_single`, especially
prepared direct DELETE. The post-active-probe DML profile in
`tests/artifacts/perf/codex-dml-profile-after-active-probe-fix-20260517T1730Z/`
shows this is not a single helper problem: the cost is spread across retained
leaf-run probing, leaf materialization/flush, per-row maintenance, and fixed
BEGIN/COMMIT/dispatch overhead. The next source-level DELETE attempt should be
operator-scoped, not another leaf-search, materialization, synced-root, or
MemDatabase micro-patch.
