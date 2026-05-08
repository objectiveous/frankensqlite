# Fixed-width REAL update page-local payload patch

Date: 2026-05-08
Agent: SwiftGate
Decision: rejected, source restored

## Candidate

Added a hidden B-tree primitive that mutates the current local table payload as
a page-resident slice, then routed the fixed-width REAL direct UPDATE fast path
through it. The intended win was to avoid `payload_into` and the full same-size
payload copy back through `table_overwrite_current_payload_same_size_no_overflow`
for `UPDATE bench SET value = ?2 WHERE id = ?1`.

Touched during the scratch candidate:

- `crates/fsqlite-btree/src/cursor.rs`
- `crates/fsqlite-core/src/connection.rs`

The patch was restored after measurement.

## Correctness Proof

```text
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-swiftgate-payload-patch-btree CARGO_BUILD_JOBS=8 cargo test -p fsqlite-btree test_table_mutate_current_payload_same_size_no_overflow_patches_local_payload -- --nocapture
```

Passed: 1 B-tree test.

```text
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-swiftgate-payload-patch-core CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_single_real_column_patches_payload_without_decode -- --nocapture --test-threads=1
```

Passed: 1 direct UPDATE regression test.

## Measurements

Release-perf build:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-swiftgate-payload-patch-bench CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete
```

Isolated candidate probe:

```text
/data/tmp/frankensqlite-swiftgate-payload-patch-bench/release-perf/perf-update-delete 100 20000 update fsqlite isolated
```

Result: `636 ns/update`.

Focused matrix:

| Run | Avg | Geomean | P90 | P99 | F/C/C-slower |
| --- | ---: | ---: | ---: | ---: | --- |
| restored clean | `1.0410476491` | `1.0255034389` | `1.3191885268` | `1.3191885268` | `3 / 1 / 2` |
| candidate | `1.1541475349` | `1.1366021497` | `1.4638185577` | `1.4638185577` | `0 / 3 / 3` |
| candidate repeat | `1.0661982520` | `1.0529987672` | `1.3262300030` | `1.3262300030` | `2 / 2 / 2` |

The restored clean source beat both candidate runs on aggregate score and tail,
so the candidate failed the keep gate.

## Artifacts

- `candidate-isolated-update.txt`
- `candidate-update.json`
- `candidate-update-repeat.json`
- `clean-update.json`
- `stdout/`
