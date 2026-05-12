# Small UPDATE Transaction-Envelope Rescreen

- Date: 2026-05-12
- Git: `a19571f8f88e8023274f1a8d3775189c90565b45`
- Target: `UPDATE/DELETEThroughput` `100 rows / update 10 rows`

## Commands

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-update-fixed CARGO_BUILD_JOBS=8 FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --json-out tests/artifacts/perf/codex-update-fixed-overhead-20260512T091030Z/baseline-update.json --no-html
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-update-fixed CARGO_BUILD_JOBS=8 cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 100 20000 update compare standard
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-update-fixed CARGO_BUILD_JOBS=8 cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- 100 20000 update compare isolated
```

## Result

The focused matrix still reports the 100-row UPDATE row slower than C SQLite:
`1.378x` F/C (`4.107us` C SQLite, `5.661us` FrankenSQLite). The larger UPDATE
rows are still green: `0.758x` for 1000 rows and `0.642x` for 10000 rows.

The narrow harness separates mutation from transaction/setup envelope:

- Standard mode: `719ns/update` FrankenSQLite versus `424ns/update` C SQLite.
- Isolated mode: `119ns/update` FrankenSQLite versus `285ns/update` C SQLite.

Conclusion: the remaining small UPDATE red row is not a physical mutation or
payload-patch problem. Another standalone direct UPDATE mutation tweak is not a
credible next lever; the retry condition is a transaction-envelope or
release-boundary redesign that preserves the isolated mutation win and improves
the full quick primary score.
