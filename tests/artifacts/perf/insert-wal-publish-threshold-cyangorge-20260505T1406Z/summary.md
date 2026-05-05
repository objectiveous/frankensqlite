# Thresholded WAL prepared-frame publication A/B

Run ID: `insert-wal-publish-threshold-cyangorge-20260505T1406Z`

## Scenario

This was an isolated follow-up to the dirty WAL direct-publication check. The
dirty direct path improved some large-row medians but worsened the primary
weighted insert score. Because `crates/fsqlite-core/src/wal_adapter.rs` was
reserved by another agent, this candidate was tested only in the temporary
worktree `/data/tmp/frankensqlite-cyangorge-wal-threshold-20260505T1406`.

The candidate kept small prepared-frame commits on the existing
`pending_publication_frames` path and used direct publication only when
`prepared.frame_count() >= 128`. The intent was to preserve write-single
behavior while keeping the large-row benefit.

The source patch is preserved in `source.diff`; it was not applied to the main
worktree.

## Correctness checks

- `cargo fmt`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-wal-threshold-target cargo test -p fsqlite-core --lib append -- --nocapture`
  passed: `18 passed; 0 failed`. This includes a new 128-frame prepared append
  unit proof for the threshold path.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-wal-threshold-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
  passed.

## Benchmark command

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-cyangorge-wal-threshold-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/insert-wal-publish-threshold-cyangorge-20260505T1406Z/report.json \
  --no-html
```

Baseline for comparison:
`tests/artifacts/perf/insert-external-qb-hint-owned-cyangorge-baseline-20260505T1318Z/report.json`.

Prior dirty direct-publication check:
`tests/artifacts/perf/insert-wal-publish-direct-current-dirty-cyangorge-20260505T135315Z/report.json`.

## Result

Rejected. The thresholded variant was worse than the clean baseline and also
worse than the full dirty direct-publication variant:

| Metric | Baseline | Full direct dirty | Threshold candidate |
| --- | ---: | ---: | ---: |
| Average F/C ratio | 2.5011x | 2.4813x | 2.6316x |
| Geomean F/C ratio | 2.3832x | 2.3890x | 2.5341x |
| Median F/C ratio | 2.2317x | 2.2006x | 2.3690x |
| Weighted score | 1.6578 | 1.7359 | 1.8890 |
| Write-bulk geomean | 2.5538x | 2.5388x | 2.6799x |
| Write-single geomean | 1.4354x | 1.5293x | 1.6811x |

Selected row medians:

| Row | Baseline F median | Full direct dirty F median | Threshold F median |
| --- | ---: | ---: | ---: |
| single txn medium_6col 10K | 14.579 ms | 14.707 ms | 14.153 ms |
| single txn large_10col 10K | 37.587 ms | 35.188 ms | 37.229 ms |
| record-size medium_6col 10K | 10.597 ms | 9.759 ms | 11.191 ms |
| record-size large_10col 10K | 39.468 ms | 34.709 ms | 38.261 ms |

The profile shows the intended commit path did not dominate enough to offset the
code-shape disturbance. Large-row commit roundtrip remained high:

- `fs_insert_single_txn_large_10col_10000`: `commit_roundtrip_ns=17652726`
- `fs_insert_record_size_large_10col_10000`: `commit_roundtrip_ns=17059283`

## Disposition

Do not retry a simple frame-count threshold around prepared-frame direct
publication. It failed to preserve the full direct-publication large-row win and
made write-single substantially worse. Future work in this area should first
prove, with instrumentation, why the full direct path improved large rows before
changing the publication path again.
