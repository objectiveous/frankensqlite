# Legacy Write-Gap Notes

Date: 2026-05-07
Agent: TanBear

## Scope

This artifact records a no-code comparison pass while
`crates/fsqlite-core/src/connection.rs` and
`tests/artifacts/perf/large-record-gap-crimsongorge-20260507T1610Z/**` are
exclusively reserved by CrimsonGorge. The shared worktree contains a dirty
compact-concat candidate in `connection.rs`; I did not edit, revert, or package
that peer-owned change.

## Current Clean Baseline

Clean full quick matrix artifact:
`tests/artifacts/perf/full-quick-clean-tanbear-20260507T1633Z/summary.md`.

Remaining clean C-faster rows are concentrated in:

- Small UPDATE/DELETE setup rows.
- Small and wide single-transaction INSERT rows.
- Low-writer concurrent insert rows.

Clean insert profile artifact:
`tests/artifacts/perf/insert-profile-clean-tanbear-20260507T1641Z/summary.md`.

For clean `large_10col` 10K rows, the dominant visible costs were row
construction and commit/page volume rather than the measured B-tree insert
counter alone:

- `fs_insert_single_txn_large_10col_10000`: `row_build_ns=5057682`,
  `btree_insert_ns=851722`, `commit_roundtrip_ns=2056642`,
  `page_pool_misses=2006`.
- `fs_insert_record_size_large_10col_10000`: `row_build_ns=5100094`,
  `btree_insert_ns=664922`, `commit_roundtrip_ns=1812184`,
  `page_pool_misses=2006`.

## Legacy SQLite Comparison

Legacy C SQLite routes write rows through a very narrow VDBE-to-B-tree path:

- `OP_Concat` (`legacy_sqlite_code/sqlite/src/vdbe.c`) grows the output once
  to the exact combined byte count and can avoid a copy when the output register
  is the left input. This explains why isolated concat micro-specializations can
  help row-build profiles but still fail the full matrix when they do not
  remove a broader record/page assembly cost.
- `OP_MakeRecord` computes serial types, header bytes, and payload bytes in one
  opcode-local pass, then writes the final record into one reusable output
  register buffer. `sqlite3VdbeSerialPut()` is explicitly documented as inlined
  into `OP_MakeRecord`.
- `OP_Insert` passes the prebuilt record to `sqlite3BtreeInsert()` with
  `OPFLAG_APPEND`, `OPFLAG_SAVEPOSITION`, and `OPFLAG_PREFORMAT` when available.
  The B-tree layer can therefore avoid reformatting and can trust a previous
  seek/append result.
- `sqlite3BtreeInsert()` has same-size overwrite, seek-result reuse, append
  hints, preformatted payload support, and cursor-position preservation in the
  same function. The important design point is not one local shortcut; it is a
  continuous path from prepared value registers to a preformatted cell/record
  and retained right-edge cursor state.
- `balance_quick()` only handles the rightmost leaf/rightmost parent-child
  overflow case. FrankenSQLite already has a similar quick-balance path in
  `crates/fsqlite-btree/src/balance.rs`; broadening it blindly is fenced by the
  negative ledger. The remaining opportunity is to feed it fewer recreated
  cells and less repeated connection-layer setup, not to retry generic split
  shortcuts.

## Negative-Ledger Boundaries

The following source shapes are already fenced and should not be retried as
standalone changes:

- Param-one or text-piece concat direct INSERT encoder variants.
- Direct INSERT concat owned-text move and thread-local value pools.
- Broad non-empty page-run/page-builder admission.
- Pager page-buffer lease-size or contiguous append tweaks.
- Direct UPDATE/DELETE leaf hints and one-row payload patches.
- Broad retained-autocommit page-run widening without a correctness proof for a
  specific prepared INSERT shape.

## Recommendation Cards

### A. Finish Or Revert The Peer-Owned Compact Concat Candidate

Evidence:
`tests/artifacts/perf/large-record-gap-crimsongorge-20260507T1610Z/summary.md`
reports a candidate average full quick ratio of `0.4786135613` versus local
baseline `0.5110643564`, with C-faster rows reduced from `16` to `11`.

Expected value: high, but currently owned by CrimsonGorge. Do not duplicate it.

Keep gate:

1. Commit only after `cargo fmt --check`, focused prepared-insert tests, and
   workspace check/clippy pass or a documented blocker.
2. Force-add the ignored artifact bundle if it is part of the evidence.
3. Repeat a clean full quick matrix from a clean worktree after the commit,
   because the current shared tree is dirty.

Fallback:
If the peer candidate fails workspace verification or a clean full quick repeat,
record the rejection in `docs/progress/perf-negative-results.md` and restore the
source by a normal revert commit, not by destructive checkout.

### B. Row-Template Record Builder Coupled To Page-Run Flush

Candidate:
Compile prepared INSERT rows into a row-template object that can emit both the
SQLite record image and page-run cell metadata in one pass. This is the closest
Rust analogue to C SQLite's fused `OP_MakeRecord` plus `BTREE_PREFORMAT`
handoff, but must be narrower than the rejected concat-only variants.

Expected value: medium-high after compact concat lands or is rejected. It
attacks the remaining row-build plus commit/page-volume pair together instead
of only shortening expression dispatch.

Keep gate:

1. Start with one exact row: `large_10col` record-size 10K.
2. Require `row_build_ns` to drop and require `commit_us` /
   `commit_roundtrip_ns` not to absorb the saved time.
3. Run focused insert profile, then full quick. Keep only if the target row and
   the weighted full quick matrix both move in the right direction.

Fallback:
If the row-template only improves row-build while worsening commit/page-run
cost, fence it as another row-builder-local loss.

### C. Retained Direct-DML Batch Cursor Kernel

Candidate:
For small UPDATE/DELETE rows, retain cursor positioning and statement setup
across the direct DML batch instead of adding new one-row leaf hints.

Expected value: medium. CrimsonGorge's
`tests/artifacts/perf/update-delete-gap-crimsongorge-20260507T1555Z/summary.md`
shows the small rows are not dominated by the mutation counter alone; setup is
still a large share.

Keep gate:

1. Benchmark `100 rows / update 10 rows` and `100 rows / delete 5 rows` first.
2. Require correctness tests for failed constraints and `changes()` semantics.
3. Full quick must not regress insert or concurrent-writer sections.

Fallback:
If isolated per-row mutation improves but setup/full quick does not, reject it.

## Blocker

The highest-value source file for A/B/C is currently peer-owned:

- `crates/fsqlite-core/src/connection.rs`, reserved by CrimsonGorge until
  2026-05-07T18:22:17Z.

I sent Agent Mail message `2965` asking whether the dirty compact-concat
candidate is still owned or safe to verify/package. Until that is resolved, the
safe campaign move is to avoid `connection.rs` edits and use this artifact as
the handoff map.
