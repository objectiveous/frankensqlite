# Lowercase Schema Index Lookup Attempt

Date: 2026-05-08

This bundle is retained as an invalidated read-only attempt, not a valid
candidate rejection.

I initially observed an unowned dirty `crates/fsqlite-core/src/connection.rs`
diff around `Connection::schema_index_of()` in the shared checkout and started a
read-only measurement. Before the artifact bundle captured `candidate.diff`, the
dirty source diff disappeared from the shared checkout. `candidate.diff` is
therefore empty, and the release-perf binary was built from the restored
baseline source. Treat the JSON files here as a noisy baseline rerun, not as
evidence for or against the candidate.

## Intended Candidate

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

The build passed, but it no longer represented the intended candidate because
the source diff had vanished:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-private-shards-retry-target \
  CARGO_BUILD_JOBS=16 \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

## Captured UPDATE/DELETE Rerun

Baseline: `tests/artifacts/perf/current-post-dml-tanbear-20260508T0110Z/update-profile-current.json`.

Captured rerun: `candidate-update.json`.

| Scenario | Baseline ratio | Candidate ratio | Baseline F ms | Candidate F ms |
| --- | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 1.325330 | 1.416287 | 0.119224 | 0.119063 |
| 100 rows / delete 5 rows | 1.459979 | 1.655411 | 0.113874 | 0.134522 |
| 1000 rows / update 100 rows | 0.997968 | 0.957129 | 0.403266 | 0.394138 |
| 1000 rows / delete 50 rows | 1.039226 | 0.868763 | 0.372148 | 0.368551 |
| 10000 rows / update 1000 rows | 1.050288 | 1.083403 | 3.867106 | 4.082236 |
| 10000 rows / delete 500 rows | 1.074879 | 1.047201 | 3.475211 | 3.571098 |

Average/geomean moved `1.157945 / 1.146025` to `1.171366 / 1.141453`;
C-SQLite-faster rows improved `4 -> 3`. Because the source diff was gone, these
numbers should be interpreted only as baseline noise.

## Captured Full Quick Rerun

Baseline: `tests/artifacts/perf/calmthrush-clean-noprofile-20260508T0219Z/full-quick-clean-noprofile.json`.

Captured rerun: `candidate-full.json`.

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

Invalidated. Do not cite this bundle as a rejection of lower-case-only
`schema_index_of()` allocation elision. A valid test would need the source diff
captured in `candidate.diff` before the build and then a fresh same-window A/B.
