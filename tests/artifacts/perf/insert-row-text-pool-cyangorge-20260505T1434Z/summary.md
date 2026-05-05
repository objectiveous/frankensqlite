# Direct INSERT row text pool candidate

Run: `2026-05-05T14:34Z`

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/cargo-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/insert-row-text-pool-cyangorge-20260505T1434Z/report.json --no-html
```

Candidate:

- `crates/fsqlite-core/src/connection.rs` only, reverted after measurement.
- Returned heap-backed direct INSERT row-scratch values to the existing
  `fsqlite_types::value` TLS pool when the lazy `:memory:` insert path clears
  `mem_row_values`.
- Built concat-chain `SqliteValue::Text` results from a pooled `SmallText` slot
  when available, using `SmallText::overwrite`, so repeated large payload rows
  could reuse heap allocations.

Correctness/proof gates before benchmark:

- `cargo fmt --check`
- `cargo test -p fsqlite-core test_prepared_direct_simple_insert_returns_concat_text_to_value_pool -- --nocapture`
- `cargo test -p fsqlite-core test_prepared_direct_simple_insert_large_profile_breakdown -- --nocapture`
- `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`
- `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`

Comparison baseline:

- `tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`
  from source-equivalent current head before the docs-only no-retry commits.

Result:

- Rejected and reverted.
- Insert avg ratio improved `2.4610x -> 2.3595x` and geomean improved
  `2.3623x -> 2.2890x`, but the primary insert weighted score regressed
  `1.6991 -> 1.7329`.
- Write-single geomean regressed `1.4908x -> 1.5517x`.
- Important absolute FrankenSQLite medians regressed:
  - `small_3col` 1K single transaction: `0.8055 ms -> 0.9613 ms` (+19.3%).
  - `small_3col` 10K single transaction: `6.8949 ms -> 7.7481 ms` (+12.4%).
  - `medium_6col` 10K single transaction: `13.6661 ms -> 14.6216 ms` (+7.0%).
  - `large_10col` 10K single transaction: `36.1651 ms -> 36.7869 ms` (+1.7%).
  - record-size `large_10col` 10K: `37.0559 ms -> 37.6541 ms` (+1.6%).
- The profile rejected the root hypothesis on the target large rows: row-build
  time worsened instead of shrinking (`large_10col` single transaction
  `5.958 ms -> 7.404 ms`, record-size `large_10col` `5.973 ms -> 6.722 ms`),
  so the TLS pool traffic added more overhead than the saved heap allocation.

Disposition:

- Do not retry direct INSERT row-value pooling / `SmallText::overwrite` reuse as
  a standalone row-build optimization. It passed focused correctness but lost on
  the weighted insert score and the largest source-visible row-build counters.
