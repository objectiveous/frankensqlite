# Post-Compact UPDATE/DELETE Reprofile

Date: 2026-04-28
Base commit: `24ec46ad perf(btree): publish compact delete proof pack`
Binary: `/data/tmp/cargo-target-snowyfortress-20260428-post-compact-profile/release-perf/perf-update-delete`
Scenario: `perf-update-delete 10000 100 both`

## Timing

Command:

```bash
hyperfine --warmup 1 --runs 12 \
  --export-json tests/artifacts/perf/20260428T0106Z-snowyfortress-post-compact/hyperfine-head-10000x100-both.json \
  --command-name head-24ec46ad \
  '/data/tmp/cargo-target-snowyfortress-20260428-post-compact-profile/release-perf/perf-update-delete 10000 100 both'
```

Result:

- mean: `1.339538196653333s`
- stddev: `0.022551189813900935s`
- median: `1.3412338213200001s`
- min: `1.3099163423200002s`
- max: `1.3755560403200002s`
- user: `1.1591181300000002s`
- system: `0.17610858s`

## Profile

Command:

```bash
perf record -F 997 -g --call-graph dwarf \
  -o /data/tmp/snowyfortress-post-compact-both.data -- \
  /data/tmp/cargo-target-snowyfortress-20260428-post-compact-profile/release-perf/perf-update-delete 10000 100 both
```

Run output:

```text
total=1491ms populate=806ms update=396ms delete=194ms | per-row-update=3964ns per-row-delete=3889ns
```

Flat profile top symbols:

- `8.45%` `__memmove_avx_unaligned_erms`
- `5.94%` `Connection::execute_prepared_direct_simple_insert`
- `3.68%` `BtCursor<SharedTxnPageIo>::delete`
- `3.05%` `_int_malloc`
- `2.34%` `SharedTxnPageIo::write_page_internal`
- `2.31%` `WalChecksumTransform::for_wal_frame`
- `2.18%` `Connection::execute_prepared_with_params_after_background_status`
- `2.14%` `core::str::converts::from_utf8`
- `2.09%` `BtCursor::table_seek_for_insert`
- `2.02%` `concurrent_page_state`

Children profile shows the earlier compact-delete sort hotspot is no longer dominant. The next visible cluster is the INSERT populate/write path: record/value construction, memcpy/memmove into pages, staged page mutation/allocation, WAL checksum, and MVCC write bookkeeping.

## Candidate Rejected

I explored wiring the existing table-leaf writer callback primitive so prepared direct INSERT could serialize record bytes directly into retained rightmost-leaf page space instead of serializing to `record_scratch` and then copying into the page. That matched the `memmove`/insert cluster but required restructuring the prepared direct INSERT hot path.

The candidate was rejected and fully rolled back in the worktree. It did not reach benchmark/commit because targeted direct-insert tests exposed an unsafe validation surface before measurement:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260428-zero-copy-check \
  cargo test -p fsqlite-core direct_simple_insert -- --nocapture
```

Observed during the rejected candidate loop:

- `29 passed`
- `2 failed`
- failing area: prepared direct INSERT autocommit/profile coverage

No code change is included in this artifact. The current tree is back to the base code for the explored files.

## Next Target

The next optimization should stay profile-first and avoid restructuring direct INSERT until the retained/autocommit interaction is isolated. Lower-risk follow-up targets from this profile:

- Measure whether `Connection::execute_prepared_with_params_after_background_status` can avoid the unconditional `Instant::now`/statement counter path when hot-path profiling is disabled.
- Split INSERT populate from UPDATE/DELETE timing in a follow-up profile so insert-path wins are not mistaken for update/delete wins.
- Reprofile with allocation telemetry around `PageData::as_bytes_mut`, staged page overwrite, and `SharedTxnPageIo::write_page_internal` before touching page ownership.
