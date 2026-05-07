# UPDATE/DELETE next-gap profile

- Agent: CrimsonGorge
- Date: 2026-05-07
- Source: `main @ dd253b94` after the prepared direct-insert change-tracking fix
- Target section: `UPDATE/DELETEThroughput`

## Commands

```bash
env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-perf-target CARGO_BUILD_JOBS=16 cargo build -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete --profile release-perf
FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-update-delete-perf-target/release-perf/comprehensive-bench --quick --filter update --json-out tests/artifacts/perf/update-delete-gap-crimsongorge-20260507T1555Z/update-delete-profile.json --no-html
/data/tmp/frankensqlite-update-delete-perf-target/release-perf/perf-update-delete 100 200 both compare isolated
/data/tmp/frankensqlite-update-delete-perf-target/release-perf/perf-update-delete 100 200 both compare standard
```

## Matrix

`update-delete-profile.json` produced six rows:

- `100 rows / update 10 rows`: ratio `1.277491`
- `100 rows / delete 5 rows`: ratio `1.451292`
- `1000 rows / update 100 rows`: ratio `1.073694`
- `1000 rows / delete 50 rows`: ratio `1.237866`
- `10000 rows / update 1000 rows`: ratio `1.033423`
- `10000 rows / delete 500 rows`: ratio `1.048116`

Section weighted/geomean score: `1.177766`.

## Profile Read

The small rows look like the largest ratios, but the FSQLite mutation slice is not the main absolute cost in the full benchmark closure:

- `fs_delete_100`: `setup_us=56.8`, `prepare_us=17.2`, `mutate_us=8.8`, `commit_us=5.5`
- `fs_update_100`: `setup_us=60.3`, `prepare_us=21.1`, `mutate_us=12.7`, `commit_us=5.9`
- All rows used the direct path (`direct_update`/`direct_delete` equals mutation count, `fast` equals mutation count, `vdbe_opcodes=0`).

The isolated profiler still shows direct DML is slower per row than C SQLite:

- Isolated `100` rows, `200` iterations: update `799 ns` vs C `323 ns`; delete `1124 ns` vs C `258 ns`
- Standard `100` rows, `200` iterations: update `1465 ns` vs C `428 ns`; delete `1941 ns` vs C `429 ns`

## Interpretation

Do not chase another local fixed-width UPDATE or simple DELETE micro-hint here. The negative ledger already fences several standalone direct-DML seek/payload/hint ideas, and this profile shows that the benchmark rows mix fixed setup/populate cost with a still-real direct-DML per-row gap. The next plausible step-change candidate is not a one-row hint; it is a retained-cursor or batch direct-DML kernel that keeps one positioned B-tree cursor across monotone rowid UPDATE/DELETE runs and proves both isolated DML and the full `UPDATE/DELETEThroughput` section move together.
