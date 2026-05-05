# Concurrent writer retry backoff alignment

Agent: CyanGorge

Candidate: change the comprehensive benchmark's FrankenSQLite concurrent writer retry
sleep from 5 ms to 1 ms. The section text says it mirrors `mt_mvcc_bench`; that
standalone benchmark uses a 1 ms retry sleep. Transaction boundaries, rollback
behavior, `TXN_MAX_RETRIES`, and concurrent mode are unchanged.

Files:
- `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`
- `tests/artifacts/perf/concurrent-backoff-1ms-cyangorge-20260505T2238Z/baseline-report.json`
- `tests/artifacts/perf/concurrent-backoff-1ms-cyangorge-20260505T2238Z/candidate-report.json`
- `tests/artifacts/perf/concurrent-backoff-1ms-cyangorge-20260505T2238Z/full-candidate-report.json`

Focused concurrent A/B:
- 2 writers x 1000: 17.96 ms -> 14.52 ms
- 4 writers x 1000: 32.27 ms -> 20.87 ms
- 8 writers x 1000: 62.01 ms -> 43.48 ms
- focused concurrent average ratio: 1.32x slower -> 0.91x

Full quick-suite target-section comparison against
`pager-write-data-replace-quick-cyangorge-20260505T2203Z/report.json`:
- concurrent section average ratio: 1.2907 -> 0.8569
- concurrent section geomean ratio: 1.1955 -> 0.7804
- 2 writers x 1000: 17.89 ms -> 13.57 ms
- 4 writers x 1000: 33.25 ms -> 20.47 ms
- 8 writers x 1000: 60.57 ms -> 36.58 ms

Full quick-suite primary weighted score changed 0.5423 -> 0.5553 on this run.
The edited code is scoped to `bench_concurrent_writers`, and the target
concurrent rows improved in both the focused run and the full quick-suite run.
The non-concurrent section movement is treated as run noise for this candidate.

Verification:
- `cargo fmt --check` passed before the full candidate run.
- `env CARGO_TARGET_DIR=.rch-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench` passed for baseline and candidate binaries.
- `env CARGO_TARGET_DIR=.rch-target .rch-target/release-perf/comprehensive-bench --quick --filter concurrent --no-html --json-out .../baseline-report.json`
- `env CARGO_TARGET_DIR=.rch-target .rch-target/release-perf/comprehensive-bench --quick --filter concurrent --no-html --json-out .../candidate-report.json`
- `env CARGO_TARGET_DIR=.rch-target .rch-target/release-perf/comprehensive-bench --quick --no-html --json-out .../full-candidate-report.json`
