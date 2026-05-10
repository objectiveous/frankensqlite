# Non-DML Frontier Rescreen

Date: 2026-05-10

Source artifacts reviewed:

- Full quick:
  `tests/artifacts/perf/codex-fresh-frontier-full-quick-20260510T093306Z/full-quick.json`
- Insert profile:
  `tests/artifacts/perf/codex-fresh-frontier-insert-profile-20260510T093306Z/insert.json`
  and `stderr.log`
- Concurrent profile hook:
  `tests/artifacts/perf/codex-concurrent-profile-hook-20260510T1140Z/`
- Negative-results ledger:
  `docs/progress/perf-negative-results.md`

Current non-DML C-faster frontier from the full quick matrix:

- `INSERTThroughput - Record Size Comparison (10K rows, single txn) /
  large_10col`: `8.965639 ms` C SQLite, `11.470256 ms` FrankenSQLite,
  `1.279357333x` F/C.
- 100-row INSERT fixed-cost rows:
  `tiny_1col` `1.087511669x`, `small_3col` `1.153645146x`,
  `medium_6col` `1.201775885x`, `large_10col` `1.157806117x`.
- Small transaction-strategy rows:
  `100 rows / single txn` `1.132581119x` and
  `100 rows / batched (100/txn)` `1.118619291x`.
- Low-writer concurrent rows remain near the boundary, but the current profile
  shows the fast direct-INSERT lane is already used and the pending page-run
  path is inactive for file-backed concurrent inserts.

Screened one-lever candidates:

- Standalone INSERT setup/open trimming: exhausted by the connection/open,
  PRAGMA, transaction-control, sqlite_master, function-registry, and page-cache
  fixed-cost rejects already in the ledger.
- Standalone concat / record-template / param-one / integer-text row-build
  variants: exhausted by the direct concat encoder, record-template serializer,
  param-one concat encoder, and integer placeholder cache rejects.
- Standalone direct INSERT page-run threshold/arena variants: the current code
  already contains the measured record-band policy (`16` byte admission,
  arena below `384`, owned above `384`); broad/admission-only variants are
  ledgered negatives.
- Standalone file-backed concurrent page-run admission or wait tuning: already
  screened by the 2026-05-10 concurrent profile hook. The profile points at
  transaction retry/stale-snapshot churn and MVCC publication shape, not a
  direct INSERT parser/serializer miss.

Conclusion:

No additional safe micro-optimization is justified from the current artifacts.
The next credible non-DML source change is a real fused record/page builder
that computes row bodies and B-tree page layout in one pass and then publishes
the resulting pages through the pager/MVCC path with correctness coverage.
For the concurrent rows, the corresponding design must batch page construction
and MVCC page publication together for file-backed `BEGIN CONCURRENT`.

Keep gate:

- Focused INSERT A/B must improve the large-row absolute medians and focused
  write-bulk/geomean scores.
- Focused concurrent A/B must improve the 2/4 writer rows without regressing
  8 writers.
- Full quick must improve or preserve the primary weighted score and C-faster
  row count in the same measurement window.
