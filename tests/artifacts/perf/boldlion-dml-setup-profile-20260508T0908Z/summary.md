# DML Setup and Mutation Profile

Date: 2026-05-08
Agent: BoldLion
HEAD: `a13ebebdc81aa7a47a59987d991d3f5c4a8fce90`

## Scope

Fresh profile of the current `UPDATE/DELETEThroughput` frontier after the
scratch-borrow and private-DML bypass rejections, followed by one measured
direct REAL assignment shortcut candidate. The candidate source edit was
restored after the repeated focused gate rejected it.

The release-perf binary used here was built before the artifact-only `a13ebebd`
commit; source code is unchanged from the measured binary, but the benchmark
stdout still reports the binary-predates-HEAD warning.

While the candidate was being measured, the artifact-only `073f6dd7` frontier
repeat commit landed on `main`. It added perf artifacts only; the candidate
still touched only `crates/fsqlite-core/src/connection.rs` and was restored.

## Commands

```text
env FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-boldlion-dml-current-target/release-perf/comprehensive-bench --quick --filter update --no-html --json-out tests/artifacts/perf/boldlion-dml-setup-profile-20260508T0908Z/current-dml-profile.json

/data/tmp/frankensqlite-boldlion-dml-current-target/release-perf/perf-update-delete 100 200 both compare standard

perf record -F 1999 -g --call-graph dwarf -o tests/artifacts/perf/boldlion-dml-setup-profile-20260508T0908Z/perf-update-100-long.data -- /data/tmp/frankensqlite-boldlion-dml-current-target/release-perf/perf-update-delete 100 20000 update fsqlite isolated
perf report --stdio --no-children --sort=dso,symbol -i tests/artifacts/perf/boldlion-dml-setup-profile-20260508T0908Z/perf-update-100-long.data

env FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-boldlion-real-assignment-bench-target/release-perf/comprehensive-bench --quick --filter update --no-html --json-out tests/artifacts/perf/boldlion-dml-setup-profile-20260508T0908Z/candidate-real-assignment-dml.json

env FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-boldlion-real-assignment-bench-target/release-perf/comprehensive-bench --quick --filter update --no-html --json-out tests/artifacts/perf/boldlion-dml-setup-profile-20260508T0908Z/candidate-real-assignment-dml-repeat2.json

/data/tmp/frankensqlite-boldlion-real-assignment-bench-target/release-perf/perf-update-delete 100 20000 update fsqlite isolated
```

## Results

Focused `comprehensive-bench --quick --filter update` still has the same two
slow rows:

| Scenario | C SQLite | FrankenSQLite | Ratio |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 97.7 us | 128.9 us | 1.32x |
| 100 rows / delete 5 rows | 80.6 us | 111.6 us | 1.39x |
| 1000 rows / update 100 rows | 397.2 us | 398.9 us | 1.00x |
| 1000 rows / delete 50 rows | 399.9 us | 364.8 us | 1.10x faster |
| 10000 rows / update 1000 rows | 3.61 ms | 3.49 ms | 1.04x faster |
| 10000 rows / delete 500 rows | 3.45 ms | 3.15 ms | 1.10x faster |

Profile lines for the 100-row tails:

| Scenario | setup_us | begin_us | prepare_us | mutate_us | commit_us | direct DML |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| update 10/100 | 73.3 | 11.8 | 14.4 | 13.0 | 6.3 | 10 direct updates |
| delete 5/100 | 53.1 | 4.9 | 11.9 | 8.2 | 4.9 | 5 direct deletes |

The isolated standard `perf-update-delete 100 200 both compare standard`
showed mutation remains slower even when setup is separated:

| Engine | populate | update | delete | per-row update | per-row delete |
| --- | ---: | ---: | ---: | ---: | ---: |
| FrankenSQLite | 5 ms | 2 ms | 1 ms | 1442 ns | 1891 ns |
| C SQLite | 5 ms | 0 ms | 0 ms | 430 ns | 430 ns |

The longer `perf record` on isolated 100-row UPDATE captured 380 samples with
FrankenSQLite at `656 ns/update`. Top self symbols included:

| Self | Symbol |
| ---: | --- |
| 9.95% | `__memmove_avx_unaligned_erms` |
| 7.43% | `Connection::execute_prepared_direct_simple_update` |
| 4.75% | `_int_malloc` |
| 4.53% | `BtCursor<SharedTxnPageIo>::table_seek_for_insert` |
| 3.14% | `BtCursor<SharedTxnPageIo>::load_page` |
| 2.32% | `table_overwrite_current_payload_same_size_no_overflow` |
| 2.22% | `parse_record_projected_column_offsets` |
| 2.16% | `SharedTxnPageIo::with_concurrent` |
| 1.98% | `CellSlotCache::insert_slow` |

The direct REAL assignment shortcut did not meet the keep gate. It bypassed
generic `SqliteValue::apply_affinity` for already-numeric RHS values in the
fixed-width REAL patch path, then restored the source after measurement.

| Run | Avg ratio | Geomean | p90/p99 | Notable row |
| --- | ---: | ---: | ---: | --- |
| Baseline focused DML | 1.0830 | 1.0667 | 1.3856 | 100-row update `128.9 us`, ratio `1.3194x` |
| Candidate repeat 1 | 1.0896 | 1.0664 | 1.4365 | 1000-row update `1.1296x`, `17.6%` F CV |
| Candidate repeat 2 | 1.1043 | 1.0843 | 1.4356 | 10000-row update regressed to `1.0462x` |

The isolated update loop did move in the intended direction:
`perf-update-delete 100 20000 update fsqlite isolated` improved from
`656 ns/update` to `624 ns/update`. That micro win was not enough to keep the
source edit because the repeated filtered matrix worsened.

## Readout

The profile confirms two facts that matter for the next pass:

1. The full 100-row rows are setup-heavy, so broad direct-INSERT/setup work can
   move them, but the negative ledger already fences many direct INSERT
   ceremony and serializer candidates.
2. The isolated mutation loop is still materially slower than C SQLite, but the
   top sampled costs mostly map to already rejected no-retry shapes:
   `SharedTxnPageIo` bypass/reuse, retained cursor or leaf hints, staged-page or
   uncached overwrite, fixed REAL payload patching, and cell-slot cache churn.

The measured direct REAL assignment shortcut is now fenced in
`docs/progress/perf-negative-results.md`: do not retry it as a standalone
optimization. The next plausible direction remains a broader DML batch or
leaf-run operator that removes larger per-row mutation work and wins repeated
focused gates plus the full quick matrix.
