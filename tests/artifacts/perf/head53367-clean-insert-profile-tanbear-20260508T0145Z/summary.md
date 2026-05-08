# Clean Current-Head INSERT Profile

Date: 2026-05-08

Clean worktree:
`/data/tmp/frankensqlite-directcompiled-rowvalues-20260508T0140Z`

Head:
`53367d5ae5b250750b39e6455359de7c5f265224`
(`docs(perf): reject insert row-value retention cleanup`)

The shared checkout had an unowned dirty
`crates/fsqlite-e2e/src/bin/comprehensive_bench.rs` change reserved by another
agent, so this profile was built and run from the clean detached worktree. The
benchmark report recorded `git_dirty=false` and
`benchmark_binary_older_than_git_head=false`.

Build command:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-head53367-insert-target \
  CARGO_BUILD_JOBS=10 \
  cargo build -p fsqlite-e2e --bin comprehensive-bench --profile release-perf
```

Run command:

```text
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-head53367-insert-target/release-perf/comprehensive-bench \
  --quick --filter insert --no-html \
  --json-out tests/artifacts/perf/head53367-clean-insert-profile-tanbear-20260508T0145Z/insert-profile.json
```

## Summary

- Total scenarios: `25`
- FrankenSQLite faster: `17`
- Comparable: `1`
- C SQLite faster: `7`
- Average ratio: `0.8422226193858254`
- Geomean ratio: `0.8146965455920142`
- P90 ratio: `1.1383879631659723`
- P99 ratio: `1.2528803731015827`
- Primary weighted score: `0.8005251603354614`

Rows still above `1.05x`:

| Ratio | Section | Scenario | FSQLite ms | C SQLite ms |
| ---: | --- | --- | ---: | ---: |
| `1.252880` | Single Transaction medium_6col | 1000 rows | `0.676707` | `0.540121` |
| `1.143339` | Transaction Strategy small_3col | 100 rows / single txn | `0.083346` | `0.072897` |
| `1.138388` | Transaction Strategy small_3col | 100 rows / batched (100/txn) | `0.083075` | `0.072976` |
| `1.120724` | Single Transaction small_3col | 100 rows | `0.083977` | `0.074931` |
| `1.118877` | Single Transaction large_10col | 100 rows | `0.163977` | `0.146555` |
| `1.101279` | Single Transaction medium_6col | 100 rows | `0.111118` | `0.100899` |
| `1.096652` | Single Transaction tiny_1col | 100 rows | `0.072526` | `0.066134` |

## Profile Readout

The slow rows are not record-serializer limited anymore. The relevant profile
lines report `serialize_ns=0` and no record decode work. The visible costs are
connection-level direct INSERT expression/row construction plus fixed
schema/MemDB/change bookkeeping:

| Row | setup_us | begin_us | prepare_us | insert_us | commit_us | row_build_ns | btree_insert_ns | memdb_apply_ns | schema_validation_ns | change_tracking_ns |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `single tiny_1col 100` | `16.3` | `9.8` | `8.6` | `47.8` | `15.2` | `3418` | `3206` | `2384` | `3175` | `2355` |
| `single small_3col 100` | `15.8` | `9.1` | `9.4` | `55.8` | `11.3` | `11486` | `3315` | `2423` | `3237` | `2345` |
| `batched small_3col 100` | `14.9` | `8.0` | `12.3` | `56.4` | `10.3` | `11542` | `3255` | `2889` | `3211` | `2418` |
| `single txn small_3col 100` | `16.7` | `9.6` | `10.5` | `56.8` | `10.9` | `11990` | `3284` | `2898` | `3250` | `2420` |
| `single small_3col 1000` | `18.0` | `9.7` | `10.7` | `535.6` | `42.3` | `108443` | `29989` | `23944` | `31001` | `23994` |
| `record large_10col 10000` | `45.7` | `15.7` | `35.8` | `8573.9` | `3921.0` | `3936233` | `632835` | `240663` | `315166` | `237912` |

## Interpretation

This profile reinforces the current stop rules:

- Do not chase standalone record serialization changes for the remaining INSERT
  rows; `serialize_ns=0` on this profile.
- Do not retry the newly ledgered direct-compiled row-values cleanup; it was
  already measured and rejected in `53367d5a`.
- The remaining high-EV work is in
  `crates/fsqlite-core/src/connection.rs`: direct INSERT row expression/build
  fusion, fixed schema validation/change tracking amortization, or a broader
  page-run/page-builder design that removes several of those costs together.

At capture time, `connection.rs` and the benchmark harness were reserved by
another agent, so this artifact is a clean baseline and target map, not a source
candidate.
