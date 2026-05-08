# Current-Head Concurrent Gate - 2026-05-08

## Scope

Measured current `main` at `953959cbb2b495700c0737d155e6f7c84ce20acc`
after `fix(e2e): stagger concurrent benchmark retries`.

This supersedes the earlier clean profile at `e305a172` for gate decisions.
The earlier profile remains useful for CPU attribution, but the current
benchmark harness no longer has a C-faster concurrent row.

## Command

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-rusticgrove-current-concurrent-20260508T1455Z \
cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --filter concurrent \
  --json-out tests/artifacts/perf/rusticgrove-concurrent-wal-profile-20260508T1455Z/concurrent-profile.json \
  --html tests/artifacts/perf/rusticgrove-concurrent-wal-profile-20260508T1455Z/concurrent-profile.html
```

## Results

| Scenario | C SQLite median ms | FrankenSQLite median ms | F/C time ratio |
|---|---:|---:|---:|
| 2 writers x 1000 rows | 14.646 | 14.328 | 0.978x |
| 4 writers x 1000 rows | 21.338 | 20.324 | 0.952x |
| 8 writers x 1000 rows | 92.786 | 42.353 | 0.456x |

Summary: `1/2/0` faster/comparable/slower, average ratio `0.796x`,
geomean/primary `0.752x`.

## Decision

Do not start a new concurrent-writer source patch from the older `e305a172`
gap. On current `main`, the concurrent benchmark no longer has a C SQLite
faster row. The next optimization target should come from the current full
quick matrix rather than the stale low-thread concurrent row.
