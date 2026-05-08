# Concurrent writer retry harness proof - 2026-05-08

## Candidate

- Base at measurement time: `e305a172 docs(perf): record rejected REAL update leaf patch`
- Code under test: `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`
- Change shape: replace fixed `64 * 1ms` transaction retry sleeps in the concurrent-writer benchmark with bounded exponential busy backoff, deterministic per-thread jitter, and a 128-attempt transaction retry cap.
- Purpose: prevent the benchmark harness from herding 8 concurrent FSQLite writers into repeated snapshot conflicts while preserving concurrent-writer mode.

## Verification commands

```bash
rustfmt --check crates/fsqlite-e2e/src/bin/comprehensive_bench.rs
git diff --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-concurrent-retry-target cargo test -p fsqlite-e2e --bin comprehensive-bench busy_backoff_delay_caps_and_jitters_deterministically -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-concurrent-retry-check-target CARGO_BUILD_JOBS=12 cargo test -p fsqlite-e2e --bin comprehensive-bench busy_backoff_delay_caps_and_jitters_deterministically -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-concurrent-retry-check-target CARGO_BUILD_JOBS=12 cargo check --workspace --all-targets
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-concurrent-retry-check-target CARGO_BUILD_JOBS=12 cargo clippy --workspace --all-targets -- -D warnings
ubs crates/fsqlite-e2e/src/bin/comprehensive_bench.rs
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-concurrent-retry-bench-target CARGO_BUILD_JOBS=12 cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
/data/tmp/frankensqlite-silveranchor-concurrent-retry-bench-target/release-perf/comprehensive-bench --quick --filter concurrent --json-out tests/artifacts/perf/silveranchor-concurrent-retry-20260508T1346Z/concurrent-quick.json --no-html
/data/tmp/frankensqlite-silveranchor-concurrent-retry-bench-target/release-perf/comprehensive-bench --quick --json-out tests/artifacts/perf/silveranchor-concurrent-retry-20260508T1346Z/full-quick.json --no-html
```

Note: the RCH unit-test command printed `1 passed`; the helper was terminated after the successful result because artifact retrieval stayed open. UBS still exits nonzero on pre-existing benchmark-harness inventories; its own fmt, clippy, check, and test-build substeps were clean.

## Results

The focused concurrent quick run completed without the prior panic:

| Scenario | C SQLite median ms | FSQLite median ms | F/C ratio |
| --- | ---: | ---: | ---: |
| 2 writers x 1000 rows | 13.188514 | 12.567592 | 0.952919 |
| 4 writers x 1000 rows | 20.294355 | 18.312613 | 0.902350 |
| 8 writers x 1000 rows | 92.039900 | 40.407552 | 0.439022 |

The full quick matrix also completed without the prior 8-writer snapshot-conflict panic:

- Sections: 14
- Rows: 96
- Concurrent 8-writer row: C SQLite `90.578202 ms`, FSQLite `38.526939 ms`, ratio `0.425344`

## Artifacts

- `concurrent-quick.json`
- `concurrent-quick.stdout`
- `concurrent-quick.stderr`
- `full-quick.json`
- `full-quick.stdout`
- `full-quick.stderr`
