# Sparse Isolated DELETE Delayed CPU Profile

- Date: 2026-05-12
- Git: `4017e4b407d13e725583db2138d878df0aae4ce1`
- Target: benchmark-shaped sparse DELETE attribution with delayed `perf`
  sampling.

## Command

```bash
perf record --delay 3300 -F 999 -g --call-graph dwarf \
  -o tests/artifacts/perf/codex-delete-sparse-isolated-delayed-cpu-profile-20260512T1050Z/perf.data \
  -- /data/tmp/frankensqlite-target-sparse-profiler/release-perf/perf-update-delete \
  10000 1000 delete fsqlite sparse-isolated
```

The delay starts sampling after the one-time populate phase. The run reported:

- total `4131ms`
- populate `3305ms`
- delete `730ms`
- per-row-delete `1461ns`
- captured `898` samples

## Top Self Samples

- `__memmove_avx_unaligned_erms`: `9.43%`
- `TransactionHandle::get_page`: `7.88%`
- `TransactionHandle::prefetch_page_hint`: `5.96%`
- `_int_malloc`: `4.74%`
- `TableLeafDeleteRun::materialize_deletions`: `3.69%`
- `TableLeafDeleteRun::delete_rowid_with_reason`: `1.94%`
- `Connection::execute_prepared_direct_simple_delete`: `1.26%`
- `Connection::flush_pending_direct_delete_leaf_run`: `0.47%`

## Interpretation

The profile avoids the previous contiguous-delete artifact and does not point
at the direct DELETE dispatcher as the main remaining cost. The visible costs
are page-copy/publication, page reads/prefetch, allocator activity, and retained
delete-run materialization.

Those families are already fenced in the negative ledger as standalone
micro-patches: retained materializer/direct-write variants and borrowed page
write attempts have not moved the primary matrix. This profile supports keeping
the next source lever at the broader transaction-local DML mutation/read-view
boundary rather than another isolated leaf-run rewrite.
