# Current-HEAD DML Boundary Refresh

Date: 2026-05-12

Commit under review: `0c0161449aeb91bc4c93de3568045295482b74b3`

## Commands

Benchmark binary rebuild:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target CARGO_BUILD_JOBS=8 \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

Focused DML refresh:

```bash
env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-target/release-perf/comprehensive-bench \
  --quick --filter update \
  --json-out tests/artifacts/perf/codex-head-dml-refresh-20260512T0340Z/update-delete-profile.json \
  --no-html
```

Repeat with stdout capture:

```bash
env FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-target/release-perf/comprehensive-bench \
  --quick --filter update \
  --json-out tests/artifacts/perf/codex-head-dml-refresh-20260512T0340Z/update-delete-profile-repeat.json \
  --no-html \
  2>&1 | tee tests/artifacts/perf/codex-head-dml-refresh-20260512T0340Z/stdout-repeat.txt
```

The benchmark reported `Git dirty: yes` because this artifact directory and
the existing untracked RCH scratch directories were present.

## Results

First run, `update-delete-profile.json`:

- `100 rows / update 10 rows`: C `0.005230 ms`, F `0.005721 ms`, F/C `1.094x`.
- `100 rows / delete 5 rows`: C `0.004278 ms`, F `0.006382 ms`, F/C `1.492x`.
- `1000 rows / update 100 rows`: C `0.038292 ms`, F `0.033232 ms`, F/C `0.868x`.
- `1000 rows / delete 50 rows`: C `0.015840 ms`, F `0.028864 ms`, F/C `1.822x`.
- `10000 rows / update 1000 rows`: C `0.390752 ms`, F `0.257752 ms`, F/C `0.660x`.
- `10000 rows / delete 500 rows`: C `0.159649 ms`, F `0.258053 ms`, F/C `1.616x`.

Repeat, `update-delete-profile-repeat.json`:

- `100 rows / update 10 rows`: C `0.004147 ms`, F `0.005761 ms`, F/C `1.389x`.
- `100 rows / delete 5 rows`: C `0.002204 ms`, F `0.006232 ms`, F/C `2.828x`.
- `1000 rows / update 100 rows`: C `0.058500 ms`, F `0.032380 ms`, F/C `0.554x`.
- `1000 rows / delete 50 rows`: C `0.025498 ms`, F `0.045745 ms`, F/C `1.794x`.
- `10000 rows / update 1000 rows`: C `0.355135 ms`, F `0.273282 ms`, F/C `0.770x`.
- `10000 rows / delete 500 rows`: C `0.160551 ms`, F `0.263383 ms`, F/C `1.640x`.

## Profile Highlights

All DELETE rows stayed on the prepared direct path (`slow=0`). The repeat
10K/500 DELETE row still had the retained same-leaf signature:

```text
delete_leaf_start=64/67
delete_leaf_active=433/496
delete_leaf_miss=63
delete_leaf_miss_out_of_leaf=60
delete_leaf_miss_last_cell=3
delete_leaf_flush=64/64
delete_leaf_flush_ns=82404
delete_leaf_materialize=64/69271
delete_leaf_write=64/7676
commit_us=52.7
```

The 100-row UPDATE row is noise-sensitive but still fixed-cost dominated. The
larger UPDATE rows remain faster than C SQLite. The DELETE rows remain the
durable red rows, and their counters match the retained leaf-run boundary
already documented in the negative ledger.

## Outcome

No source patch was attempted. The current-HEAD retest does not open a new
standalone DELETE micro-optimization: cursor preservation, retained leaf-run
search/admission/materialization, direct writer/flush, scratch reset, exact
transaction-control bypass, logical rowid buffers, and cell-log hooks are all
already measured as rejected or incomplete. The next credible source shape is
the broader transaction-local DML mutation operator that removes per-leaf
mutation/publication ceremony while preserving read-your-writes,
rollback/savepoints, duplicate/missing rowid behavior, schema drift handling,
quotient-filter/cache invalidation, and MVCC publication.
