# Performance Negative Results Ledger

This ledger records performance ideas that were measured and rejected. Check it
before starting a new optimization pass, and add an entry whenever a candidate is
abandoned, reverted, or kept out of the tree because the benchmark matrix did not
move in the intended direction.

Each entry should include:
- Target workload rows or benchmark section.
- Files or subsystem touched.
- Baseline and candidate evidence.
- Result and reason for rejection.
- Conditions under which the idea is worth retrying.

## 2026-05-04 - Single-value insert serialization specialization

- Target: insert throughput, especially tiny/small single-column and small-record rows.
- Touched: `crates/fsqlite-types/src/record.rs`, `crates/fsqlite-vdbe/src/engine.rs`.
- Candidate commit: `7fa3f4d0 perf(record): specialize single-value insert serialization`.
- Revert commit: `5e9445ac Revert "perf(record): specialize single-value insert serialization"`.
- Evidence:
  - Baseline: `/data/tmp/frankensqlite-purplecoast-postcommit-parent-20260504T220353Z-report.json`.
  - Candidate: `/data/tmp/frankensqlite-purplecoast-postcommit-head-20260504T220353Z-report.json`.
- Result: rejected and reverted. Overall fsqlite geomean time changed by `1.0247x`
  slower, average time was `+3.89%`, with 11 improved rows and 14 regressed rows.
- Do not retry unless the exact insert section is benchmarked first and the
  implementation avoids adding overhead to multi-column insert rows.

## 2026-05-04 - Two-byte precomputed record header support

- Target: insert serialization for records whose serial types need two-byte varints.
- Touched: `crates/fsqlite-types/src/record.rs`, `crates/fsqlite-vdbe/src/engine.rs`.
- Candidate shape: add `PrecomputedSerialTypeKind::AnyTwoByteVarint` and patch
  precomputed record headers at runtime.
- Evidence:
  - Candidate: `/data/tmp/frankensqlite-purplecoast-two-byte-record-candidate-20260504T2218Z-report.json`.
  - Baseline: `/data/tmp/frankensqlite-purplecoast-postcommit-parent-20260504T220353Z-report.json`.
- Result: rejected before commit. Overall fsqlite geomean time changed by
  `1.1139x` slower, average time was `+13.97%`, with 6 improved rows and
  19 regressed rows.
- Do not retry as a general record-header optimization. Only reconsider if a
  profile proves two-byte serial type patching is isolated to a workload where
  the end-to-end matrix improves.

## 2026-05-04 - Prepared PK rowid last-result cache

- Target: `Read-After-Write Query Performance`, especially `point lookup (PK)`.
- Touched: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: one-entry version-scoped cache for repeated prepared primary
  key rowid lookups, sharing invalidation keys with existing prepared MemDB
  caches.
- Evidence:
  - Full matrix that motivated the target: `/data/tmp/frankensqlite-purplecoast-current-full-20260504T2230Z-report.json`.
  - Candidate read section: `/data/tmp/frankensqlite-purplecoast-rowid-cache-candidate-read-20260504T2245Z-report.json`.
  - Close baseline read section: `/data/tmp/frankensqlite-purplecoast-rowid-cache-baseline-read-20260504T2252Z-report.json`.
  - Saved rejected patch: `/data/tmp/frankensqlite-purplecoast-rowid-cache-20260504T2252Z.patch`.
- Result: rejected before commit. The targeted correctness test passed, but the
  close A/B read geomean regressed from `2.41x` to `3.15x` versus C SQLite.
  PK fsqlite-time rows also regressed: `100 rows` by `1.15x`, `1000 rows` by
  `1.43x`, and `10000 rows` by `2.26x`.
- Do not retry the same one-entry rowid result cache. Reconsider only if the
  query-row dispatch path is redesigned so the cache removes more work than it
  adds, and prove it with a close A/B read-section run.

## 2026-05-04 - Standard-library ASCII LIKE byte comparison

- Target: string workload rows, especially LIKE prefix/contains/wildcard scans.
- Touched: `crates/fsqlite-types/src/value.rs`.
- Candidate shape: replace the local ASCII-case byte comparison helper with
  `[u8]::eq_ignore_ascii_case`.
- Evidence:
  - Baseline: `tests/artifacts/perf/string-clean-head-cyangorge-20260504T2240Z/report.json`.
  - Candidate: `tests/artifacts/perf/string-std-ascii-ci-cyangorge-20260504T2246Z/report.json`.
- Result: rejected before commit. Average string-section ratio worsened from
  about `3.03x` to `3.73x`; 100-row and 10K-row prefix/wildcard rows regressed,
  with only the 1K-row prefix case improving.
- Do not retry as a general LIKE matcher cleanup. Reconsider only with an
  end-to-end string-section A/B that shows row-level wins beyond noise.

## 2026-05-04 - Exact-sized record body writes

- Target: record-size insert section, especially `large_10col`.
- Touched: `crates/fsqlite-types/src/record.rs`.
- Candidate shape: pre-size the serialized record buffer to the full record size
  and write payload bytes into exact slices instead of appending payload bytes.
- Evidence:
  - Baseline: `tests/artifacts/perf/record-current-clean-cyangorge-20260504T2300Z/report.json`.
  - Candidate: `tests/artifacts/perf/record-exact-body-write-cyangorge-20260504T2300Z/report.json`.
- Result: rejected before commit. Tiny rows improved, but small/medium/large
  FrankenSQLite medians regressed; the section only appeared better because the
  C SQLite large-row sample slowed down.
- Do not retry the same exact-body `Vec::resize` strategy unless a profile proves
  payload append/copy dominates and a close A/B record-section run improves the
  actual FrankenSQLite medians.

## 2026-05-04 - Two-byte runtime precomputed record headers, repeat

- Target: record-size insert section, especially medium/large rows with long
  TEXT serial types.
- Touched: `crates/fsqlite-types/src/record.rs`, `crates/fsqlite-vdbe/src/engine.rs`.
- Candidate shape: add a two-byte runtime precomputed-header slot for direct
  inserts whose first row has long TEXT/BLOB serial types.
- Evidence:
  - Baseline: `tests/artifacts/perf/record-current-clean-cyangorge-20260504T2300Z/report.json`.
  - Candidate: `tests/artifacts/perf/record-two-byte-runtime-header-cyangorge-20260504T2315Z/report.json`.
  - Candidate repeat: `tests/artifacts/perf/record-two-byte-runtime-header-repeat-cyangorge-20260504T2320Z/report.json`.
- Result: rejected before commit. The repeat showed tiny/medium improvements but
  large-row FrankenSQLite time regressed from the clean baseline, and the ratio
  improvement was mostly from a slower C SQLite large-row sample.
- Do not retry as a broad runtime-header extension. Only revisit if two-byte
  patching is isolated to a proven row shape and judged on FrankenSQLite absolute
  time as well as C/FrankenSQLite ratio.
