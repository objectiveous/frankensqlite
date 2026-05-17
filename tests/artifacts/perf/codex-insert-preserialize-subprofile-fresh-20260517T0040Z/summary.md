# INSERT Preserialize Subprofile Refresh

Date: 2026-05-17 04:45:51 UTC

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
CARGO_TARGET_DIR=/data/tmp/frankensqlite-insert-preserialize-subprofile-fresh \
cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter insert \
  --json-out tests/artifacts/perf/codex-insert-preserialize-subprofile-fresh-20260517T0040Z/insert.json \
  --no-html
```

Source: `main @ 6b4181415c1e1a38c013b895cdca5f8ace522aaa`, dirty with
profiling-only preserialize subcounters.

Host/toolchain: Ubuntu 25.10, Linux 6.17.0-19-generic, AMD Ryzen Threadripper
PRO 5995WX 64-Cores, 499.3 GB RAM, nightly rustc 1.97.0-nightly
`d7f14d3d8`, `release-perf`.

Artifacts:

- `insert.json`
- `run.log`

## Matrix Result

The focused INSERT matrix covered 25 scenarios:

- FrankenSQLite faster: 15
- Comparable: 4
- C SQLite faster: 6
- Average F/C time ratio: 0.920x
- Geomean F/C time ratio: 0.900x
- Median F/C time ratio: 0.879x

Remaining rows slower than C SQLite by more than 5 percent:

| Scenario | C SQLite | FrankenSQLite | F/C |
|---|---:|---:|---:|
| single txn tiny_1col 100 rows | 0.071 ms | 0.075 ms | 1.06x |
| single txn small_3col 100 rows | 0.076 ms | 0.090 ms | 1.18x |
| single txn medium_6col 100 rows | 0.104 ms | 0.131 ms | 1.26x |
| single txn large_10col 100 rows | 0.174 ms | 0.191 ms | 1.10x |
| small_3col 100 rows batched 100/txn | 0.074 ms | 0.106 ms | 1.42x |
| small_3col 100 rows single txn | 0.081 ms | 0.088 ms | 1.10x |

Large 10K-row rows are not a stable current red frontier in this run:

- single-txn large_10col 10000 rows: C SQLite 10.238 ms,
  FrankenSQLite 10.156 ms, F/C 0.99x.
- record-size large_10col 10000 rows: C SQLite 10.424 ms,
  FrankenSQLite 10.643 ms, F/C 1.02x.

## Preserialize Subprofile

The `preserialize_*` counters are attribution counters from an additional
profile-only run. They add nested `Instant` calls and should not be interpreted
as wall-clock-equivalent benchmark time. Use them to rank work inside the direct
record preserializer.

| Profile row | row_build | preserialize | cell | eval | affinity | layout | encode | direct_flush | page misses |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| single_txn small_3col 10000 | 7.433 ms | 6.816 ms | 5.538 ms | 1.153 ms | 0.515 ms | 0.759 ms | 0.348 ms | 0.605 ms | 64 |
| single_txn medium_6col 10000 | 12.818 ms | 12.214 ms | 10.704 ms | 2.038 ms | 1.259 ms | 1.528 ms | 0.586 ms | 0.855 ms | 455 |
| single_txn large_10col 10000 | 21.397 ms | 20.789 ms | 19.049 ms | 4.358 ms | 2.242 ms | 2.547 ms | 0.799 ms | 2.461 ms | 2004 |
| record_size small_3col 10000 | 7.213 ms | 6.619 ms | 5.339 ms | 1.055 ms | 0.535 ms | 0.745 ms | 0.372 ms | 0.301 ms | 64 |
| record_size medium_6col 10000 | 12.826 ms | 12.225 ms | 10.722 ms | 2.055 ms | 1.259 ms | 1.541 ms | 0.553 ms | 0.773 ms | 455 |
| record_size large_10col 10000 | 22.634 ms | 22.023 ms | 20.135 ms | 4.532 ms | 2.394 ms | 2.678 ms | 0.895 ms | 2.742 ms | 2004 |

## Decision

Do not start another standalone large-record serializer tweak from this artifact.
The current measured INSERT frontier is the fixed-cost 100-row tail, not the
large 10K-row shape by itself. If large-row INSERT regresses again on a full
matrix refresh, the next admissible design must be a broader fused
row/body/page construction pass, not isolated capacity tuning, scratch reuse,
or page-run mechanics already fenced in `docs/progress/perf-negative-results.md`.

Alien-graveyard fit for future work:

- Vectorized execution and AMAC-style interleaving need a batched interface; the
  direct INSERT path already has batch-shaped evidence, but single 100-row tails
  are more fixed-cost dominated than memory-latency dominated.
- B-epsilon/write-buffer ideas fit page-run/index write amplification, but this
  artifact does not show page-run mechanics as the next narrow lever.
- The highest-EV next step is a fixed-cost profile of 100-row INSERT setup,
  begin/commit, schema validation, and benchmark/harness overhead before any
  source optimization.
