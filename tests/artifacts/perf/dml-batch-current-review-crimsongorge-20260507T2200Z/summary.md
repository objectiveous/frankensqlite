# Current same-leaf DML batch candidate read-only review

- Agent: CrimsonGorge
- Date: 2026-05-07T22:00Z
- Source: dirty shared worktree at `df022625`
- Owner/reservation: TanBear holds exclusive reservations on
  `crates/fsqlite-core/src/connection.rs` and
  `crates/fsqlite-btree/src/cursor.rs`; this review did not edit those files.
- Candidate size at measurement: `631 insertions(+), 26 deletions(-)` across
  the two reserved files.

## Correctness recheck

The current dirty candidate passed the focused guards I could run read-only:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-peer-dml-current-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-core test_direct_fixed_real_update_run_flushes_on_read_and_commit -- --nocapture
# ok: 1 passed

env CARGO_TARGET_DIR=/data/tmp/frankensqlite-peer-dml-current-target CARGO_BUILD_JOBS=10 cargo test -p fsqlite-btree test_table_overwrite_sorted_payloads_same_size_no_overflow -- --nocapture
# ok: 2 passed

git diff --check -- crates/fsqlite-btree/src/cursor.rs crates/fsqlite-core/src/connection.rs
# ok

env CARGO_TARGET_DIR=/data/tmp/frankensqlite-peer-dml-current-target CARGO_BUILD_JOBS=10 cargo fmt --check
# ok
```

## Focused isolated performance

Commands:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-dml-batch-current-review-target CARGO_BUILD_JOBS=10 cargo build -p fsqlite-e2e --bin perf-update-delete --bin comprehensive-bench --profile release-perf

/data/tmp/frankensqlite-dml-batch-current-review-target/release-perf/perf-update-delete 100 2000 both compare isolated
/data/tmp/frankensqlite-dml-batch-current-review-target/release-perf/perf-update-delete 1000 500 both compare isolated
/data/tmp/frankensqlite-dml-batch-current-review-target/release-perf/perf-update-delete 1000 500 both compare isolated
/data/tmp/frankensqlite-dml-batch-current-review-target/release-perf/perf-update-delete 10000 100 both compare isolated
```

| Rows | F update | C update | Update ratio | F delete | C delete | Delete ratio | Output |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 100 | `1079 ns/row` | `318 ns/row` | `3.39x` | `1203 ns/row` | `275 ns/row` | `4.38x` | `stdout/perf-update-delete-100.out` |
| 1000 | `1409 ns/row` | `333 ns/row` | `4.24x` | `1200 ns/row` | `276 ns/row` | `4.34x` | `stdout/perf-update-delete-1000.out` |
| 1000 repeat | `1381 ns/row` | `327 ns/row` | `4.22x` | `1235 ns/row` | `268 ns/row` | `4.60x` | `stdout/perf-update-delete-1000-repeat.out` |
| 10000 | `1636 ns/row` | `345 ns/row` | `4.75x` | `1364 ns/row` | `275 ns/row` | `4.97x` | `stdout/perf-update-delete-10000.out` |

Relevant prior isolated baseline from
`tests/artifacts/perf/update-delete-isolated-current-tanbear-20260507T1544Z/summary.md`:

| Rows | Baseline F update | Candidate F update | Baseline F delete | Candidate F delete |
| ---: | ---: | ---: | ---: | ---: |
| 100 | `788 ns/row` | `1079 ns/row` | `1233 ns/row` | `1203 ns/row` |
| 1000 | `913 ns/row` | `1409 ns/row` / `1381 ns/row` | `1209 ns/row` | `1200 ns/row` / `1235 ns/row` |
| 10000 | `916 ns/row` | `1636 ns/row` | `1328 ns/row` | `1364 ns/row` |

## Readout

This candidate is correctness-clean in the focused tests above, but the
isolated mutation kernel does not meet the first keep gate. UPDATE regresses at
all measured sizes, with the largest rows moving from roughly `916 ns/row` to
`1636 ns/row`; DELETE is at best noise-flat and slightly worse at 10K rows.

I did not run the broader `comprehensive-bench --quick --filter update` gate
because the isolated mutation gate already rejected the candidate, and the
reserved source files are owned by TanBear.
