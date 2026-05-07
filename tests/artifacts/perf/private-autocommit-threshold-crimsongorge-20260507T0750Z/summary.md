# Private Memory Autocommit Threshold Candidate

Candidate: change `Connection::adaptive_flush_threshold()` so pure-write
private `:memory:` retained autocommit batches flush after 1024 statements
instead of the existing 256. File-backed pure writes and mixed read/write
workloads were left unchanged.

## Proof Before Measurement

- `cargo fmt -p fsqlite-core --check` passed.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-autocommit-threshold-target cargo test -p fsqlite-core retained_autocommit_adaptive_flush -- --nocapture`
  passed the three retained-autocommit adaptive-threshold tests, including a
  temporary memory-specific assertion for the candidate.
- Candidate release-perf build passed:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-crimsongorge-autocommit-threshold-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`.

## Same-Window Transaction-Section A/B

Baseline binary:
`/data/tmp/frankensqlite-purpleotter-lockshards64-perf-target/release-perf/comprehensive-bench`.

Candidate binary:
`/data/tmp/frankensqlite-crimsongorge-autocommit-threshold-target/release-perf/comprehensive-bench`.

Command shape:

```bash
comprehensive-bench --quick --filter transaction --json-out <out.json> --no-html
```

| Scenario | Baseline F median | Candidate F median | Direction |
| --- | ---: | ---: | --- |
| 100 rows / autocommit | 0.147657 ms | 0.153147 ms | worse |
| 100 rows / batched (100/txn) | 0.092022 ms | 0.117490 ms | worse |
| 100 rows / single txn | 0.090439 ms | 0.095198 ms | worse |
| 1000 rows / autocommit | 1.129896 ms | 1.189378 ms | worse |
| 1000 rows / batched (1000/txn) | 0.306504 ms | 0.310221 ms | worse |
| 1000 rows / single txn | 0.312575 ms | 0.311613 ms | neutral |
| 10000 rows / autocommit | 11.044968 ms | 11.279396 ms | worse |
| 10000 rows / batched (1000/txn) | 4.501872 ms | 4.481204 ms | slight win |
| 10000 rows / single txn | 2.881837 ms | 3.039603 ms | worse |

The 10K autocommit ratio improved (`1.345x -> 1.263x`) only because the
C SQLite median was slower in the candidate run; the absolute FrankenSQLite
median regressed. The source change was reverted before commit.

Disposition: rejected. Do not retry a simple private-memory pure-write retained
autocommit threshold increase. Revisit only if a phase profile proves retained
flush boundaries dominate the actual autocommit row and a same-window A/B
improves absolute FrankenSQLite medians, not just ratios.
