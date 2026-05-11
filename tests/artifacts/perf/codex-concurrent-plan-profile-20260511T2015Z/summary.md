# Concurrent Commit Plan Profile

- Date: 2026-05-11
- Build:
  `rch exec -- env CARGO_TARGET_DIR=/tmp/frankensqlite-codex-concurrent-plan-profile-bench CARGO_BUILD_JOBS=4 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
- Run:
  `FSQLITE_BENCH_PROFILE_CONCURRENT=1 /tmp/frankensqlite-codex-concurrent-plan-profile-bench/release-perf/comprehensive-bench --quick --filter concurrent --json-out tests/artifacts/perf/codex-concurrent-plan-profile-20260511T2015Z/concurrent.json --no-html`

## Evidence

| Row | ratio F/C | plan attempts | successes | BusySnapshot plan errors | pending pages | write pages | uncontended fast path | full validation | write-time stale retries |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2 writers x 1000 rows | 1.2453 | 36 | 36 | 0 | 192 | 192 | 26 | 10 | 10 |
| 4 writers x 1000 rows | 1.1628 | 70 | 60 | 10 | 498 | 408 | 24 | 36 | 72 |
| 8 writers x 1000 rows | 0.6468 | 142 | 108 | 34 | 1290 | 900 | 28 | 80 | 321 |

## Readout

The 2-thread red row is not currently dominated by commit-plan FCW rejection:
all 36 plan attempts succeeded, and 26 used the uncontended fast path. Its
remaining retries are write-time stale snapshot/page-lock retries during the
INSERT body.

The 4- and 8-thread rows do have commit-plan rejections, but they are already
winning or near the crossover point. This keeps the next optimization target on
the write-body conflict path or a broader transaction-local page construction
primitive, not on standalone commit-plan filtering.
