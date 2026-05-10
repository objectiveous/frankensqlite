# Frontier Scratch Rejection Sync

Date: 2026-05-10

Current HEAD reviewed: `1742da90 docs(perf): record DML mutation design blocker`.

## Purpose

Recent `bd-db300.10.10` bead comments recorded several scratch-worktree
performance probes whose source patches were reverted and whose evidence lived
only in `/data/tmp/frankensqlite-codex-frontier-profile-20260510/`. This
artifact makes those rejection results durable and searchable from `main`.

The synced scratch worktree base was `601dc619` for the probe family, so exact
ratios are evidence for that same measurement window. Their retry guidance is
still relevant because the current `HEAD` has since added only docs/artifact
records for these adjacent perf fronts.

## Synced Rejections

| Candidate | Target | Touched During Probe | Result | Durable Source |
| --- | --- | --- | --- | --- |
| Bulk leaf varint-length shortcut | INSERT page-run grouping | `crates/fsqlite-btree/src/cursor.rs` | Rejected: weighted INSERT score worsened `0.836266 -> 0.909343`, C-faster rows `5 -> 8` | `bd-db300.10.10` comment `3707`; `/data/tmp/frankensqlite-codex-frontier-profile-20260510/tests/artifacts/perf/codex-frontier-varint-len-probe-20260510Tcandidate/summary.md` |
| Direct INSERT page-run writer storage | INSERT record-to-page-run copy boundary | `crates/fsqlite-core/src/connection.rs` | Rejected: broad writer path worsened avg/geomean/median `0.8394/0.8089/0.8228 -> 0.8663/0.8445/0.8611`; owned-only regressed large 10-col 10K to `1.40x` slower | `bd-db300.10.10` comment `3706`; `/data/tmp/frankensqlite-codex-frontier-profile-20260510/tests/artifacts/perf/codex-frontier-insert-writer-probe-20260510T0312Z/summary.md` |
| Generated `?1` page-run payload source | INSERT row generation deferral to page-run flush | `crates/fsqlite-core/src/connection.rs`, `crates/fsqlite-btree/src/cursor.rs` | Rejected: focused INSERT weighted score `0.894259`, p99 `3.847718`; `10000 rows / batched (1000/txn)` regressed to `3.85x` slower | `bd-db300.10.10` comment `3712`; `/data/tmp/frankensqlite-codex-frontier-profile-20260510/tests/artifacts/perf/codex-frontier-generated-pagerun-probe-20260510Tcandidate/rejection-summary.md` |
| File-backed page-run admission | Low-thread concurrent writer rows | `crates/fsqlite-core/src/connection.rs` | Rejected: focused `mt-mvcc-bench` 2-thread throughput ratio dropped `0.59x -> 0.53x`; low-thread time ratios stayed red | `bd-db300.10.10` comment `3709`; `/data/tmp/frankensqlite-codex-frontier-profile-20260510/tests/artifacts/perf/codex-frontier-file-backed-page-run-20260510T0415Z/rejection-summary.md` |
| Read-witness record-time dedup | Low-thread concurrent witness churn | `crates/fsqlite-mvcc/src/begin_concurrent.rs` or adjacent witness code | Rejected: comprehensive concurrent score worsened `0.864282 -> 0.957858`, p90/p99 `1.181698 -> 1.257733`, 4-writer row `1.257733x` slower | `bd-db300.10.10` comment `3708`; `/data/tmp/frankensqlite-codex-frontier-profile-20260510/tests/artifacts/perf/codex-frontier-read-witness-dedup-20260510T0342Z/rejection-summary.md` |
| Transaction-control SQL fast path | Small fixed-cost INSERT/DML setup | `crates/fsqlite-core/src/connection.rs` | Rejected: INSERT red families remained and DML split stayed red; update bucket `1.91x`, delete bucket `4.98x` | `bd-db300.10.10` comment `3710`; `/data/tmp/frankensqlite-codex-frontier-profile-20260510/tests/artifacts/perf/codex-frontier-txn-fastpath-probe-20260510T0400Z/rejection-summary.md` |
| Simple connection PRAGMA execute fast path | Fresh `:memory:` setup cost | `crates/fsqlite-core/src/connection.rs` | Rejected: DML quick/filter worsened target small rows, avg/geomean/median `1.124177/1.094957/1.348310` | `bd-db300.10.10` comment `3711`; `/data/tmp/frankensqlite-codex-frontier-profile-20260510/tests/artifacts/perf/codex-frontier-simple-pragma-probe-20260510Tcandidate/rejection-summary.md` |
| WAL-control env cache | Fresh setup/open-state overhead | `crates/fsqlite-pager/src/group_commit.rs` or adjacent WAL-control env resolution | Rejected: best focused DML result did not repeat; same-window clean baseline was essentially equal to candidate repeat | `bd-db300.10.10` comment `3705`; `/data/tmp/frankensqlite-codex-frontier-profile-20260510/tests/artifacts/perf/codex-frontier-env-cache-probe-20260510T0330Z/summary.md` |

## Current Interpretation

The synced probes close off the remaining obvious one-lever families around
INSERT page-run storage, generated payload deferral, file-backed page-run
admission, low-thread witness bookkeeping, exact transaction/PRAGMA setup
fast paths, and WAL-control env resolution. They are not proof that the broader
frontier is impossible; they are proof that these isolated forms do not move the
benchmark matrix safely.

## Retry Conditions

Only retry these families when the candidate is part of a broader design that
also changes the representation boundary:

- INSERT: a fused row/page builder must remove page layout and copy boundaries
  together without moving page construction into per-row execution or regressing
  batched transactions.
- Concurrent writers: page construction and MVCC publication must be batched
  together; standalone witness or admission changes are fenced.
- Setup/open-state: `SharedMvccState`, pager/page-cache construction, page-1
  bootstrap, PRAGMA/transaction dispatch, and env/config resolution must move as
  one measured redesign, with repeated focused gates and full quick neutrality.
