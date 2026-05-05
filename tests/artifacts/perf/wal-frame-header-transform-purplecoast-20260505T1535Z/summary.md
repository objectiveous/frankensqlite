# WAL Frame Header Transform Candidate

Date: 2026-05-05
Agent: PurpleCoast

## Scope

Measured the dirty `crates/fsqlite-wal/src/checksum.rs` candidate that replaced
`WalChecksumTransform::from_aligned_bytes(&frame[..8], ...)` in
`WalChecksumTransform::for_wal_frame` with a direct one-chunk affine transform
for the 8-byte WAL frame header.

CyanGorge held exclusive reservations on `checksum.rs` and
`docs/progress/perf-negative-results.md`, so this artifact records evidence
only. I did not edit, stage, or commit the source file or ledger.

## Correctness

Passed:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-wal-header-transform-purplecoast-target \
  cargo test -p fsqlite-wal test_wal_checksum_transform_matches_frame_checksum -- --nocapture
```

Result: 1 matching checksum transform test passed.

## Benchmark

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-wal-header-transform-purplecoast-target/release-perf/comprehensive-bench \
  --quick --filter insert --no-html \
  --json-out tests/artifacts/perf/wal-frame-header-transform-purplecoast-20260505T1535Z/candidate-insert-report.json
```

Baseline artifact:
`tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`

## Aggregate Result

Rejected by the insert matrix.

| Metric | Baseline | Candidate |
|---|---:|---:|
| primary weighted insert score | 1.6991 | 1.8718 |
| average F/C ratio | 2.4610x | 2.5723x |
| geomean F/C ratio | 2.3623x | 2.4773x |
| write_bulk geomean | 2.5153x | 2.6131x |
| write_single geomean | 1.4908x | 1.6748x |

## Selected FSQLite Median Movement

| Row | Baseline F median | Candidate F median | Delta |
|---|---:|---:|---:|
| single txn large_10col 100 rows | 0.397 ms | 0.500 ms | +26.0% |
| batched small_3col 1000 rows | 0.806 ms | 0.935 ms | +16.0% |
| autocommit small_3col 100 rows | 0.204 ms | 0.236 ms | +15.7% |
| record-size tiny_1col 10K | 4.514 ms | 4.925 ms | +9.1% |
| record-size medium_6col 10K | 9.889 ms | 10.718 ms | +8.4% |
| record-size large_10col 10K | 37.056 ms | 37.670 ms | +1.7% |
| single txn large_10col 1000 rows | 2.069 ms | 1.975 ms | -4.6% |

The few improved FSQLite medians are small and mixed. The matrix-level result
worsens enough that this is not a keep.

## Files

- `candidate.diff`
- `status-before.txt`
- `baseline-insert-report.json`
- `baseline-insert-summary.json`
- `candidate-insert-report.json`
- `candidate-insert-stdout.txt`
- `candidate-insert-stderr.txt`
