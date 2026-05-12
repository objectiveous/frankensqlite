# Current Full-Quick Frontier After Shared-Table Retry Fix

- Date: 2026-05-12 12:05 UTC
- Commit: `70994b63a0060025d20347338ae992ac64d0cb75`
- Command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-0710 CARGO_BUILD_JOBS=4 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --no-html --json-out tests/artifacts/perf/codex-current-frontier-after-mtfix-20260512T1200Z/full-current.json`
- Validity: `git_dirty=false`, `benchmark_binary_older_than_git_head=false`.
- Evidence to use: `full-current.json`, `stdout-current.txt`, `stderr-current.txt`.
- Evidence to ignore: `full.json`, `stdout.txt`, and `stderr.txt` in this directory came from a direct prebuilt-binary probe. That run exited 0 but warned that the benchmark binary predated Git HEAD, so it is not decision evidence.

## Summary

The rebuilt quick matrix covers 93 scenarios:

| Metric | Value |
| --- | ---: |
| Franken faster / comparable / C faster | `79 / 4 / 10` |
| Average F/C | `0.494731010964893` |
| Geomean F/C | `0.2757405133769323` |
| Primary weighted score | `0.3741665361944529` |

Compared with `tests/artifacts/perf/codex-current-frontier-fullquick-20260512T0810Z/full.json` at `6d26e7d50c6e99137c7c451f6d1e03111fd1cacf`, the primary score changed from `0.37100042867198824` to `0.3741665361944529`, geomean from `0.27423740495932974` to `0.2757405133769323`, and average F/C from `0.4852381197057886` to `0.494731010964893`.

## Rows Above 1.03 F/C

| F/C | Prior F/C | Category | Scenario |
| ---: | ---: | --- | --- |
| `3.1293916023993145` | `2.969811320754717` | `write_single` | `UPDATE/DELETEThroughput`, `100 rows / delete 5 rows` |
| `1.9052005012531328` | `1.846849656893325` | `write_single` | `UPDATE/DELETEThroughput`, `1000 rows / delete 50 rows` |
| `1.6375796494185901` | `1.6260645002613214` | `write_single` | `UPDATE/DELETEThroughput`, `10000 rows / delete 500 rows` |
| `1.4628597957288765` | `1.4088152985074627` | `write_single` | `UPDATE/DELETEThroughput`, `100 rows / update 10 rows` |
| `1.3228377525252526` | `1.062464644432991` | `write_bulk` | `INSERTThroughput - Transaction Strategy Comparison (small_3col)`, `100 rows / single txn` |
| `1.120814543224702` | `1.0891429882595893` | `write_bulk` | `INSERTThroughput - Transaction Strategy Comparison (small_3col)`, `100 rows / batched (100/txn)` |
| `1.106762284053603` | `1.1006570707104535` | `write_bulk` | `INSERTThroughput - Single Transaction - large_10col`, `100 rows` |
| `1.0990955211642923` | `1.0291982419842562` | `write_bulk` | `INSERTThroughput - Single Transaction - small_3col`, `100 rows` |
| `1.06790114148331` | `1.087960816497609` | `concurrent_writers` | `Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC`, `2 writers x 1000 rows` |
| `1.0576530462728702` | `1.0379819903024705` | `write_bulk` | `INSERTThroughput - Single Transaction - medium_6col`, `100 rows` |

## Decision

No source patch is justified from this run. The remaining red rows match already-fenced families:

- Small DML retained-run / transaction-envelope rows: see `docs/progress/perf-negative-results.md` entries "Transaction-local DML mutation boundary", "Current small UPDATE transaction-envelope rescreen", and the retained DELETE negative-result series.
- Small fixed-cost INSERT rows: see "Current INSERT profile boundary refresh".
- Low-thread concurrent writer tail: see "Current concurrent writer boundary refresh".

The 2-writer concurrent row improved from `1.087960816497609` to `1.06790114148331`, and the 4-writer row moved below the 1.03 threshold (`0.9742996936844054`). The eligible next source frontier remains a broader transaction-local DML mutation/read-view operator, not another narrow leaf-run, insert serializer, or retry/backoff tweak.
