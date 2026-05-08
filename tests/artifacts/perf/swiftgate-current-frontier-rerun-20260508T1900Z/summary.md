# Current frontier rerun - 2026-05-08

## Scope

Fresh clean-source reproduction after `7a89c73b`
(`docs(perf): record page-size pragma rejection`). No Rust source changed since
the clean benchmark binary used for this pass; commits after `2bd0717a` are
docs/perf artifacts only.

The purpose was to re-apply the profiling-first gate before selecting another
source lever. Recent artifacts showed the remaining gaps were high-variance and
clustered in already-fenced standalone optimization families.

## Commands

```bash
env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-page-size-skip-clean-target/release-perf/comprehensive-bench \
  --quick --filter update \
  --json-out tests/artifacts/perf/swiftgate-current-frontier-rerun-20260508T1900Z/dml-profile.json \
  --html tests/artifacts/perf/swiftgate-current-frontier-rerun-20260508T1900Z/dml-profile.html

env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-page-size-skip-clean-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/swiftgate-current-frontier-rerun-20260508T1900Z/insert-profile.json \
  --html tests/artifacts/perf/swiftgate-current-frontier-rerun-20260508T1900Z/insert-profile.html

/data/tmp/frankensqlite-page-size-skip-clean-target/release-perf/comprehensive-bench \
  --quick \
  --json-out tests/artifacts/perf/swiftgate-current-frontier-rerun-20260508T1900Z/full-quick.json \
  --html tests/artifacts/perf/swiftgate-current-frontier-rerun-20260508T1900Z/full-quick.html
```

Stdout/stderr for each command is under `stdout/`.

## Focused gates

| Gate | Avg | Geomean | Weighted | P90 | P99 | Faster / Comparable / C faster |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| DML | 1.0538138111 | 1.0429015795 | 1.0429015795 | 1.2767802682 | 1.2767802682 | 1 / 2 / 3 |
| INSERT | 0.8159075810 | 0.7926860280 | 0.7873845281 | 1.1165621485 | 1.1666214861 | 17 / 3 / 5 |

Focused INSERT did not reproduce a large-record 10K source target. The slower
INSERT rows were 100-row fixed-cost cases:

| Row | C SQLite ms | FrankenSQLite ms | F/C |
| --- | ---: | ---: | ---: |
| tiny_1col 100 rows | 0.064901 | 0.072466 | 1.1166 |
| small_3col 100 rows | 0.076082 | 0.085651 | 1.1258 |
| medium_6col 100 rows | 0.100198 | 0.108152 | 1.0794 |
| small_3col 100 rows batched | 0.073778 | 0.086071 | 1.1666 |
| small_3col 100 rows single txn | 0.075751 | 0.082123 | 1.0841 |

Focused DML kept the 100-row rows slow and showed a noisy 10K update tail:

| Row | C SQLite ms | FrankenSQLite ms | F/C |
| --- | ---: | ---: | ---: |
| update 10/100 | 0.108844 | 0.125685 | 1.1547 |
| delete 5/100 | 0.090169 | 0.115126 | 1.2768 |
| update 100/1000 | 0.387946 | 0.397354 | 1.0243 |
| update 1000/10000 | 3.675071 | 3.937312 | 1.0714 |

100-row DML counters again show setup/prepare/transaction ceremony dominates
over the actual direct mutation:

| Row | setup_us | begin_us | prepare_us | mutate_us | commit_us |
| --- | ---: | ---: | ---: | ---: | ---: |
| update 10/100 | 56.9 | 6.8 | 14.5 | 13.2 | 6.3 |
| delete 5/100 | 55.2 | 5.2 | 12.1 | 9.4 | 6.0 |

## Full quick

- Scenarios: `93`
- Faster / comparable / slower: `82 / 4 / 7`
- Average ratio: `0.4505594084`
- Geomean ratio: `0.2643439387`
- Weighted score: `0.3400136964`
- P90 / P99 ratio: `0.9804477882 / 1.8367705191`

Full quick C-faster rows:

| Row | C SQLite ms | FrankenSQLite ms | F/C | Notes |
| --- | ---: | ---: | ---: | --- |
| small_3col 100 rows insert | 0.076904 | 0.141255 | 1.8368 | high variance: C CV 24.1%, F CV 28.1% |
| delete 5/100 | 0.082996 | 0.111178 | 1.3396 | stable DML tail |
| update 10/100 | 0.086913 | 0.114454 | 1.3169 | stable DML tail |
| large_10col 100 rows insert | 0.152696 | 0.183022 | 1.1986 | fixed-cost tail, F CV 14.5% |
| small_3col 100 rows batched | 0.076283 | 0.087494 | 1.1470 | fixed-cost tail |
| small_3col 100 rows single txn | 0.077705 | 0.085530 | 1.1007 | fixed-cost tail |
| large_10col 10K record-size | 9.728625 | 10.381528 | 1.0671 | mild; focused record profile was high-variance |

## Source decision

No source patch was attempted.

The current rerun does not expose a high-confidence standalone optimization:

- Large-record 10K is not repeatedly slow in focused INSERT.
- 100-row INSERT and DML tails are shared fixed-cost/setup cases, but the
  obvious transaction-control, PRAGMA, sqlite_master setup, schema lookup,
  root predecode, statement microbatch, and direct-DML ceremony trims are
  already rejected in `docs/progress/perf-negative-results.md`.
- Direct DML mutation itself remains small (`13.2 us` update, `9.4 us`
  delete), so a row-local patch would repeat the fenced payload/scratch/cursor
  candidates.

The next source attempt should either implement a real broader DML leaf-run
operator with read-after-write visibility proof, or wait for a new profile that
shows an unfenced top-5 hotspot. For the current evidence, the correct action is
to keep the frontier artifact and avoid another no-op micro-optimization.
