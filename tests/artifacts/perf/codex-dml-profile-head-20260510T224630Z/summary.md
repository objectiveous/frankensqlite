# Current DML DELETE Profile

Date: 2026-05-10 UTC

Source: `39559112e38d775fb19076a098c60ee8e9ba2fac`

Command:

```bash
FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-codex-dml-profile-target/release-perf/comprehensive-bench \
  --quick \
  --filter update \
  --json-out tests/artifacts/perf/codex-dml-profile-head-20260510T224630Z/update-profile.json \
  --no-html
```

The binary was built with:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-dml-profile-target \
  CARGO_BUILD_JOBS=16 \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

`update-profile.json` reports `git_dirty: true`; the visible workspace dirt at
the time was pre-existing untracked RCH target directories.

## Matrix Rows

| Scenario | C median ms | F median ms | Ratio |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.004148 | 0.006703 | 1.61596 |
| 100 rows / delete 5 rows | 0.002285 | 0.007995 | 3.49891 |
| 1000 rows / update 100 rows | 0.036098 | 0.032140 | 0.89035 |
| 1000 rows / delete 50 rows | 0.015720 | 0.033924 | 2.15802 |
| 10000 rows / update 1000 rows | 0.350717 | 0.287659 | 0.82020 |
| 10000 rows / delete 500 rows | 0.159709 | 0.307647 | 1.92630 |

## DELETE Profile Counters

| Scenario | direct_delete | leaf starts | active hits | active misses | dirty flushes | active ns | flush ns | direct_flush ns | commit us |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 5 | 1/1 | 4/4 | 0 | 1/1 | 672 | 2495 | 2805 | 9.0 |
| 1000 rows / delete 50 rows | 50 | 6/6 | 44/49 | 5 | 6/6 | 5500 | 14126 | 1874 | 10.5 |
| 10000 rows / delete 500 rows | 500 | 64/67 | 433/496 | 63 | 64/64 | 49748 | 108719 | 1444 | 31.9 |

## Conclusion

This rules out the generic direct-write flush wrapper as the current DELETE
frontier: `direct_flush_ns` is only `1.4us` on the 500-row DELETE profile. The
remaining measured work is retained delete leaf-run admission/materialization
plus transaction commit publication. All profiled DELETE rows already use the
prepared direct fast path (`fast == direct_delete`, `slow == 0`).

The misses are page-boundary transitions (`rowid_not_in_leaf`) plus a small
number of conservative last-cell misses, not cell-shape failures. That matches
the existing rejected one-lever leaf-run family: another isolated
`TableLeafDeleteRun` threshold, next-cell hint, stack-entry move, or direct
writer publication patch is unlikely to satisfy the full matrix keep gate.
