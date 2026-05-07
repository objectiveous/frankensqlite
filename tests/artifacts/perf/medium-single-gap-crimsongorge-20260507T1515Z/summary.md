# Medium single-transaction INSERT gap pass

- Agent: CrimsonGorge
- Date: 2026-05-07
- Target: `INSERTThroughput - Single Transaction - medium_6col / 1000 rows`
- Baseline: `tests/artifacts/perf/explicit-batch-frontier-crimsongorge-20260507T1350Z/final-full-repeat.json`

## Baseline profile

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-next-gap-perf-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/medium-single-gap-crimsongorge-20260507T1515Z/insert-profile.json --no-html
```

Observed target row:

- `medium_6col / 1000 rows`: C SQLite `0.535523 ms`, FrankenSQLite `0.742511 ms`, ratio `1.386516`
- Profile: `row_build_ns=248814`, `btree_insert_ns=97260`, `commit_roundtrip_ns=109646`

## Rejected candidate

Candidate shape: compile text-literal/`?1` concat chains into a compact prepared-direct expression that reused the cached integer text for `?1`.

Correctness gate passed:

```bash
cargo fmt -p fsqlite-core --check
env TMPDIR=/data/tmp/frankensqlite-crimsongorge-tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-next-gap-check-target CARGO_BUILD_JOBS=16 cargo test -p fsqlite-core test_prepared_insert_ -- --nocapture
```

Focused insert matrix:

- `insert-profile.json` average ratio: `0.923939`
- `insert-paramconcat-candidate.json` average ratio: `0.918999`
- Target row improved: FrankenSQLite `0.742511 ms -> 0.684553 ms`, ratio `1.386516 -> 1.250723`
- Target `row_build_ns` improved: `248814 -> 201460`

Full quick matrix:

- `final-full-repeat.json` average ratio: `0.496253`, C-faster rows `13`, p99 `1.509348`
- `full-paramconcat-candidate.json` average ratio: `0.503004`, C-faster rows `14`, p99 `1.536446`

Decision: rejected and reverted. The targeted row-build idea worked locally, but the full quick benchmark moved the wrong way.

## Kept fix

While validating the candidate, the existing prepared direct-insert test group exposed a real SQLite-parity bug: the fast lanes returned constraint errors without resetting `changes()` for failed direct inserts. The kept patch resets statement change tracking on the direct-insert error paths and adds a semantic prepared-concat NULL test.
