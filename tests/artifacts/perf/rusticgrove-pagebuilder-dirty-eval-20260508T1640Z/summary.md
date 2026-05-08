# Dirty Page-Builder Evaluation - 2026-05-08

## Source Basis

- Shared checkout: `/data/projects/frankensqlite`
- Git HEAD: `5c39cd73b757968fb1324f92acf4183752d6f876`
- Branch: `main`
- Dirty source files under SwiftGate's page-builder slice:
  - `crates/fsqlite-btree/src/cursor.rs`
  - `crates/fsqlite-btree/src/lib.rs`
  - `crates/fsqlite-core/src/connection.rs`
- Benchmark JSON records `git_dirty: true`.
- This pass was read-only for the dirty source files. The only owned artifact is this evaluation bundle.

## Read-Only Correctness Checks

```bash
cargo test -p fsqlite-btree \
  test_table_bulk_load_empty_root_prebuilt_leaf_pages_builds_reachable_tree \
  -- --nocapture
```

Result: passed.

```bash
cargo test -p fsqlite-core \
  test_prepared_direct_insert_large_empty_page_run_uses_prebuilt_leaves \
  -- --nocapture --test-threads=1
```

Result: passed.

## Build

```bash
env CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=12 \
  cargo build --profile release-perf \
  -p fsqlite-e2e \
  --bin comprehensive-bench
```

Result: passed.

## Benchmark Command

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/cargo-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/rusticgrove-pagebuilder-dirty-eval-20260508T1640Z/dirty-insert-profile.json \
  --html tests/artifacts/perf/rusticgrove-pagebuilder-dirty-eval-20260508T1640Z/dirty-insert-profile.html
```

- Generated: `2026-05-08 16:44:45 UTC`
- Result: passed and produced JSON/HTML.

## Clean Comparator

- Bundle: `tests/artifacts/perf/rusticgrove-next-frontier-20260508T1630Z`
- JSON: `insert-profile.json`
- Generated: `2026-05-08 16:35:28 UTC`
- Clean worktree commit: `f749770ccc32857cf936ae8ce9f48f15e00ca233`

Clean focused insert result:

- Scenarios: `25`
- Faster / comparable / slower: `19 / 3 / 3`
- Average ratio: `0.7843378778`
- Geomean ratio: `0.7645362854`
- Weighted score: `0.7894624354`
- P90 / P99 ratio: `1.0621609033 / 1.1126186681`

## Dirty Page-Builder Focused Insert Result

- Scenarios: `25`
- Faster / comparable / slower: `16 / 4 / 5`
- Average ratio: `0.8347510087`
- Geomean ratio: `0.8111669802`
- Weighted score: `0.8317763024`
- P90 / P99 ratio: `1.1338115468 / 1.2010066059`

The dirty candidate worsened every aggregate focused-insert gate metric versus
the clean comparator.

## Largest Dirty Regressions Versus Clean Comparator

| Scenario | Clean ratio | Dirty ratio | Clean F ms | Dirty F ms |
| --- | ---: | ---: | ---: | ---: |
| single txn large_10col, 100 rows | `0.6391` | `0.9545` | `0.157916` | `0.170078` |
| single txn tiny_1col, 100 rows | `0.8913` | `1.1187` | `0.068828` | `0.069650` |
| single txn small_3col, 100 rows | `1.0622` | `1.2010` | `0.084377` | `0.087814` |
| txn strategy small_3col, 100 rows autocommit | `0.8493` | `0.9796` | `0.105428` | `0.115747` |
| txn strategy small_3col, 10000 rows single txn | `0.6304` | `0.7121` | `2.005584` | `2.317368` |
| record-size large_10col, 10000 rows | `0.9923` | `1.0350` | `9.306969` | `10.295721` |
| single txn large_10col, 10000 rows | `0.9720` | `1.0047` | `9.120270` | `9.643219` |

## Decision

Do not land the dirty page-builder slice as-is.

The focused insert admission gate failed before a full quick matrix was
justified:

1. Faster rows dropped from `19` to `16`.
2. Slower rows increased from `3` to `5`.
3. Average, geomean, weighted, P90, and P99 ratios all moved in the wrong direction.
4. The large-row target rows that motivated a page-builder probe were worse in the dirty run:
   - single txn `large_10col` 10K: `0.9720x` clean to `1.0047x` dirty
   - record-size `large_10col` 10K: `0.9923x` clean to `1.0350x` dirty

If this slice continues, the next useful step is to prove whether the prebuilt
leaf path is actually hot for the benchmark rows and to repeat the focused
insert gate before spending time on a full quick run. The current artifact is a
rejection of this dirty candidate, not a source-change endorsement.
