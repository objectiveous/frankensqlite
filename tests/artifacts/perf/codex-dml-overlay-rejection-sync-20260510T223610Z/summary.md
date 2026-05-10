# DML Overlay Rejection Sync

Date: 2026-05-10T22:36:10Z
HEAD: `a6295256`

## Purpose

This pass publishes the missing May 10 side-worktree evidence for
`bd-db300.11.1`, the committed leaf-delta DELETE overlay prototype family. The
goal is to keep future DML work from repeating a tombstone-only overlay,
dense-rowid queued overlay, or standalone freed-page lookup retry that has
already missed the keep gate.

## Published Summaries

- `tests/artifacts/perf/codex-logical-tombstone-probe-20260510T0058Z/summary.md`
  from `/data/tmp/frankensqlite-codex-logical-tombstone-probe-20260510`.
- `tests/artifacts/perf/codex-dense-rowid-delete-overlay-20260510T0115Z/summary.md`
  from `/data/tmp/frankensqlite-codex-delete-seek-hint-probe-20260510`.
- `tests/artifacts/perf/codex-frontier-profile-20260510T0148Z/summary.md`
  from `/data/tmp/frankensqlite-codex-frontier-profile-20260510`.
- `tests/artifacts/perf/codex-frontier-delete-isolated-profile-20260510T0158Z/summary.md`
  from `/data/tmp/frankensqlite-codex-frontier-profile-20260510`.

Raw `perf.data` recordings and large `perf report` text dumps were left in the
side worktrees. They are machine-local, large, and ignored by the curated
`tests/artifacts/perf/` policy. The committed summaries retain the exact
commands, focused numbers, profile top frames, and side-worktree source paths.

## Consolidated Result

The logical tombstone probe passed narrow private `:memory:` correctness tests,
but focused DELETE remained slower than C SQLite:

| Mode | Rows | Iters | F per-row DELETE | C per-row DELETE | F/C |
| --- | ---: | ---: | ---: | ---: | ---: |
| standard | 100 | 10 | 1783 ns | 465 ns | 3.84x |
| standard | 1000 | 5 | 729 ns | 364 ns | 2.00x |
| standard | 10000 | 3 | 709 ns | 355 ns | 2.00x |
| isolated | 10000 | 3 | 597 ns | 278 ns | 2.15x |

The dense-rowid queued DELETE overlay also passed local build/correctness gates,
but the focused standard 100-row DELETE row regressed to `3244 ns` per FSQLite
DELETE versus `471 ns` for C SQLite (`6.89x` slower).

The isolated DELETE-body profile confirmed that DELETE itself is still a real
gap, while UPDATE in the same isolated harness is already faster than C SQLite:

| Workload | FSQLite | C SQLite | Ratio |
| --- | ---: | ---: | ---: |
| 100-row isolated DELETE | 1594 ns/delete | 304 ns/delete | 5.25x |
| 1000-row isolated DELETE | 1814 ns/delete | 294 ns/delete | 6.18x |
| 100-row isolated UPDATE | 130 ns/update | 303 ns/update | 0.43x |

The isolated profile's top self-time frames were
`TransactionKind::get_page` (`35.96%`) and
`TransactionKind::write_page_data` (`23.12%`). Annotation placed most local
`get_page` samples in the transaction-local `freed_pages` membership loop, but
that standalone lookup family was already rejected by the current keep gate:
it helped a diagnostic long-DELETE microcase while worsening the focused
UPDATE/DELETE matrix.

## Boundary

Do not retry a tombstone-only DELETE overlay, dense-rowid queued overlay, or
standalone `freed_pages` lookup as the next DML patch. A retry is only justified
as part of a broader transaction-local DML mutation operator that proves
read-your-writes, rollback, savepoints, schema drift, duplicate/missing rowids,
and MVCC publication behavior before winning the 5-row, 50-row, and 500-row
DELETE rows in one focused benchmark window.
