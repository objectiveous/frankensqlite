# Update/Delete No-Retry Harness Candidate

Date: 2026-05-17

Source: `6b4181415c1e1a38c013b895cdca5f8ace522aaa` plus dirty DML profiling
counter patch and the temporary `comprehensive_bench.rs` no-retry candidate.

Command shape:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fresh-eyes-20260517i cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update-delete --json-out tests/artifacts/perf/codex-update-delete-no-retry-harness-20260517T1620Z/candidate-update-delete.json --no-html
```

Result: rejected and unwound uncommitted.

The candidate removed the benchmark-level BusySnapshot/Busy retry wrapper from
single-connection `:memory:` FrankenSQLite UPDATE/DELETE setup, measurement, and
teardown calls. That looked plausible as a measurement correction because these
rows are deterministic and should not need busy retry accounting. The focused
same-window gate did not support keeping it: weighted/geomean UPDATE/DELETE
score moved from `1.488025550482293` to `1.5030257550239214`.

Row-level movement was mixed. The 100-row DELETE tail improved from
`F=8.285 us` (`3.579x`) to `F=7.394 us` (`3.237x`), and 10K UPDATE improved
from `F=276.618 us` (`0.761x`) to `F=263.823 us` (`0.720x`). Those gains were
offset by 1000-row DELETE regressing from `F=29.876 us` (`1.886x`) to
`F=31.289 us` (`1.960x`) and 10K DELETE regressing from `F=269.745 us`
(`1.641x`) to `F=316.122 us` (`1.928x`).

Retry-wrapper overhead is therefore not a stable standalone explanation for the
remaining focused UPDATE/DELETE gap. Revisit only if the benchmark harness grows
a broader paired C/F retry-accounting design and the focused plus full quick
matrix both improve in the same run window.
