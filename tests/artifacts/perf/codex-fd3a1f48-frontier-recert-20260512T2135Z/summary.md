# fd3a1f48 Frontier Recertification

- Date: 2026-05-12 21:35Z
- Source commit: `fd3a1f48dce044bd91f870724fdbd873b695d8bf`
- Source state: clean scratch worktree
- Purpose: remeasure the current perf frontier after the MVCC index
  materialization-key hardening commit and decide whether an unfenced
  single-lever source patch is still justified.

## Commands

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-next-target \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick \
  --json-out tests/artifacts/perf/codex-current-fullquick-fd3a1f48-20260512T2120Z/full-quick.json \
  --no-html

env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-next-target \
  FSQLITE_BENCH_PROFILE_DML=1 \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter update \
  --json-out tests/artifacts/perf/codex-current-dml-profile-fd3a1f48-20260512T2110Z/update-delete-profile.json \
  --no-html

env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-next-target \
  FSQLITE_BENCH_PROFILE_INSERT=1 \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter insert \
  --json-out tests/artifacts/perf/codex-current-insert-profile-fd3a1f48-20260512T2122Z/insert-profile.json \
  --no-html
```

The source JSON and stdout/stderr logs were copied into this artifact directory:

- `full-quick.json`, `full-quick.stdout.log`, `full-quick.stderr.log`
- `update-delete-profile.json`, `update-delete-profile.stdout.log`,
  `update-delete-profile.stderr.log`
- `insert-profile.json`, `insert-profile.stdout.log`,
  `insert-profile.stderr.log`

## Full Quick Result

- Scenarios: `93`
- FrankenSQLite faster / comparable / C SQLite faster: `81 / 3 / 9`
- Average F/C ratio: `0.4886618611`
- Geomean F/C ratio: `0.2714660338`
- Primary `per_category_weighted.score`: `0.3676859704`
- p90 ratio: `1.0341433039`
- p99 ratio: `3.0998271392`

Rows still above `1.0x` F/C:

| Section | Row | F/C | FSQLite ms | C SQLite ms |
| --- | --- | ---: | ---: | ---: |
| INSERT single txn | tiny_1col 100 rows | `1.0633x` | `0.074600` | `0.070162` |
| INSERT single txn | small_3col 100 rows | `1.1100x` | `0.084709` | `0.076313` |
| INSERT single txn | medium_6col 100 rows | `1.0264x` | `0.107040` | `0.104286` |
| INSERT single txn | large_10col 100 rows | `1.0295x` | `0.159579` | `0.155000` |
| INSERT txn strategy | 100 rows / batched | `1.1421x` | `0.085179` | `0.074580` |
| INSERT txn strategy | 100 rows / single txn | `1.1131x` | `0.084879` | `0.076253` |
| Concurrent writers | 2 writers x 1000 rows | `1.0665x` | `12.902059` | `12.097112` |
| Concurrent writers | 4 writers x 1000 rows | `1.0341x` | `19.962244` | `19.303170` |
| UPDATE/DELETE | 100 rows / update 10 rows | `1.4495x` | `0.006201` | `0.004278` |
| UPDATE/DELETE | 100 rows / delete 5 rows | `3.0998x` | `0.007173` | `0.002314` |
| UPDATE/DELETE | 1000 rows / delete 50 rows | `1.8221x` | `0.029154` | `0.016000` |
| UPDATE/DELETE | 10000 rows / delete 500 rows | `1.6350x` | `0.261730` | `0.160080` |

## Focused Profiles

The profiled UPDATE/DELETE run kept all DELETE rows on the prepared direct path
(`slow=0`). For the 500-delete row, the profile reports:

- `delete_leaf_active=433/496`
- `delete_leaf_miss=63`
- `delete_leaf_miss_out_of_leaf=60`
- `delete_leaf_miss_last_cell=3`
- `delete_leaf_flush=64/64`
- `delete_leaf_materialize=64/107007ns`
- `delete_leaf_write=64/13433ns`
- `delete_leaf_search=560/46573ns`
- `delete_leaf_dupcheck=500/14204ns`
- `delete_leaf_compact=497/17221ns`
- `delete_leaf_cellparse=497/14333ns`

The INSERT profile repeated the already-fenced 100-row fixed-cost pattern:

- tiny_1col 100 rows: `1.2355x`
- small_3col 100 rows: `1.1171x`
- medium_6col 100 rows: `1.0798x`
- large_10col 100 rows: `1.1107x`
- small_3col 100 rows / single txn: `1.3755x`

## Source Screen

The MVCC materialization hardening commit touched only
`crates/fsqlite-mvcc/src/materialize.rs`. It corrects materialized index-key
matching and missing-cell validation, but it does not change the prepared
direct DELETE/INSERT hot path measured above.

The remaining tempting source edits are all already fenced by same-window
measurement failures:

- retained DELETE leaf-run admission/search/compactness/materializer tweaks;
- direct writer, borrowed publication, cursorless flush, and direct-flush
  pre-gates;
- rowid/logical DELETE buffers or commit-side-only cell-log hooks;
- direct UPDATE active-patch continuation and small transaction-envelope
  bypasses;
- fixed-cost INSERT row-template/schema/guard/page-run variants;
- low-thread concurrent wait/retry shaping.

The MVCC crate now contains cell-delta materialization and transaction-local
read-view primitives, but the core execution path still observes pager/B-tree
physical page images. A direct `record_delete()` hook without a core B-tree
overlay would still be incomplete: later reads, restore INSERTs, rollback,
savepoints, and commit publication would not all observe the same row-level
state.

## Decision

No source patch was attempted from this pass.

The next credible source lever remains the broader transaction-local DML
mutation operator: buffer rowid DML messages in logical key space, route
point/scanned reads through the same transaction-local view, and publish the
same MVCC conflict surface at proven flush/commit boundaries. Anything smaller
would repeat the rejected standalone direct DELETE, fixed INSERT, or cell-log
hook families above.
