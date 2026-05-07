# Retained dirty-table mark fast path

Status: rejected and reverted.

Candidate: add an early return in
`Connection::retained_autocommit_mark_dirty` when the retained autocommit dirty
set already contains a lowercase table name and all retained read caches /
preserve flags are absent.

Correctness proof:

- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-dirty-mark-local-target cargo test -p fsqlite-core test_retained_autocommit_dirty_mark_repeated_table_still_clears_overlay_cache -- --nocapture`

Current-head gate:

- Baseline:
  `current0f6-baseline-dirtymark-transaction.json`
- Candidate:
  `current0f6-candidate-dirtymark-transaction.json`
- Both built from `0f6a2fd6`; candidate carried only the `connection.rs` patch.

Result:

- Primary transaction score regressed `1.1329 -> 1.1551`.
- Geomean regressed `1.0102 -> 1.0731`.
- Write-bulk geomean regressed `0.9216 -> 1.0117`.
- Autocommit medians improved for 1K/10K rows, but batched and single-transaction
  regressions failed the section keep gate.

Disposition: source reverted; negative ledger entry added in
`docs/progress/perf-negative-results.md`.
