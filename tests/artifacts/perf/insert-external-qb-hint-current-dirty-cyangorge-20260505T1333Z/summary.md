# External quick-balance hint dirty-tree check

Run: `2026-05-05T13:33Z`

Source state:

- Base commit: `3f66c981`
- Dirty source included peer-owned `crates/fsqlite-btree/src/cursor.rs`
  external quick-balance hint handoff:
  - move `result.new_page_data` into the caller-owned hint,
  - clear the cursor-local `rightmost_leaf_cache` instead of caching another
    owned copy.
- No CyanGorge source edits were present in this run.

Commands:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-check-target cargo test -p fsqlite-btree rightmost_leaf -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-check-target cargo test -p fsqlite-core prepared_direct_simple_insert_implicit_rowid -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-check-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
env FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/insert-external-qb-hint-current-dirty-cyangorge-20260505T1333Z/report.json --no-html
```

Correctness gates:

- `fsqlite-btree rightmost_leaf`: 12 passed.
- `fsqlite-core prepared_direct_simple_insert_implicit_rowid`: 3 passed.
- `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`: passed.

Baseline compared:

- `tests/artifacts/perf/insert-external-qb-hint-owned-cyangorge-baseline-20260505T1318Z/report.json`

Matrix result:

| Metric | Clean baseline | Dirty candidate |
| --- | ---: | ---: |
| Average ratio | `2.5011x` | `2.5043x` |
| Geomean ratio | `2.3832x` | `2.4069x` |
| Median ratio | `2.2317x` | `2.3441x` |
| Primary weighted score | `1.6578` | `1.6726` |
| Write-bulk geomean | `2.5538x` | `2.5796x` |
| Write-single geomean | `1.4354x` | `1.4477x` |

Selected FSQLite medians:

| Row | Clean baseline | Dirty candidate |
| --- | ---: | ---: |
| single large_10col 10K | `37.5866 ms` | `35.5522 ms` |
| record-size large_10col 10K | `39.4682 ms` | `38.8316 ms` |
| single medium_6col 10K | `14.5795 ms` | `14.0969 ms` |
| record-size medium_6col 10K | `10.5972 ms` | `10.0059 ms` |
| single small_3col 10K | `7.3423 ms` | `6.9432 ms` |
| record-size small_3col 10K | `6.8629 ms` | `6.4866 ms` |

Verdict:

- Mixed but not a keep on the primary insert matrix. Several FSQLite medians
  improved, including the intended large-row path, but the primary weighted
  score and geomean ratio worsened against the clean baseline artifact.
- Do not land this exact dirty-tree diff on CyanGorge evidence alone. If the
  source owner continues it, use an interleaved clean/candidate A/B or a fresh
  isolated worktree comparison because the C SQLite side varied enough to make
  ratio-only interpretation noisy.
