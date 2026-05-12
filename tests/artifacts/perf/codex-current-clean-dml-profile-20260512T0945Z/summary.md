# Clean Current DML Profile After Page-1 Skip Rejection

- Date: 2026-05-12
- Git: `5fdec8ce9906dadde6b9b75ed97b7ca592b2efd3`
- Source state: clean current `HEAD`
- Target: current `UPDATE/DELETEThroughput` boundary after rejecting and
  unwinding the normal private-memory page-1 skip candidate.

## Command

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-clean-dml-profile CARGO_BUILD_JOBS=8 FSQLITE_BENCH_PROFILE_DML=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter update --json-out tests/artifacts/perf/codex-current-clean-dml-profile-20260512T0945Z/current-update.json --no-html
```

## Focused Rows

- `100 rows / update 10 rows`: C `0.004328 ms`, F `0.006282 ms`, `1.451x`.
- `100 rows / delete 5 rows`: C `0.002314 ms`, F `0.007133 ms`, `3.083x`.
- `1000 rows / update 100 rows`: C `0.037410 ms`, F `0.028193 ms`, `0.754x`.
- `1000 rows / delete 50 rows`: C `0.015990 ms`, F `0.029405 ms`, `1.839x`.
- `10000 rows / update 1000 rows`: C `0.378869 ms`, F `0.247253 ms`, `0.653x`.
- `10000 rows / delete 500 rows`: C `0.160731 ms`, F `0.258264 ms`, `1.607x`.

Update-filter geomean was `1.3662x` F/C. The larger UPDATE rows remain green;
the 100-row UPDATE fixed-cost row and the DELETE rows remain the active DML
frontier.

## Counter Notes

The profiled DELETE rows still use the prepared direct path with `slow=0`. The
500-row DELETE profile reports `delete_leaf_active=433/496`,
`delete_leaf_miss=63`, `delete_leaf_flush=64/64`,
`delete_leaf_flush_ns=50506`, `delete_leaf_materialize=64/37672`,
`delete_leaf_write=64/7431`, and `commit_us=38.9`.

The 100-row UPDATE profile reports `direct_update=10`, `slow=0`,
`begin_ns=2425`, `execute_body_ns=7724`, `prepared_lookup_ns=2143`,
`commit_roundtrip_ns=2014`, and `commit_us=6.8`.

## Outcome

No source patch was attempted from this profile. The obvious standalone
families are already fenced by the negative ledger: exact transaction-control
SQL bypasses, parser/background/prepared-lookup trimming, retained DELETE
leaf-run admission/materialization tweaks, direct-flush wrappers, and
standalone page-1 commit skipping.

The next credible source lever remains the broader transaction-local DML
mutation operator that changes the representation and read/materialization
boundary for many rowid mutations inside a transaction.
