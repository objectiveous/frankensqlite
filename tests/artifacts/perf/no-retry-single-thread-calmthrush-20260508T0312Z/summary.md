# Single-Thread Busy-Retry Wrapper Probe

Date: 2026-05-08 03:12Z
Agent: CalmThrush
Head: `6726103a docs(perf): map direct insert row template`

## Question

The non-concurrent rows in `comprehensive-bench` run FrankenSQLite calls
through `retry_on_busy`, while the comparable C SQLite rows execute directly.
This artifact checks whether that wrapper is enough to explain the remaining
small `UPDATE/DELETEThroughput` gap.

No source was changed for this probe.

## Build

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-update-delete-calmthrush-target \
  CARGO_BUILD_JOBS=16 \
  cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

## Runs

```text
/data/tmp/frankensqlite-perf-update-delete-calmthrush-target/release-perf/perf-update-delete \
  100 50 both compare standard

/data/tmp/frankensqlite-perf-update-delete-calmthrush-target/release-perf/perf-update-delete \
  1000 30 both compare standard
```

Stdout/stderr are under `stdout/`.

## Result

The retry wrapper is not the whole problem. The narrow binary runs the same
standard workload without the comprehensive harness wrapper and still shows the
direct mutation kernel behind C SQLite:

| Rows | F total | C total | Populate ratio | Update ratio | Delete ratio |
| ---: | ---: | ---: | ---: | ---: | ---: |
| `100` | `9 ms` | `4 ms` | `1.06x` | `3.67x` | `5.04x` |
| `1000` | `16 ms` | `12 ms` | `0.76x` | `2.94x` | `4.57x` |

This supports keeping future UPDATE/DELETE work focused on direct DML execution
or broader setup/prepopulation removal. It does not justify a harness-only
single-thread no-retry rewrite, especially while
`crates/fsqlite-e2e/src/bin/comprehensive_bench.rs` is reserved by another
agent.
