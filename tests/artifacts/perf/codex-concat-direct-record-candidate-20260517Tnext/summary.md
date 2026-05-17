# Direct concat-to-record candidate

Date: 2026-05-17

Command:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-concat-direct-record-candidate-20260517 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert --json-out tests/artifacts/perf/codex-concat-direct-record-candidate-20260517Tnext/insert.json --no-html
```

Candidate:

- `crates/fsqlite-core/src/connection.rs` temporarily represented direct
  `ConcatChain` TEXT record cells as measured concat segments, then emitted the
  concat text directly into the SQLite record body during encoding.
- This avoided materializing concat TEXT into `prepared_direct_insert_text_scratch`
  before copying it into `prepared_direct_insert_record_scratch`.
- The candidate also added stored-value assertions to the large explicit-txn
  profile test. Those assertions were retained after the candidate was unwound.

Correctness before measurement:

- `cargo check -p fsqlite-core --lib` passed.
- Focused tests passed:
  - `test_prepared_direct_simple_insert_concat_chain_matches_mixed_coercions`
  - `test_prepared_direct_insert_preserialize_fallback_does_not_publish_child_profile`
  - `test_prepared_direct_simple_insert_large_profile_breakdown`

Focused INSERT result:

- Total scenarios: `25`.
- FrankenSQLite faster / comparable / C-SQLite faster: `16 / 4 / 5`.
- Average ratio: `0.9175352228881044`.
- Geomean ratio: `0.8905643951889011`.
- Weighted score: `0.8867708596034523`.
- p90 / p99: `1.231920806425139` / `1.4390306496553746`.

Rejected rows:

- `large_10col` 100 rows: `1.231920806425139x` slower
  (`C=0.176681 ms`, `F=0.217657 ms`).
- `large_10col` 1000 rows: `1.2296993720854474x` slower
  (`C=1.003799 ms`, `F=1.234371 ms`).
- `large_10col` 10000 rows: `1.2043136437716568x` slower
  (`C=9.948063 ms`, `F=11.980588 ms`).
- Record-size `large_10col` 10K: `1.26898229529697x` slower
  (`C=10.198872 ms`, `F=12.942188 ms`).

Decision:

Rejected and manually unwound. The aggregate focused INSERT score improved
against the latest no-profile repeat baseline, but the actual target rows
worsened materially. This violates the large-row keep gate for the current
campaign.
