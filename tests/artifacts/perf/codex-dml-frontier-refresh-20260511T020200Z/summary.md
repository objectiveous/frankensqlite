# DML Frontier Refresh

- Run date: 2026-05-11T02:07:30Z
- Commit: `1567cae3c83eaf300e50faf5ca2ee02b156f81c4`
- Command:
  `FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-codex-fresh-eyes/release-perf/comprehensive-bench --quick --filter update --json-out tests/artifacts/perf/codex-dml-frontier-refresh-20260511T020200Z/update-delete.json --no-html`
- Purpose: fresh-eyes confirmation after the INSERT row-build profile split, before attempting any further `bd-db300.11.1` source patch.

## Section Result

- Scenarios: 6
- FSQLite faster / comparable / C SQLite faster: 2 / 0 / 4
- Average ratio: 1.842686991124011
- Geomean ratio: 1.6029243318693236
- Median ratio: 2.0532770586650586
- p90 ratio: 3.620842572062084
- p99 ratio: 3.620842572062084
- Weighted score: 1.6029243318693234

## Rows

| Scenario | Ratio | FSQLite ms | C SQLite ms |
|---|---:|---:|---:|
| 100 rows / update 10 rows | 1.6411174785100286 | 0.006873 | 0.004188000000000001 |
| 100 rows / delete 5 rows | 3.620842572062084 | 0.008165 | 0.002255 |
| 1000 rows / update 100 rows | 0.8777430334560146 | 0.031719 | 0.036137 |
| 1000 rows / delete 50 rows | 2.1138801464448584 | 0.032910999999999996 | 0.015569 |
| 10000 rows / update 1000 rows | 0.7492616576060205 | 0.273993 | 0.365684 |
| 10000 rows / delete 500 rows | 2.0532770586650586 | 0.32011 | 0.155902 |

## DELETE Profile Signals

| Profile Row | Fast / Slow | Leaf Start | Leaf Active | Leaf Miss | Leaf Flush | Leaf Flush ns | Materialize ns | Write ns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `fs_delete_100` | 5 / 0 | 1 / 1 | 4 / 4 | 0 | 1 / 1 | 2124 | 1162 | 651 |
| `fs_delete_1000` | 50 / 0 | 6 / 6 | 44 / 49 | 5 | 6 / 6 | 23332 | 19195 | 2875 |
| `fs_delete_10000` | 500 / 0 | 64 / 67 | 433 / 496 | 63 | 64 / 64 | 110481 | 73555 | 25601 |

## Interpretation

The latest focused run revalidates the existing frontier: prepared direct DELETE is active (`slow=0`), and the 500-row row still spends visible time across many leaf-run starts, active mutations, and per-leaf materialization/publication. `direct_flush_ns` remains tiny compared with the leaf-run work.

No source patch was attempted from this pass. The safe next patch is still a transaction-local DML mutation operator with a logical key-space proof envelope, not another standalone `TableLeafDeleteRun` micro-optimization, direct-flush wrapper, page-boundary admission tweak, tombstone-only overlay, or dense-rowid queue.
