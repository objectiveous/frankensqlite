# Focused Small UPDATE Frontier Repeat

Date: 2026-05-11

## Purpose

Recheck the remaining `100 rows / update 10 rows` red row after
`a7afd44a` without changing source. The current full quick matrix reports this
row at `1.324678418294426x` F/C, while larger UPDATE rows are already green.

## Build

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/codex-dml-update-repeat-target \
  CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e \
  --bin perf-update-delete
```

The release-perf build completed on worker `ts2`.

## Focused Standard UPDATE Results

Commands:

```bash
/data/tmp/codex-dml-update-repeat-target/release-perf/perf-update-delete 100 2000 update compare standard
/data/tmp/codex-dml-update-repeat-target/release-perf/perf-update-delete 1000 1000 update compare standard
/data/tmp/codex-dml-update-repeat-target/release-perf/perf-update-delete 10000 300 update compare standard
```

Artifacts:

- `update-100-standard-run1.txt`
- `update-100-standard-run2.txt`
- `update-100-standard-run3.txt`
- `update-1000-standard.txt`
- `update-10000-standard.txt`

| Rows / updates | Run | FrankenSQLite | C SQLite | F/C update time |
|---|---:|---:|---:|---:|
| 100 / 10 | 1 | 706 ns/update | 434 ns/update | 1.63x |
| 100 / 10 | 2 | 707 ns/update | 421 ns/update | 1.68x |
| 100 / 10 | 3 | 742 ns/update | 426 ns/update | 1.74x |
| 1000 / 100 | 1 | 336 ns/update | 394 ns/update | 0.85x |
| 10000 / 1000 | 1 | 300 ns/update | 383 ns/update | 0.78x |

## Conclusion

The small UPDATE tail is stable in the focused standard harness, but the shape
is fixed transaction/statement ceremony amortized over only 10 row mutations.
The already-published DML profile shows larger UPDATE rows green and points the
tiny-row fixed cost at transaction-control/prepared dispatch ceremony rather
than retained update patch-run work. The standalone exact transaction-control
bypass was already rejected by the full quick matrix in
`docs/progress/perf-negative-results.md`, so no source patch was attempted from
this repeat.

The next credible retry for this row is not another fixed-width UPDATE
micro-optimization. It would need to be part of a broader transaction lifecycle
redesign that also improves the full quick primary score and does not create new
INSERT or write-bulk red rows.
