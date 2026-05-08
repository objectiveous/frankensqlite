# Lowercase Schema Index Lookup Candidate

Date: 2026-05-08

This was a read-only measurement of an unowned dirty
`crates/fsqlite-core/src/connection.rs` diff in the shared checkout. I did not
edit, stage, revert, or claim the source file.

## Candidate

`Connection::schema_index_of()` skips allocating `name.to_ascii_lowercase()`
when the incoming table name has no ASCII uppercase bytes:

```rust
let schema_idx = self.schema_by_name.borrow();
if name.bytes().any(|byte| byte.is_ascii_uppercase()) {
    schema_idx.get(&name.to_ascii_lowercase()).copied()
} else {
    schema_idx.get(name).copied()
}
```

The hypothesis was that benchmark SQL overwhelmingly uses lower-case table
names (`bench`, `orders`, `products`), so prepare/schema lookup fixed cost might
drop across small write rows.

## Build

Candidate build passed:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-private-shards-retry-target \
  CARGO_BUILD_JOBS=16 \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

## Focused UPDATE/DELETE

Baseline: `tests/artifacts/perf/current-post-dml-tanbear-20260508T0110Z/update-profile-current.json`.

Candidate: `candidate-update.json`.

| Scenario | Baseline ratio | Candidate ratio | Baseline F ms | Candidate F ms |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 1.325330 | 1.416287 | 0.119224 | 0.119063 |
| 100 rows / delete 5 rows | 1.459979 | 1.655411 | 0.113874 | 0.134522 |
| 1000 rows / update 100 rows | 0.997968 | 0.957129 | 0.403266 | 0.394138 |
| 1000 rows / delete 50 rows | 1.039226 | 0.868763 | 0.372148 | 0.368551 |
| 10000 rows / update 1000 rows | 1.050288 | 1.083403 | 3.867106 | 4.082236 |
| 10000 rows / delete 500 rows | 1.074879 | 1.047201 | 3.475211 | 3.571098 |

Focused average/geomean moved `1.157945 / 1.146025` to
`1.171366 / 1.141453`; C-SQLite-faster rows improved `4 -> 3`. This was mixed,
so I ran the full quick matrix.

## Full Quick

Baseline: `tests/artifacts/perf/calmthrush-clean-noprofile-20260508T0219Z/full-quick-clean-noprofile.json`.

Candidate: `candidate-full.json`.

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Primary weighted score | 0.345939 | 0.347366 |
| Average ratio | 0.454261 | 0.456919 |
| Geomean ratio | 0.267475 | 0.265545 |
| Median ratio | 0.292509 | 0.289769 |
| P90 ratio | 0.981158 | 1.054461 |
| P99 ratio | 1.401515 | 1.471466 |
| Comparable rows | 5 | 3 |
| C-SQLite-faster rows | 8 | 10 |

## Disposition

Rejected by the full quick keep gate. The candidate produced some geomean and
mid-distribution wins, but worsened the primary weighted score, average ratio,
p90/p99, and number of C-faster rows.

Do not keep or retry lower-case-only `schema_index_of()` allocation elision as a
standalone optimization. Reconsider only as part of a broader schema lookup
cache that proves full quick primary-score and tail improvement in the same
run.

