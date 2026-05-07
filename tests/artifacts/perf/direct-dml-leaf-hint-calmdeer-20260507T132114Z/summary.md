# Direct DML Leaf-Hint Probe

- Date: 2026-05-07
- Candidate commit tested: `6e13684f` (`perf(direct-dml): cache hinted leaf for repeated fixed-width UPDATEs`)
- Baseline parent: `5af003c1`
- Target: `comprehensive-bench --quick --filter update` UPDATE/DELETE throughput
- Decision: rejected and reverted after the focused matrix failed the keep gate.

## Candidate

The candidate added a conservative `PreparedDirectDmlLeafHint` on `Connection`
plus `BtCursor::table_move_to_leaf_hint`. After a same-size fixed-width REAL
direct-simple UPDATE overwrote a payload in place, the connection remembered the
leaf page and tried that leaf first for the next UPDATE on the same table root.

The hint was cleared on direct INSERT, direct DELETE, mixed-shape UPDATE, and
delete+insert fallback paths.

## Correctness Evidence

- `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-dml-leafhint-test-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-btree test_table_move_to_leaf_hint_uses_hinted_leaf_when_bounds_match -- --nocapture`
- `env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-dml-leafhint-test-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core direct_simple_update -- --nocapture`

Both passed. The btree test covered exact hit, in-leaf miss, and out-of-bounds
fallback; the core filter covered existing direct-simple UPDATE/DELETE guards.

## Performance Evidence

Built two `release-perf` binaries in separate target dirs:

- Baseline worktree: `/data/tmp/frankensqlite-dml-leafhint-baseline-20260507T132114Z` at `HEAD^`
- Candidate worktree: `/data/projects/frankensqlite` at `6e13684f`

Primary run:

| Scenario | Baseline FSQLite ms | Candidate FSQLite ms | Candidate vs baseline | Baseline ratio | Candidate ratio |
| --- | ---: | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.144170 | 0.130484 | -9.49% | 1.7203 | 1.5337 |
| 100 rows / delete 5 rows | 0.123081 | 0.151113 | +22.78% | 1.5170 | 1.8396 |
| 1000 rows / update 100 rows | 0.439974 | 0.452818 | +2.92% | 1.0339 | 1.1482 |
| 1000 rows / delete 50 rows | 0.395722 | 0.412283 | +4.19% | 1.0365 | 1.1020 |
| 10000 rows / update 1000 rows | 3.985014 | 4.497334 | +12.86% | 1.0966 | 1.2704 |
| 10000 rows / delete 500 rows | 3.614440 | 4.031732 | +11.55% | 1.0441 | 1.1959 |

Repeat run:

| Scenario | Baseline FSQLite ms | Candidate FSQLite ms | Candidate vs baseline | Baseline ratio | Candidate ratio |
| --- | ---: | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.141646 | 0.140463 | -0.84% | 1.2663 | 1.2938 |
| 100 rows / delete 5 rows | 0.122349 | 0.163867 | +33.93% | 1.4779 | 2.0437 |
| 1000 rows / update 100 rows | 0.474549 | 0.457127 | -3.67% | 0.9544 | 1.1369 |
| 1000 rows / delete 50 rows | 0.395791 | 0.420177 | +6.16% | 0.9019 | 1.1391 |
| 10000 rows / update 1000 rows | 4.464252 | 4.415611 | -1.09% | 1.2310 | 1.2464 |
| 10000 rows / delete 500 rows | 4.003529 | 4.003017 | -0.01% | 1.1756 | 1.1987 |

The primary run was a clear regression outside the tiny update row. The repeat
left update rows noisy or mixed and still regressed the small delete rows. Since
the section is C-relative and the candidate added state checks to the shared
direct DML path, this is not a keep.

## Artifacts

- `baseline-update.json`, `candidate-update.json`
- `baseline-update-repeat.json`, `candidate-update-repeat.json`
- `compare-update.tsv`, `compare-update-repeat.tsv`
- Build stdout/stderr logs for both binaries
- `candidate-source.patch`
