# update-delete-isolated-current-tanbear-20260507T1544Z

Dedicated `perf-update-delete` isolated comparison on current `main` after
`bd067912`.

This run was taken to separate the UPDATE/DELETE mutation core from the
`comprehensive-bench` Section 6 setup/prepopulation work.

## Commands

```bash
rch exec -- env \
  TMPDIR=/data/tmp/frankensqlite-tanbear-tmp3 \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-update-delete-isolated-target \
  CARGO_BUILD_JOBS=16 \
  cargo build -p fsqlite-e2e --bin perf-update-delete --profile release-perf

/data/tmp/frankensqlite-update-delete-isolated-target/release-perf/perf-update-delete \
  100 2000 both compare isolated

/data/tmp/frankensqlite-update-delete-isolated-target/release-perf/perf-update-delete \
  1000 500 both compare isolated

/data/tmp/frankensqlite-update-delete-isolated-target/release-perf/perf-update-delete \
  10000 100 both compare isolated
```

## Results

| Rows | F update | C update | Update ratio | F delete | C delete | Delete ratio | Populate ratio |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | `788 ns/row` | `314 ns/row` | `2.51x` | `1233 ns/row` | `290 ns/row` | `4.25x` | `0.91x` |
| 1000 | `913 ns/row` | `341 ns/row` | `2.68x` | `1209 ns/row` | `282 ns/row` | `4.29x` | `1.06x` |
| 10000 | `916 ns/row` | `344 ns/row` | `2.66x` | `1328 ns/row` | `279 ns/row` | `4.76x` | `1.05x` |

## Interpretation

The comprehensive Section 6 gaps are not only setup noise. In this isolated
harness, prepopulation is near parity, but direct UPDATE remains about
`2.5-2.7x` slower per row and direct DELETE about `4.2-4.8x` slower per row.

This confirms a real mutation-core gap. The obvious local retries are already
fenced in `docs/progress/perf-negative-results.md`: schema-proof carry,
scratch-reset removal, fixed-width REAL patching, `SharedTxnPageIo` reuse,
table-seek hints, direct-DML buffering, scan-merge flushing, and staged-page
marker/published-page tweaks.

The next viable DML idea should avoid those standalone shapes. Based on this
run and the ledger, the remaining high-EV direction is a true retained-cursor
or bulk same-page mutation kernel that improves both isolated 1K/10K rows and
the real `comprehensive-bench --quick --filter update` Section 6 gate.
