# Rejected WAL prepared-transform slice candidate

Date: 2026-05-05
Agent: CyanGorge
Baseline artifact:
`tests/artifacts/perf/insert-current-head-profile-cyangorge-20260505T2340Z/report.json`
Candidate artifacts:
`tests/artifacts/perf/wal-transform-slice-cyangorge-20260505T2350Z/report.json`
and
`tests/artifacts/perf/wal-transform-slice-cyangorge-20260505T2350Z/report-repeat.json`

## Candidate

Remove the redundant conversion in
`crates/fsqlite-core/src/wal_adapter.rs::finalize_prepared_batch_against_current_state`
that copied every prepared checksum transform into a fresh
`Vec<WalChecksumTransform>` before calling
`WalFile::finalize_prepared_frame_bytes`.

The idea was plausible because `PreparedWalChecksumTransform` is already a type
alias to the canonical WAL transform type. The source diff was reverted after
measurement.

## Correctness

Passed:

```bash
rch exec -- env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-core --lib append -- --nocapture
```

Result: `17` tests passed.

## INSERT matrix result

| Run | Weighted score | Avg ratio | Geomean ratio | Write-bulk geomean | Write-single geomean | P99 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | `1.7491` | `2.4316x` | `2.3183x` | `2.4461x` | `1.5640x` | `4.0792x` |
| candidate 1 | `1.6877` | `2.4194x` | `2.3170x` | `2.4612x` | `1.4882x` | `4.3231x` |
| candidate 2 | `1.7528` | `2.4468x` | `2.3677x` | `2.5072x` | `1.5556x` | `3.8639x` |

## Decision

Rejected. The first run improved the primary weighted score but regressed
write-bulk geomean and p99. The repeat run regressed the primary weighted score
relative to baseline (`1.7491 -> 1.7528`) and also regressed average/geomean and
write-bulk geomean. This is not stable enough to keep.

Do not retry removing the prepared checksum-transform copy as a standalone WAL
optimization unless a fresh profile proves the copy dominates and an
interleaved A/B improves both the weighted INSERT score and write-bulk geomean.
