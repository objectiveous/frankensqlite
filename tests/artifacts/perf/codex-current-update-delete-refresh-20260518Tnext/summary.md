# 2026-05-18 current UPDATE/DELETE refresh

Context:
- Current local branch head: `26d35cb59ac253530be1c0718611b7e90ecfc612`.
- The benchmark report recorded `git_dirty=true` because the working tree had
  staged source/docs/artifact changes when the run was captured.
- `TMPDIR=/data/tmp` was used because `/tmp` has been space-constrained in
  recent benchmark runs.
- `rch` fell back locally because workers were under critical pressure.

Command:

```bash
rch exec -- env TMPDIR=/data/tmp \
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-update-delete-20260518 \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter update-delete \
  --json-out tests/artifacts/perf/codex-current-update-delete-refresh-20260518Tnext/update-delete.json \
  --no-html
```

Result:
- Total scenarios: `6`.
- FrankenSQLite faster / comparable / C SQLite faster: `2 / 0 / 4`.
- Average ratio: `3.1128x`; geomean: `1.9405x`; weighted score:
  `1.9405x`; p90/p99: `11.1749x`.
- Medium/large UPDATE remains green: `1000 rows / update 100 rows` at
  `0.8883x`, and `10000 rows / update 1000 rows` at `0.7456x`.
- DELETE remains the durable red path:
  - `100 rows / delete 5 rows`: C `0.002184 ms`, F `0.024406 ms`,
    `11.1749x` slower; high variance (`CV C 104.5%`, `CV F 26.3%`).
  - `1000 rows / delete 50 rows`: C `0.021059 ms`, F `0.038653 ms`,
    `1.8355x` slower.
  - `10000 rows / delete 500 rows`: C `0.167574 ms`, F `0.276107 ms`,
    `1.6477x` slower.

Interpretation:
- The 100-row rows are too noisy to select a source patch by themselves.
- The stable signal matches the prior full-quick and DML profile artifacts:
  medium/large DELETE is still the next high-value gap, while standalone
  retained-leaf, MemDatabase, synced-root, and rowid-buffer micro-patches remain
  fenced by the negative ledger.
