# INSERT quick-balance exact divider space

Date: 2026-05-05 11:51 UTC

Commit measured: `199bd14b perf(btree/balance): gate balance_quick on the exact divider size, not the worst-case 15 bytes`

Candidate:
- `balance_quick_known_divider_rowid` builds the exact parent divider before the free-space gate.
- The parent-space check now requires `2 + 4 + actual_rowid_varint_len` bytes instead of always requiring 15 bytes.
- Added `test_balance_quick_uses_exact_divider_space` for the tight-space case that the old worst-case gate rejected.

Baseline artifact:
- `tests/artifacts/perf/insert-leaf-profile-cyangorge-20260505T113952Z/report.json`

Candidate artifact:
- `tests/artifacts/perf/insert-quick-balance-exact-space-cyangorge-20260505T115109Z/report.json`
- `tests/artifacts/perf/insert-quick-balance-exact-space-cyangorge-20260505T115109Z/run.log`
- `tests/artifacts/perf/insert-quick-balance-exact-space-cyangorge-20260505T115109Z/stdout.txt`

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --filter insert --json-out tests/artifacts/perf/insert-quick-balance-exact-space-cyangorge-20260505T115109Z/report.json --no-html
```

Primary result:
- Per-category weighted score: `1.7176362905025258` -> `1.714134818968377`
- Average ratio: `2.43227815474639` -> `2.426999068384355`
- Geomean ratio: `2.3451625121055546` -> `2.351875133557269`
- P90 ratio: `3.1813373850706426` -> `2.9840453469298707`
- P99 ratio: `4.614112196715469` -> `4.14480259701774`

Target-row movement:
- `record-size large_10col 10K`: FrankenSQLite median `37.802235 ms` -> `22.080128 ms` (`-41.59%`)
- `single-transaction large_10col 100K`: `460.426631 ms` -> `415.901572 ms` (`-9.67%`)
- `single-transaction large_10col 10K`: `37.472879 ms` -> `34.756262 ms` (`-7.25%`)
- `single-transaction medium_6col 10K`: `14.146497 ms` -> `13.498243 ms` (`-4.58%`)

Known caveat:
- Some small-row ratios moved the wrong direction, especially `single-transaction small_3col 100` by ratio. The FrankenSQLite median also regressed on that row (`0.228158 ms` -> `0.307706 ms`), so this should be watched in a later repeated A/B run. The candidate is kept because the primary weighted score and intended split-heavy rows moved favorably.
