# 2026-05-17 current INSERT refresh

Context:
- Current local branch head: `26d35cb59ac253530be1c0718611b7e90ecfc612`.
- The benchmark report recorded `git_dirty=true` because the working tree had
  local edits/artifacts when the run was captured.
- `/tmp` was full, so both benchmark runs used `TMPDIR=/data/tmp`.
- `rch` fell back locally because workers were under critical pressure.

Commands:

```bash
rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-insert-20260517j \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter insert \
  --json-out tests/artifacts/perf/codex-current-insert-refresh-20260517Tnext/insert.json \
  --no-html

rch exec -- env TMPDIR=/data/tmp FSQLITE_BENCH_PROFILE_INSERT=1 \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-insert-20260517j \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter insert \
  --json-out tests/artifacts/perf/codex-current-insert-refresh-20260517Tnext/insert-profile.json \
  --no-html
```

No-profile result:
- Total scenarios: 25.
- FrankenSQLite faster: 19; comparable: 0; C SQLite faster: 6.
- Average ratio: `0.8664x`; geomean: `0.8342x`; weighted score:
  `0.9248x`; p90: `1.2373x`; p99: `1.6522x`.
- Remaining C-faster rows above `1.05x`:
  - `100 rows` small 3-col write bulk: C `0.150793 ms`, F `0.175839 ms`, `1.1661x`.
  - `100 rows` large 10-col write bulk: C `0.179065 ms`, F `0.295854 ms`, `1.6522x`.
  - `10000 rows` large 10-col write bulk: C `10.232949 ms`, F `12.041954 ms`, `1.1768x`.
  - `100 rows / autocommit`: C `0.135334 ms`, F `0.167453 ms`, `1.2373x`.
  - `100 rows / batched (100/txn)`: C `0.102942 ms`, F `0.109365 ms`, `1.0624x`.
  - `large_10col` record-size row: C `10.133212 ms`, F `12.634582 ms`, `1.2468x`.

Profile-enabled result:
- Total scenarios: 25.
- FrankenSQLite faster: 15; comparable: 6; C SQLite faster: 4.
- Average ratio: `0.9198x`; geomean: `0.8706x`; weighted score:
  `0.9062x`; p90: `1.1305x`; p99: `2.5673x`.
- Remaining C-faster rows above `1.05x`:
  - `100 rows` small 3-col write bulk: C `0.083937 ms`, F `0.215494 ms`, `2.5673x`.
  - `1000 rows` large 10-col write bulk: C `0.926053 ms`, F `1.046889 ms`, `1.1305x`.
  - `10000 rows` large 10-col write bulk: C `10.015982 ms`, F `11.274959 ms`, `1.1257x`.
  - `large_10col` record-size row: C `9.280667 ms`, F `11.052963 ms`, `1.1910x`.

Hot-path reading:
- The large 10-col 10K-row rows are dominated by direct-record row building,
  not by page-run construction. Representative profile rows put row build near
  `20.9 ms`, preserialization near `20.4 ms`, and the B-tree bulk leaf
  build/write plus direct flush below that by an order of magnitude.
- Subprofiled preserialization cost in the large 10-col 10K-row shape is split
  across expression evaluation (`~4.6 ms`), affinity (`~2.3 ms`), layout
  (`~2.5 ms`), and encode (`~1.0 ms`), with the rest inside cell planning.
- This argues against retrying standalone page-run or scratch-buffer tweaks as
  the next insert lever. The next insert attempt needs to reduce the
  prepared-record cell planning/eval/affinity/layout path in a same-window A/B,
  while respecting the existing negative ledger fences for rowid alias,
  affinity-only, scratch-buffer, direct-concat, and page-run-only candidates.
