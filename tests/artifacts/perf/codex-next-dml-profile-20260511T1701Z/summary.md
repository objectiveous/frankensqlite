# Current DML Frontier Profile

Date: 2026-05-11

## Purpose

Refresh the DELETE/update-delete profile after
`786adc9469fac0e299dfd16b24a776174da4de44` so the next perf lever starts from
the current retained same-leaf delete-run boundary instead of the older
memory-direct page-I/O screen.

## Build

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/codex-next-dml-profile-target CARGO_BUILD_JOBS=8 \
  cargo build --profile release-perf -p fsqlite-e2e \
  --bin comprehensive-bench --bin perf-update-delete
```

The build completed on worker `ts2`; RCH retrieved
`/data/tmp/codex-next-dml-profile-target/release-perf/comprehensive-bench` and
`/data/tmp/codex-next-dml-profile-target/release-perf/perf-update-delete`.

## Comprehensive DML Profile

Command:

```bash
FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/codex-next-dml-profile-target/release-perf/comprehensive-bench \
  --quick --filter update \
  --json-out tests/artifacts/perf/codex-next-dml-profile-20260511T1701Z/update-delete-profile.json \
  --no-html
```

Artifact files:

- `update-delete-profile.json`
- `update-delete-profile.stdout`
- `update-delete-profile.stderr`

Results with profiling enabled:

| Scenario | C SQLite | FrankenSQLite | F/C |
|---|---:|---:|---:|
| 100 rows / update 10 rows | 4.3 us | 6.1 us | 1.40x |
| 100 rows / delete 5 rows | 2.2 us | 7.2 us | 3.19x |
| 1000 rows / update 100 rows | 36.5 us | 28.3 us | 0.78x |
| 1000 rows / delete 50 rows | 15.8 us | 29.0 us | 1.83x |
| 10000 rows / update 1000 rows | 361.5 us | 246.3 us | 0.68x |
| 10000 rows / delete 500 rows | 158.1 us | 257.3 us | 1.63x |

Profile attribution for DELETE:

| Scenario | Direct deletes | Active hits/attempts | Active misses | Flushes | Materialize time | Write time |
|---|---:|---:|---:|---:|---:|---:|
| 100 rows / delete 5 rows | 5 | 4/4 | 0 | 1/1 | 1.2 us | 0.2 us |
| 1000 rows / delete 50 rows | 50 | 44/49 | 5 | 6/6 | 9.7 us | 0.8 us |
| 10000 rows / delete 500 rows | 500 | 433/496 | 63 | 64/64 | 73.5 us | 7.5 us |

Every profiled DELETE stayed on the prepared direct path (`slow=0`).

## Focused Standard DELETE Probe

Commands:

```bash
/data/tmp/codex-next-dml-profile-target/release-perf/perf-update-delete 100 300 delete compare standard
/data/tmp/codex-next-dml-profile-target/release-perf/perf-update-delete 1000 300 delete compare standard
/data/tmp/codex-next-dml-profile-target/release-perf/perf-update-delete 10000 100 delete compare standard
```

Artifacts:

- `perf-delete-100-standard.txt`
- `perf-delete-1000-standard.txt`
- `perf-delete-10000-standard.txt`

Focused standard per-delete results:

| Rows / deletes | FrankenSQLite | C SQLite | F/C |
|---|---:|---:|---:|
| 100 / 5 | 1311 ns | 414 ns | 3.17x |
| 1000 / 50 | 569 ns | 318 ns | 1.79x |
| 10000 / 500 | 508 ns | 330 ns | 1.54x |

## Perf Sampling

`perf stat` for `10000 500 delete fsqlite standard` reported `516 ns` per
delete and is stored in `perf-stat-delete-10000-fsqlite.txt`.

`perf record` text reports are included for standard, isolated, and
rollback-isolated probes. The standard sample includes populate work by design,
and the rollback-isolated sample is dominated by rollback-time MemDatabase
rehydration, so the comprehensive DML counters remain the better source for the
standard matrix DELETE attribution.

## Conclusion

No source patch was attempted in this pass. The live evidence still matches the
ledgered boundary in `docs/progress/perf-negative-results.md`: standalone
same-leaf DELETE admission, materializer, direct-flush wrapper, and page-boundary
hints should not be retried. The next plausible source lever remains a broader
transaction-local DML mutation primitive that removes per-leaf mutation and
publication ceremony while proving read-your-writes, rollback/savepoint,
duplicate/missing-rowid, schema drift, QF/cache invalidation, and MVCC
publication semantics.
