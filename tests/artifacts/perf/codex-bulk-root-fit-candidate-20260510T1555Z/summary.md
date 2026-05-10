# Empty-root bulk-fit probe candidate

Date: 2026-05-10

Candidate:

- Touched during candidate, then reverted:
  `crates/fsqlite-btree/src/cursor.rs`.
- Shape: replace the initial `table_bulk_load_empty_root_sorted_records`
  root-page grouping pass with an early-exit "does this run fit one page?"
  probe. The intended win was avoiding a full discarded grouping scan before
  the real leaf grouping pass for multi-page empty-root bulk loads.

Correctness proof:

- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-bulk-fit-target CARGO_BUILD_JOBS=4 cargo test -p fsqlite-btree table_bulk -- --nocapture --test-threads=1`
  passed the two matching bulk-load / bulk-append tests. The remote command
  completed with exit 0; the local RCH wrapper was stopped after it hung during
  target-directory retrieval.
- `cargo fmt -p fsqlite-btree --check` passed.

Same-window focused INSERT A/B:

- Baseline: `baseline-insert-quick.json` and `baseline-insert-quick.err`.
- Candidate: `candidate-insert-quick.json` and `candidate-insert-quick.err`.
- Command shape:
  `env FSQLITE_BENCH_PROFILE_INSERT=1 <release-perf comprehensive-bench> --quick --filter insert --json-out <artifact> --no-html`.

Result:

- Rejected and source left reverted.
- Focused INSERT weighted score improved from `0.829507599` to `0.821400045`,
  and geomean improved from `0.836925529` to `0.823158003`, but the row mix
  worsened from `17 / 1 / 7` to `17 / 0 / 8`.
- The large 10K rows were mixed:
  `Single Transaction / large_10col / 10000 rows` regressed from
  `10.842706 ms` FSQLite (`1.106135808x`) to `10.971507 ms`
  (`1.167785863x`), while `Record Size / large_10col` improved from
  `11.436718 ms` (`1.204984420x`) to `10.609499 ms` (`1.098141311x`).
- Profile counters were also mixed: candidate `direct_flush_ns` improved on
  `record_size_large_10col_10000` (`3.059757 ms` to `2.940555 ms`) but
  worsened on `single_txn_large_10col_10000` (`2.836290 ms` to
  `3.207855 ms`).

Conclusion:

Do not keep or retry this early-exit root-fit probe as a standalone
optimization. It is logically valid but does not produce a robust focused INSERT
win on the large-row frontier. Reconsider only inside a larger fused
record/page-builder design that improves both large 10K rows and the full quick
primary score in the same A/B window.
