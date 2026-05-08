# Concurrent Writer Refresh

Date: 2026-05-08

Head recorded by the benchmark reports:
`2e041edf9e7a1d6c910a874b79ddc532d3015310`.

The benchmark binary reported `benchmark_binary_older_than_git_head=true`
because the source binary had been built before the artifact-only
`docs(perf): publish current post-dml baseline` commit. The benchmark reports
also recorded `git_dirty=false` at run time.

Command shape:

```text
/data/tmp/frankensqlite-current-post-dml-target/release-perf/comprehensive-bench \
  --quick --filter concurrent --no-html \
  --json-out tests/artifacts/perf/concurrent-refresh-tanbear-20260508T0128Z/<report>.json
```

Reports:

- `concurrent-head-rebuilt.json`
- `concurrent-head-repeat2.json`
- `concurrent-head-repeat3.json`

## Results

| Report | Scenario | Ratio F/C | FSQLite ms | C SQLite ms | F CV % | C CV % |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `concurrent-head-rebuilt` | 2 writers x 1000 rows | `1.050448` | `13.302214` | `12.663378` | `3.90` | `30.19` |
| `concurrent-head-rebuilt` | 4 writers x 1000 rows | `0.929404` | `18.960202` | `20.400390` | `6.59` | `10.14` |
| `concurrent-head-rebuilt` | 8 writers x 1000 rows | `0.421615` | `38.778531` | `91.976109` | `45.73` | `9.44` |
| `concurrent-head-repeat2` | 2 writers x 1000 rows | `1.021409` | `13.569876` | `13.285443` | `5.79` | `19.69` |
| `concurrent-head-repeat2` | 4 writers x 1000 rows | `1.002415` | `20.579826` | `20.530253` | `2.71` | `14.53` |
| `concurrent-head-repeat2` | 8 writers x 1000 rows | `0.425326` | `39.134037` | `92.009421` | `55.31` | `3.12` |
| `concurrent-head-repeat3` | 2 writers x 1000 rows | `1.131596` | `14.606648` | `12.908006` | `8.89` | `6.08` |
| `concurrent-head-repeat3` | 4 writers x 1000 rows | `0.999885` | `20.357058` | `20.359403` | `14.87` | `14.25` |
| `concurrent-head-repeat3` | 8 writers x 1000 rows | `0.389204` | `35.937781` | `92.336544` | `66.58` | `21.54` |

The repeated 2-writer row ranged from `1.021409x` to `1.131596x`; the
same-window median of the three ratios is about `1.050448x`. The 4-writer row
was effectively parity to faster, and the 8-writer row remained much faster
than C SQLite but noisy.

## Interpretation

The remaining concurrent row is small and noisy compared with the INSERT and
small DML gaps. Earlier ledgered probes already reject standalone concurrent
benchmark runtime/thread-launch reuse and direct `std::thread` harness changes,
because they helped C SQLite as much or more than FrankenSQLite on the low
thread-count row. Earlier SharedTxnPageIo context and page-size/cache probes
are also rejected for adjacent write paths.

Do not start another standalone concurrent-runtime or `SharedTxnPageIo`
micro-probe from these numbers. Revisit the 2-writer row only with a current
profile showing engine self-time, not harness setup, and keep a candidate only
if the 2-writer row and the full quick matrix improve in the same A/B window.
