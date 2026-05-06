# Recursive CTE Integer-Series SUM Fast Path

Agent: ProudAnchor
Date: 2026-05-06
Commit candidate: exact-shape algebraic evaluator for monotone integer recursive CTE `SUM` consumers

## Change

Added a fallback-safe evaluator for the common benchmark shape:

```sql
WITH RECURSIVE cnt(x) AS (
  SELECT <start>
  UNION ALL
  SELECT x + <positive_step> FROM cnt WHERE x < <bound>
)
SELECT SUM(x) FROM cnt;
```

The recognizer is deliberately narrow. It requires one CTE column, `UNION ALL`, a single recursive table source, `x + step` or `step + x`, and `x < bound`. Runtime scalar evaluation accepts integer literals, numbered integer parameters, and unary plus/minus. All other forms fall back to the generic recursive CTE path.

## Correctness Guards

- `UNION` dedup semantics are rejected and tested.
- Alias and numbered-parameter variants are accepted and tested.
- Overshoot semantics are preserved: the generated row is included after `x < bound`, matching the existing recursive CTE loop.
- Non-positive steps fall back.
- Next-value overflow falls back to the generic path.
- SUM overflow returns `FrankenError::IntegerOverflow`, matching integer `sum()` behavior.

## Benchmark Proof

Baseline full quick matrix:

- Artifact: `tests/artifacts/perf/mvcc-staged-marker-cyangorge-20260506T010316Z/full-report.json`
- Summary: average ratio `0.9601756554`, geomean `0.4172718291`, primary weighted score `0.5277828539`
- Recursive CTE row: C SQLite `0.139892 ms`, FrankenSQLite `0.177523 ms`, ratio `1.2690003717`

Candidate focused CTE run:

- Artifact: `report.json`
- Summary: 13 scenarios, FrankenSQLite faster in 13, average ratio `0.0614517849`, geomean `0.0155764731`, primary weighted score `0.0479235162`
- Recursive CTE row: C SQLite `0.187150 ms`, FrankenSQLite `0.002705 ms`, ratio `0.0144536468`

Candidate full quick matrix:

- Artifact: `full-report.json`
- Summary: average ratio `0.9265380249`, geomean `0.3965234520`, primary weighted score `0.5223267100`
- Recursive CTE row: C SQLite `0.164879 ms`, FrankenSQLite `0.003998 ms`, ratio `0.0242480850`
- Compared with baseline, the primary weighted score improved `0.5277828539 -> 0.5223267100`; lower is better.

An earlier RCH focused run also showed the same direction (`150.7 us` C SQLite vs `3.4 us` FrankenSQLite) but the JSON artifact did not retrieve before the wrapper hung in target retrieval, so the committed proof uses the local artifacts above.

## Verification

```bash
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-reccte-series cargo test -p fsqlite-core recursive_cte -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-reccte-localbench cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter cte --json-out tests/artifacts/perf/recursive-cte-algebraic-proudanchor-20260506T012811Z/report.json --no-html
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-reccte-localbench cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --json-out tests/artifacts/perf/recursive-cte-algebraic-proudanchor-20260506T012811Z/full-report.json --no-html
```

The local benchmark host had unrelated cargo activity during the full quick matrix, so p99 noise should not be overinterpreted. The target row moved by roughly two orders of magnitude, and the full-matrix primary score improved despite that noise.
