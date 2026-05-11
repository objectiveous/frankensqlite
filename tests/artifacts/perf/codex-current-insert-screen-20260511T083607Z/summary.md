# Current INSERT Screen - 2026-05-11

Purpose: current-source focused INSERT refresh before picking another source
lever after the exact transaction-control fast path was rejected.

Source context from benchmark stdout:

- Git: `main @ 2f8d3b75af4daedbdcd3522c4e599f2694182749`.
- Build: `release-perf` with opt-level 3 and LTO.
- Command shape:
  `env FSQLITE_BENCH_PROFILE_INSERT=1 .../comprehensive-bench --quick --filter insert --no-html --json-out tests/artifacts/perf/codex-current-insert-screen-20260511T083607Z/insert-profile.json`.

Focused INSERT summary:

- Total scenarios: `25`.
- FSQLite faster / comparable / C SQLite faster: `17 / 1 / 7`.
- Average ratio: `0.8817942536`.
- Geomean ratio: `0.8540783298`.
- Primary weighted score: `0.8348973035`.

Top red rows:

| Scenario | FSQLite median ms | C SQLite median ms | F/C ratio |
| --- | ---: | ---: | ---: |
| tiny_1col 100 rows | 0.099016 | 0.066455 | 1.4899706568 |
| small_3col 100 rows / batched (100/txn) | 0.087304 | 0.074930 | 1.1651407981 |
| small_3col 100 rows | 0.085510 | 0.073698 | 1.1602757198 |
| large_10col 100 rows | 0.165360 | 0.145723 | 1.1347556666 |
| small_3col 100 rows / single txn | 0.085009 | 0.075001 | 1.1334382208 |
| large_10col 10000 rows | 10.645839 | 9.519309 | 1.1183415729 |
| medium_6col 100 rows | 0.108203 | 0.098946 | 1.0935560811 |
| large_10col record-size 10K rows | 9.568181 | 9.497689 | 1.0074220160 |

Profile note: current INSERT is mostly green. The remaining red rows are
tiny fixed-cost cases and a near-tie large-record row. The visible source
families for record/page-run construction are already fenced in the negative
ledger, so this screen did not justify a standalone INSERT patch.
