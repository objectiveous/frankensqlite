# Current Full-Quick Matrix

Date: 2026-05-12

Command:

```bash
/tmp/frankensqlite-codex-current-head-target/release-perf/comprehensive-bench \
  --quick \
  --json-out tests/artifacts/perf/codex-current-head-fullquick-20260512T0142/full.json \
  --no-html \
  2>&1 | tee tests/artifacts/perf/codex-current-head-fullquick-20260512T0142/stdout.txt
```

Benchmark binary build:

```bash
rch exec -- env CARGO_TARGET_DIR=/tmp/frankensqlite-codex-current-head-target \
  CARGO_BUILD_JOBS=2 \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

Measured commit: `366a08e8271c8f5858808ef55f03034159d9c9ad`

The benchmark reported `Git dirty: yes` because this artifact directory was
being written during the run. No Rust source files were dirty.

## Headline

- Total scenarios: `93`
- FrankenSQLite faster / comparable / C SQLite faster: `81 / 3 / 9`
- Average F/C time ratio: `0.4894146804579474`
- Geomean F/C time ratio: `0.27122418232778245`
- Median F/C time ratio: `0.30825382748542196`
- p90 F/C time ratio: `1.049173092400758`
- p99 F/C time ratio: `2.895644283121597`
- Weighted primary score: `0.36876886282442267`

## Remaining Red Rows

Rows with F/C ratio above `1.0` in this run:

- `UPDATE/DELETEThroughput`, `100 rows / delete 5 rows`: `2.895644283121597`
- `UPDATE/DELETEThroughput`, `1000 rows / delete 50 rows`: `1.7695837848173879`
- `UPDATE/DELETEThroughput`, `10000 rows / delete 500 rows`: `1.5776671252389243`
- `UPDATE/DELETEThroughput`, `100 rows / update 10 rows`: `1.3649356836588853`
- `INSERTThroughput - Single Transaction - small_3col`, `100 rows`: `1.135529533933622`
- `Concurrent Writers`, `2 writers x 1000 rows`: `1.1084674743930842`
- `INSERTThroughput - Single Transaction - large_10col`, `100 rows`: `1.1015091528603551`
- `INSERTThroughput - Single Transaction - tiny_1col`, `100 rows`: `1.0767199875757105`
- `INSERTThroughput - Record Size Comparison`, `large_10col`: `1.0593438630458043`
- `INSERTThroughput - Transaction Strategy`, `100 rows / single txn`: `1.049173092400758`
- `INSERTThroughput - Single Transaction - large_10col`, `10000 rows`: `1.0426484629796895`
- `INSERTThroughput - Single Transaction - medium_6col`, `100 rows`: `1.0356820416106574`

Interpretation: the unfixed tail is still the already-fenced prepared-DML
DELETE family. The non-DML rows are near parity and map to the already-recorded
fixed-cost INSERT and low-thread concurrent boundaries.
