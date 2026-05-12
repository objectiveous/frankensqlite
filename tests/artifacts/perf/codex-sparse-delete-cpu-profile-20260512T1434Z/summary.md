# Sparse DELETE CPU Profile

Date: 2026-05-12
Checkout head: `002e884eadfa9da2ec6d23d1248f88261c22459d`
Source state: unchanged Rust source since the source-equivalent build target
used for the current DML refresh.
Workload: `perf-update-delete 10000 1000 delete fsqlite sparse-isolated`

## Commands

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-dml-probe-ececff30 \
  CARGO_BUILD_JOBS=4 \
  cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

```bash
env FSQLITE_HOT_PROFILE=1 \
  perf record -F 999 --call-graph fp \
  -o tests/artifacts/perf/codex-sparse-delete-cpu-profile-20260512T1434Z/perf.data \
  -- /data/tmp/frankensqlite-target-dml-probe-ececff30/release-perf/perf-update-delete \
  10000 1000 delete fsqlite sparse-isolated
```

```bash
env FSQLITE_HOT_PROFILE=1 \
  perf record --delay 3300 -F 999 --call-graph fp \
  -o tests/artifacts/perf/codex-sparse-delete-cpu-profile-20260512T1434Z/perf-delete-window.data \
  -- /data/tmp/frankensqlite-target-dml-probe-ececff30/release-perf/perf-update-delete \
  10000 1000 delete fsqlite sparse-isolated
```

## Measurements

The first capture is dominated by population work and is retained only as a
setup trace. The delayed capture arms after the first isolated delete
iteration; it reports:

- `total=4101ms`
- `populate=3293ms`
- `delete=714ms`
- `per-row-delete=1428ns`
- `835` samples in `perf-delete-window.data`

`perf report` warns that kernel address maps are restricted on this host, so
kernel stacks in the reports contain unresolved addresses. User-space symbols
still resolve and are enough to identify the DELETE-side attribution.

## Hotspots

From `perf-delete-window-children.txt`:

- `_int_malloc`: `38.08%` children, `4.82%` self.
- `__libc_malloc2`: `36.66%` children, `0.25%` self.
- `PageData::as_bytes_mut`: `18.43%` children, `1.21%` self.
- `CachedPageEntry::shared_page`: `17.96%` children, `0.47%` self.
- `TransactionKind::get_page`: `10.99%` children, `6.59%` self.
- `TransactionKind::prefetch_page_hint`: `9.65%` children, `6.74%` self.
- `TableLeafDeleteRun::materialize_deletions`: `6.36%` children,
  `3.58%` self.
- `TableLeafDeleteRun::delete_rowid_with_reason`: `4.92%` children,
  `2.63%` self.

The attribution matches the physical retained-leaf DELETE path: sparse deletes
still stage rowids on retained leaf runs, then materialize mutable page images
from cached/shared page bytes at flush time.

## Conclusion

No source patch was attempted. This profile reconfirms the already fenced
DELETE materialization/PageData allocation family rather than exposing a new
standalone lever. Existing rejected attempts cover the obvious local variants:
materializer threshold changes, direct writer publication, borrowed write
publication, PageData move-before-publish, compactness prechecks, and retained
leaf search hints.

The next credible source change is still the broader transaction-local DML
mutation/read-view operator, with correctness proof for affected row counts,
read-your-writes, rollback/savepoints, duplicate/missing rowids, and a same-run
focused/fullquick keep gate.
