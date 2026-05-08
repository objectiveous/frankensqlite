# Clean No-Profile Full Quick Baseline

Date: 2026-05-08

Clean detached worktree:
`/data/tmp/frankensqlite-clean-noprofile-calmthrush-20260508T0219Z`

Head:
`0e68aeac088b72cb64bd636890d007b2ee758fd5`
(`docs(perf): publish no-profile current baseline`)

This replaces
`tests/artifacts/perf/calmthrush-current-noprofile-20260508T0212Z/summary.md`
for target ordering. The earlier no-profile artifact was built from the shared
checkout while an unowned `crates/fsqlite-core/src/connection.rs` diff was
present; this one was built from a clean detached worktree.

## Commands

Build:

```text
env TMPDIR=/data/tmp/frankensqlite-calmthrush-tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-clean-noprofile-target \
  CARGO_BUILD_JOBS=8 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Run:

```text
/data/tmp/frankensqlite-clean-noprofile-target/release-perf/comprehensive-bench \
  --quick --no-html \
  --json-out tests/artifacts/perf/calmthrush-clean-noprofile-20260508T0219Z/full-quick-clean-noprofile.json
```

## Summary

- Total scenarios: `93`
- FrankenSQLite faster: `80`
- Comparable: `5`
- C SQLite faster: `8`
- Average ratio: `0.4542606463918878`
- Geomean ratio: `0.2674752493298549`
- P90 ratio: `0.9811588214938469`
- P99 ratio: `1.4015153360781543`
- Primary weighted score: `0.34593878641661835`

Rows still above `1.05x`:

| Ratio | Section | Scenario | Category | FSQLite ms | C SQLite ms |
| ---: | --- | --- | --- | ---: | ---: |
| `1.401515` | UPDATE/DELETEThroughput | 100 rows / delete 5 rows | write_single | `0.115056` | `0.082094` |
| `1.398978` | UPDATE/DELETEThroughput | 100 rows / update 10 rows | write_single | `0.119123` | `0.085150` |
| `1.192042` | INSERTThroughput - Single Transaction - medium_6col | 100 rows | write_bulk | `0.125435` | `0.105227` |
| `1.127685` | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / batched (100/txn) | write_bulk | `0.084679` | `0.075091` |
| `1.113234` | INSERTThroughput - Transaction Strategy Comparison (small_3col) | 100 rows / single txn | write_bulk | `0.083025` | `0.074580` |
| `1.110156` | INSERTThroughput - Single Transaction - large_10col | 100 rows | write_bulk | `0.163577` | `0.147346` |
| `1.097814` | INSERTThroughput - Single Transaction - small_3col | 100 rows | write_bulk | `0.085130` | `0.077545` |
| `1.088204` | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 2 writers x 1000 rows | concurrent_writers | `13.499477` | `12.405278` |

Near-threshold rows:

| Ratio | Section | Scenario | Category | FSQLite ms | C SQLite ms |
| ---: | --- | --- | --- | ---: | ---: |
| `1.009610` | Concurrent Writers - C SQLite WAL vs FrankenSQLite MVCC | 4 writers x 1000 rows | concurrent_writers | `19.788805` | `19.600442` |
| `0.981159` | INSERTThroughput - Record Size Comparison (10K rows, single txn) | large_10col - 10 cols (~600B) | write_bulk | `9.501231` | `9.683683` |
| `0.968933` | INSERTThroughput - Single Transaction - medium_6col | 10000 rows | write_bulk | `6.018099` | `6.211060` |
| `0.959842` | UPDATE/DELETEThroughput | 1000 rows / delete 50 rows | write_single | `0.361106` | `0.376214` |
| `0.959238` | UPDATE/DELETEThroughput | 1000 rows / update 100 rows | write_single | `0.385251` | `0.401622` |

## Interpretation

Use this artifact as the current no-profile target map. The top true gaps are
the 100-row UPDATE/DELETE rows, both dominated by fixed setup and statement
ceremony in earlier focused profiles. The next cluster is 100-row prepared
direct INSERT fixed cost; the large 10K record-size INSERT rows are now below
parity in this clean no-profile run.

The 2-writer concurrent row remains above C SQLite, but the repeated refresh
artifact already shows that row is noisy and lower-weight than write-single /
write-bulk. Do not start another concurrent harness microprobe without a fresh
engine self-time profile.

At capture time, the shared checkout still had an unowned dirty
`crates/fsqlite-core/src/connection.rs` diff, so this clean detached worktree
artifact is the safer source of truth.
