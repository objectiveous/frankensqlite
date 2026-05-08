# SilverAnchor DML profile and range-patch rejection - 2026-05-08

## Scope

- Starting point: pushed `65d54751 perf(pager): stage page one without pagebuf copy`.
- Agent Mail coordination: `macro_start_session` and the narrower
  `file_reservation_paths` call both timed out while trying to reserve
  `crates/fsqlite-core/src/connection.rs`, `crates/fsqlite-btree/src/cursor.rs`,
  the artifact directory, and the negative-results ledger.
- Insert profile pivot: `../silveranchor-insert-profile-20260508T1155Z/insert-profile-fresh.json`
  did not reproduce the full-matrix INSERT gap in isolation, so the pass moved
  to the DML rows that remained visible in the full quick report.

## Baseline and profile

- Baseline filtered update/delete run:
  `dml-profile.json` and `dml-profile.stderr`.
- Narrow comparator baseline:
  `perf-update-delete-100x1000.stderr`.
- Perf record:
  `perf-update-isolated.data`, with top self-time including
  `__memmove_avx_unaligned_erms`, allocator frames, direct simple update,
  `TransactionKind::write_page_data`, and B-tree page load/seek paths.

## Rejected candidate

- Candidate: add a `BtCursor` payload-range overwrite primitive and route the
  fixed-width REAL direct UPDATE path through it.
- Focused tests passed before rejection:
  `cargo test -p fsqlite-btree test_table_overwrite_current_payload_range_same_size_no_overflow_patches_slice -- --nocapture`
  and
  `cargo test -p fsqlite-core test_direct_simple_update_single_real_column_patches_payload_without_decode -- --nocapture`.
- Candidate artifact:
  `dml-profile-candidate.json` plus `perf-update-delete-100x1000-candidate.stderr`.
- Same-time clean baseline artifact:
  `dml-profile-clean-head-repeat.json` plus
  `perf-update-delete-100x1000-clean-head-repeat.stderr`, from detached clean
  worktree `/data/tmp/frankensqlite-silveranchor-dml-baseline-65d54751`.

## Matrix decision

The range-patch candidate was rejected. Against the same-time clean baseline,
the focused matrix summary worsened:

| Metric | Clean `65d54751` | Range patch candidate |
| --- | ---: | ---: |
| average ratio | 1.0023996891494373 | 1.121703499721951 |
| geomean ratio | 0.9891774494336053 | 1.0981172488823585 |
| median ratio | 0.9521782416313966 | 1.2005915733983752 |
| p90 ratio | 1.3360203368047023 | 1.4432270168855534 |

Row-level result: 100-row UPDATE improved in absolute FrankenSQLite time, but
1000-row UPDATE regressed `0.388989 ms -> 0.488295 ms` and 1000-row DELETE
regressed `0.347651 ms -> 0.409748 ms`. That fails the matrix keep gate.

## Kept follow-up

- Kept source change: `0382ee26 fix(pager): unpublish staged page one before mutation`.
- Reason: correctness hardening for the prior page-one optimization. If a
  staged Page 1 already has a published snapshot, the commit path must copy it
  back to an unpublished buffer before mutating header bytes; otherwise final
  publication can return the stale cached page image.
- Note: `dml-profile-pager-fix-head.json` and
  `perf-update-delete-100x1000-pager-fix-head.stderr` are not pager-fix-only
  proof artifacts. They were captured after a new unowned dirty successor diff
  appeared in `crates/fsqlite-btree/src/cursor.rs` and
  `crates/fsqlite-core/src/connection.rs`
  (`table_overwrite_current_real_column_same_size_no_overflow`). Treat those as
  dirty exploratory artifacts only; they are not staged or claimed by this
  closeout.
