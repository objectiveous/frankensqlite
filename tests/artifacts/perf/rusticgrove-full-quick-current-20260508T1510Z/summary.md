# Current-Head Full Quick Matrix - 2026-05-08

## Scope

Measured current `main` at `953959cbb2b495700c0737d155e6f7c84ce20acc`
with the peer concurrent-retry jitter commit included.

## Commands

```bash
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-rusticgrove-current-concurrent-20260508T1455Z \
cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick \
  --json-out tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/full-quick.json \
  --html tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/full-quick.html
```

```bash
FSQLITE_BENCH_PROFILE_DML=1 \
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-rusticgrove-current-concurrent-20260508T1455Z \
cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter update \
  --json-out tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/update-profile.json \
  --html tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/update-profile.html
```

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-rusticgrove-current-concurrent-20260508T1455Z \
cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter insert \
  --json-out tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/insert-profile.json \
  --html tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/insert-profile.html
```

## Full Quick Summary

`full-quick.json`:

- Total scenarios: `93`
- Faster/comparable/slower: `81/4/8`
- Average ratio: `0.4529512304`
- Geomean ratio: `0.2630086347`
- Primary weighted score: `0.3412469584`
- p90 ratio: `1.0160260530`
- p99 ratio: `1.3806107387`

Concurrent writers no longer have a C SQLite faster row in the full quick run:

| Scenario | C SQLite median ms | FrankenSQLite median ms | F/C time ratio |
|---|---:|---:|---:|
| 2 writers x 1000 rows | 13.947 | 13.943 | 1.000x |
| 4 writers x 1000 rows | 20.972 | 20.580 | 0.981x |
| 8 writers x 1000 rows | 92.460 | 42.270 | 0.457x |

## Current C-Faster Rows

| Section | Scenario | C SQLite ms | FrankenSQLite ms | F/C ratio |
|---|---|---:|---:|---:|
| UPDATE/DELETEThroughput | 100 rows / delete 5 rows | 0.082654 | 0.114113 | 1.381x |
| UPDATE/DELETEThroughput | 100 rows / update 10 rows | 0.088315 | 0.117801 | 1.334x |
| INSERT single transaction large_10col | 10000 rows | 9.512429 | 12.058057 | 1.268x |
| INSERT single transaction small_3col | 100 rows | 0.079018 | 0.096541 | 1.222x |
| INSERT transaction strategy small_3col | 100 rows / batched | 0.077545 | 0.087173 | 1.124x |
| INSERT record size comparison | large_10col 10K rows | 10.303201 | 11.359228 | 1.102x |
| INSERT single transaction medium_6col | 100 rows | 0.102141 | 0.108654 | 1.064x |
| INSERT transaction strategy small_3col | 100 rows / single txn | 0.078467 | 0.083406 | 1.063x |
| INSERT single transaction large_10col | 100 rows | 0.161433 | 0.167494 | 1.038x |
| INSERT single transaction tiny_1col | 100 rows | 0.070011 | 0.071133 | 1.016x |

The C-faster set is entirely write-side. Reads, mixed OLTP, joins, subqueries,
and string/pattern rows are faster than C SQLite in this run.

## Focused Profiles

`update-profile.json` repeated the UPDATE/DELETE section with
`FSQLITE_BENCH_PROFILE_DML=1`:

- Faster/comparable/slower: `4/0/2`
- Average/geomean: `1.0229139149` / `1.0051237372`
- Slow rows stayed at the 100-row tail.

The 100-row DML profile shows fixed setup/prepopulation dominates the row gap:

| Row | setup us | begin us | prepare us | mutate us | commit us |
|---|---:|---:|---:|---:|---:|
| update 10/100 | 52.1 | 6.9 | 13.2 | 12.0 | 5.7 |
| delete 5/100 | 56.2 | 5.0 | 11.4 | 8.3 | 5.2 |

`insert-profile.json` repeated the INSERT sections with
`FSQLITE_BENCH_PROFILE_INSERT=1`:

- Faster/comparable/slower: `17/2/6`
- Average/geomean: `0.8031423735` / `0.7802739991`
- Primary weighted score: `0.7888688743`

Representative slow-row profile notes:

- `tiny_1col` 100 rows: setup `16.3 us`, begin `10.0 us`, prepare `8.5 us`,
  insert `48.6 us`, commit `18.6 us`.
- `small_3col` 100 rows: setup `15.7 us`, begin `9.1 us`, prepare `10.1 us`,
  insert `56.5 us`, commit `10.5 us`.
- `large_10col` record-size 10K rows: row build `4.323 ms`, B-tree insert
  `0.872 ms`, commit roundtrip `2.519 ms`, page pool misses `2006`.

## Decision

No source patch was kept from this pass. The old low-thread concurrent gap is
stale on current `main`, and the remaining current gaps line up with areas the
negative-results ledger already fences heavily: small fixed write setup and the
large-row page-builder path. A keepable next source change should either remove
a shared fixed setup cost that helps the 100-row INSERT and DML rows together,
or implement a broader true page-builder design for large rows. Standalone
direct INSERT expression, WAL, right-edge hint, retained writer, or
page-run-admission micro-optimizations should not be retried.
