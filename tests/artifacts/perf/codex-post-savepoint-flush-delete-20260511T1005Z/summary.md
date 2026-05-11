# Post-savepoint-flush UPDATE/DELETE screen - 2026-05-11

Purpose: measure the current UPDATE/DELETE tail after
`930aff5f4d6ae948408cace7b8fa8469176ff51c`
(`fix(core): flush direct runs before statement savepoints`), and check that the
statement-savepoint correctness fix did not hide a new DML performance
regression.

Command:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-bench-target CARGO_BUILD_JOBS=8 \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter delete \
  --json-out tests/artifacts/perf/codex-post-savepoint-flush-delete-20260511T1005Z/update-delete-quick.json \
  --no-html

env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-codex-bench-target/release-perf/comprehensive-bench \
  --quick --filter delete \
  --json-out tests/artifacts/perf/codex-post-savepoint-flush-delete-20260511T1005Z/update-delete-profile-quick.json \
  --no-html \
  > tests/artifacts/perf/codex-post-savepoint-flush-delete-20260511T1005Z/update-delete-profile-stdout.txt \
  2> tests/artifacts/perf/codex-post-savepoint-flush-delete-20260511T1005Z/update-delete-profile-stderr.txt
```

Environment:

- OS/kernel: Ubuntu 25.10, Linux 6.17.0-19-generic.
- CPU: AMD Ryzen Threadripper PRO 5995WX 64-Cores, 128 cores.
- Toolchain: `nightly-x86_64-unknown-linux-gnu`, rustc
  `1.97.0-nightly (4b0c9d76a 2026-05-10)`.
- Build: `release-perf`.
- Git: `main @ 930aff5f4d6ae948408cace7b8fa8469176ff51c`.
- Artifact note: the JSON reports record `git_dirty: true` because the ignored
  artifact files were being written during the run.

Focused ratios from `update-delete-quick.json`:

| Scenario | C SQLite | FrankenSQLite | F/C |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 8.846 us | 9.338 us | 1.056x |
| 100 rows / delete 5 rows | 4.108 us | 8.917 us | 2.171x |
| 1000 rows / update 100 rows | 39.083 us | 42.099 us | 1.077x |
| 1000 rows / delete 50 rows | 15.219 us | 34.534 us | 2.269x |
| 10000 rows / update 1000 rows | 399.898 us | 337.051 us | 0.843x |
| 10000 rows / delete 500 rows | 240.631 us | 343.362 us | 1.427x |

Profile-enabled rerun ratios from `update-delete-profile-quick.json`:

| Scenario | C SQLite | FrankenSQLite | F/C |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 4.058 us | 7.444 us | 1.834x |
| 100 rows / delete 5 rows | 2.255 us | 7.985 us | 3.541x |
| 1000 rows / update 100 rows | 35.617 us | 31.699 us | 0.890x |
| 1000 rows / delete 50 rows | 15.208 us | 32.891 us | 2.163x |
| 10000 rows / update 1000 rows | 348.893 us | 275.495 us | 0.790x |
| 10000 rows / delete 500 rows | 155.621 us | 295.043 us | 1.896x |

Hot-path counters for the profile-enabled 10K/500 DELETE case:

- `mutate_us=307.4`, `commit_us=66.9`, `direct_delete=500`.
- `delete_leaf_start=64/67`, `delete_leaf_start_ns=12347`.
- `delete_leaf_active=433/496`, `delete_leaf_active_ns=51751`.
- `delete_leaf_miss=63`, mostly `delete_leaf_miss_out_of_leaf=60`.
- `delete_leaf_flush=64/64`, `delete_leaf_flush_ns=108754`.
- `delete_leaf_materialize=64/77583`, `delete_leaf_write=64/24059`.
- `delete_seek_ns=35698`, `delete_physical_ns=11973`.
- `pager_mem_flush_ns=29546`, `pager_cache_finish_ns=12253`.
- `background_status`: `bg_checks=504`, `bg_ns=13910`.

Interpretation:

- The savepoint-boundary correctness fix did not change the destination
  conclusion: the remaining DELETE gap is still a representation/batching
  problem, not parser, background-status, direct-flush-wrapper, or another
  retained-run micro-patch problem.
- The measured profile points at the broader `bd-db300.11.1` transaction-local
  DML mutation operator: buffer logical rowid mutations, serve read-your-writes
  from the buffer, then flush page-local batches once through the normal MVCC
  publication path.
- Do not use this artifact to revive standalone `TableLeafDeleteRun`,
  background-status, or direct-writer-publication tweaks; those are already
  fenced in `docs/progress/perf-negative-results.md`.

SHA256:

```text
f3bc2350a3cd8ce24db9da712f5703ac56e11b0468af441e3731d8de7edc7780  update-delete-quick.json
ce22c941cc7b5125d5f63a411331a86dc1fa673d06a2b6f4f98e6dbdbae614ca  update-delete-profile-quick.json
6b01b830d75bda275d8a810670e970b3bf38d04753faeac3af44dcd2676508bf  update-delete-profile-stdout.txt
d6835aed57a0b70320056181f516d3a76e2e294c3cdb51c02338e2db2bfc24ce  update-delete-profile-stderr.txt
```
