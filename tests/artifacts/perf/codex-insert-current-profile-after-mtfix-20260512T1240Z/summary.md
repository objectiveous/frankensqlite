# Current INSERT Profile After Shared-Table Retry Fix

- Date: 2026-05-12 12:31 UTC
- Commit: `88bfb5bc8c1506e5c62d0bc5593499382ad392e0`
- Command:
  `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-frontier-insert-88bfb5bc CARGO_BUILD_JOBS=4 FSQLITE_BENCH_PROFILE_INSERT=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick --filter insert --no-html --json-out tests/artifacts/perf/codex-insert-current-profile-after-mtfix-20260512T1240Z/insert-current.json`
- Validity: `git_dirty=false`, `benchmark_binary_older_than_git_head=false`.

## Summary

The focused INSERT matrix covers 25 scenarios:

| Metric | Value |
| --- | ---: |
| Franken faster / comparable / C faster | `18 / 2 / 5` |
| Average F/C | `0.8202160704165322` |
| Geomean F/C | `0.8001131959146491` |
| Focused weighted score | `0.8051996787183783` |

Rows above `1.0x` F/C are all 100-row fixed-cost cases:

| F/C | Scenario |
| ---: | --- |
| `1.1085003721479352` | `single-txn small_3col 100 rows` |
| `1.0981796713568828` | `single-txn medium_6col 100 rows` |
| `1.0970267816613708` | `small_3col 100 rows / batched (100/txn)` |
| `1.0915849641290054` | `small_3col 100 rows / single txn` |
| `1.0910325453831355` | `single-txn large_10col 100 rows` |
| `1.021523109971589` | `single-txn tiny_1col 100 rows` |

## Profile Notes

All profiled red rows stayed on the prepared direct INSERT fast path (`fast == rows`, `slow=0`) and used one empty-root page-run flush. Representative counters:

| Scenario | Row build ns | Preserialize ns | Direct flush ns | Background ns | Schema validation ns | Change tracking ns |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `tiny_1col 100` | `4682` | `0` | `3476` | `2844` | `3163` | `2406` |
| `small_3col 100` | `27601` | `21768` | `3587` | `2923` | `3308` | `2355` |
| `medium_6col 100` | `35995` | `30139` | `7434` | `2836` | `3249` | `2425` |
| `large_10col 100` | `55195` | `49272` | `23154` | `2924` | `3268` | `2404` |

## Decision

No source patch from this profile. The red rows are the already-fenced 100-row fixed-cost INSERT family. Prior measured rejects cover standalone serializer tweaks, concat/parameter-one/template variants, row-scratch borrow deferral, page-run threshold/arena changes, prebuilt empty-root builders, owned-record borrowed flushes, and direct page-image building. A keeper would need the broader fused row/body/page construction design and same-window focused plus full-quick wins.
