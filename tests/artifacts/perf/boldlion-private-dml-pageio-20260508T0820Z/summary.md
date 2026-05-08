# Private-Memory Direct DML Page-IO Context Candidate

Date: 2026-05-08
Agent: BoldLion
Source commit: `2cbaf1f2d6059fbef3ed77f7a2b730ff05326553`
Git dirty in reports: `true`

## Candidate

The dirty source candidate in `crates/fsqlite-core/src/connection.rs` added a
private-memory-only `direct_update_delete_page_io_context()` helper. For
`self.path == ":memory:" && self.pager.is_memory()`, prepared direct
UPDATE/DELETE skipped `SharedTxnPageIo` and used the active transaction cursor
directly. File-backed and non-private memory databases still used
`concurrent_page_io_context()`.

This is distinct from the rejected reusable `SharedTxnPageIo` shell/cache probes:
it avoids constructing the wrapper for private `:memory:` direct UPDATE/DELETE,
instead of trying to recycle the wrapper.

## Baselines

Relevant current full quick baseline after the small-record append landing:

- `tests/artifacts/perf/boldlion-small-record-append-20260508T0800Z/candidate-full-quick.json`

Relevant published focused DML baseline before the small-record append landing:

- `tests/artifacts/perf/windyibis-frontier-refresh-20260508T0745Z/head756-dml-profile.json`

The candidate was first compared to the focused DML baseline, then repeated
immediately because the 100-row DELETE row was noisy and regressed.

## First Focused Gate

Artifact: `candidate-update.json`

| Metric | Published focused DML baseline | Candidate |
| --- | ---: | ---: |
| Average ratio | `1.0893517744` | `1.0129453754` |
| Geomean ratio | `1.0564291964` | `0.9733515023` |
| P90/P99 ratio | `1.5942111048` | `1.7454228976` |
| Faster / comparable / slower | `4 / 0 / 2` | `5 / 0 / 1` |

Rows:

| Row | C ms | F ms | Ratio | F CV% |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | `0.133891` | `0.112751` | `0.842110` | `8.24` |
| 100 rows / delete 5 rows | `0.078707` | `0.137377` | `1.745423` | `52.82` |
| 1000 rows / update 100 rows | `0.416139` | `0.381535` | `0.916845` | `6.22` |
| 1000 rows / delete 50 rows | `0.420378` | `0.356819` | `0.848805` | `28.32` |
| 10000 rows / update 1000 rows | `3.673822` | `3.187061` | `0.867506` | `0.81` |
| 10000 rows / delete 500 rows | `3.440596` | `2.948533` | `0.856983` | `0.30` |

## Immediate Repeat

Artifact: `candidate-update-repeat.json`

| Metric | Candidate repeat |
| --- | ---: |
| Average ratio | `1.1308286150` |
| Geomean ratio | `1.1042288976` |
| P90/P99 ratio | `1.6349627785` |
| Faster / comparable / slower | `1 / 3 / 2` |

Rows:

| Row | C ms | F ms | Ratio | F CV% |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | `0.097042` | `0.127008` | `1.308794` | `8.62` |
| 100 rows / delete 5 rows | `0.078718` | `0.128701` | `1.634963` | `54.85` |
| 1000 rows / update 100 rows | `0.385983` | `0.371075` | `0.961377` | `2.87` |
| 1000 rows / delete 50 rows | `0.381534` | `0.340488` | `0.892419` | `6.52` |
| 10000 rows / update 1000 rows | `3.482334` | `3.447729` | `0.990063` | `0.86` |
| 10000 rows / delete 500 rows | `3.400360` | `3.391373` | `0.997357` | `10.25` |

## Decision

Rejected. The first run looked promising on geomean but worsened the 100-row
DELETE tail. The immediate repeat failed the focused DML gate outright:
average/geomean regressed, both 100-row rows were slower than C SQLite, and the
larger rows collapsed back toward parity.

Do not retry this bypass as a standalone change. A future attempt should batch
same-leaf UPDATE/DELETE work so setup and mutation costs move together, then
pass repeated focused UPDATE/DELETE gates and a full quick matrix.

## Verification

Commands run:

```text
cargo fmt -p fsqlite-core --check
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-private-dml-check-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_entry_proof_no_publication_for_memory_update_delete -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-boldlion-private-dml-check-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/cargo-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete
env FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/cargo-target/release-perf/comprehensive-bench --quick --filter update --no-html --json-out tests/artifacts/perf/boldlion-private-dml-pageio-20260508T0820Z/candidate-update.json
env FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/cargo-target/release-perf/comprehensive-bench --quick --filter update --no-html --json-out tests/artifacts/perf/boldlion-private-dml-pageio-20260508T0820Z/candidate-update-repeat.json
```

Results:

- Formatting passed.
- Both focused correctness tests passed.
- Release-perf benchmark build passed.
- Focused UPDATE/DELETE repeat rejected the candidate.
