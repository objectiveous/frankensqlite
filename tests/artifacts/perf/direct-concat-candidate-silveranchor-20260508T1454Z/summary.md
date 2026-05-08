# Direct Concat Record Serializer Candidate

- Date: 2026-05-08T14:54Z
- Agent: SilverAnchor
- Source head: `cdd3d0aa58b69c0b0c574fcc8236f2e30c2ef504`
- Candidate source: temporary `crates/fsqlite-core/src/connection.rs` edit, reverted after measurement
- Benchmark: `comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/direct-concat-candidate-silveranchor-20260508T1454Z/candidate-insert.json --no-html`

## Candidate

The scratch patch added a `TextConcat` prepared direct-record value so
`Connection::try_serialize_prepared_direct_simple_insert_record` could compute
concat text length first, then serialize concat text directly into the SQLite
record body. The old `text_scratch` materialization path remained the fallback
for unsupported/lossy blob concat values and non-text affinity coercion.

This was intended to satisfy the ledger retry condition for prior rejected
param-one concat/text-cache ideas: avoid transient text materialization rather
than caching decimal text.

## Correctness Smoke

- `cargo fmt -p fsqlite-core --check`: passed after reverting the scratch patch.
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-direct-concat-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core prepared_direct_simple_insert_concat_chain -- --nocapture`: passed 3 targeted concat-chain tests on the candidate.
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-silveranchor-direct-concat-bench-target CARGO_BUILD_JOBS=16 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`: passed.

## Result

Rejected. Compared to the current insert-filter artifact
`tests/artifacts/perf/rusticgrove-full-quick-current-20260508T1510Z/insert-profile.json`,
the candidate worsened aggregate INSERT ratios:

| Metric | Current | Candidate |
|---|---:|---:|
| average ratio | 0.803142 | 0.857085 |
| geomean ratio | 0.780274 | 0.823145 |
| p90 ratio | 1.074184 | 1.176456 |
| p99 ratio | 1.132336 | 1.264681 |

Important frontier rows also regressed:

| Row | Current F/C | Candidate F/C |
|---|---:|---:|
| large_10col single txn 10K | 1.023510 | 1.135677 |
| large_10col record-size 10K | 1.083482 | 1.176456 |
| tiny_1col single txn 100 | 1.063987 | 1.264681 |
| small_3col 100 rows / batched | 1.132336 | 1.189669 |

The candidate is not a keeper. Source was restored; only this evidence bundle
and the negative-ledger entry remain.
