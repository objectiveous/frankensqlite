# SharedTxnPageIo page-size cache rejected - 2026-05-05

Agent: CyanGorge
Candidate commit: `16b1907d`
Disposition: rejected and reverted

## Target

After the pager `write_page_data` replacement landed, the remaining insert
profile still showed `SharedTxnPageIo::write_page_data` / `write_page_internal`
under retained rightmost-leaf appends for large INSERT rows.

The candidate cached `TransactionKind::page_size()` in a shared `Cell<usize>`
inside `SharedTxnPageIo`, refreshed it on `refill`, and used it in
`PageWriter::write_page` / `write_page_data` to avoid a hot-path `RefCell`
borrow before normalization.

## Correctness Smoke

Passed before measurement:

```bash
cargo fmt --check
env CARGO_TARGET_DIR=.rch-target cargo check -p fsqlite-vdbe -p fsqlite-core
env CARGO_TARGET_DIR=.rch-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

## Measurement

Baseline/current:

- `tests/artifacts/perf/insert-large-current-cyangorge-20260505T221825Z/report.json`
- `tests/artifacts/perf/insert-large-current-cyangorge-20260505T221825Z/stderr-insert.log`

Candidate:

- `tests/artifacts/perf/shared-txn-page-size-cache-cyangorge-20260505T2230Z/report.json`
- `tests/artifacts/perf/shared-txn-page-size-cache-cyangorge-20260505T2230Z/stderr.log`

## Result

Rejected. The insert-only matrix moved the wrong way:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| weighted score | `1.6759` | `1.6944` |
| average ratio | `2.3289x` | `2.3467x` |
| geomean ratio | `2.2431x` | `2.2459x` |
| p99 ratio | `3.7410x` | `3.9032x` |
| write-single geomean | `1.4928x` | `1.5151x` |

The target large rows did not improve. The single-transaction `large_10col`
profile had commit roundtrip worsen from about `16.27 ms` to `17.06 ms`. The
record-size `large_10col` profile had B-tree insert worsen from about `7.53 ms`
to `8.48 ms`, with commit roundtrip still about `17.05 ms`.

Do not retry this page-size cache as a standalone optimization. The borrow is
easy to see in code, but the benchmark matrix says it is not the bottleneck.
