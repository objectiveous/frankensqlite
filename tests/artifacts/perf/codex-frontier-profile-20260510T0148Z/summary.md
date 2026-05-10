# Codex frontier profile: standard DELETE 100-row red row

Date: 2026-05-10
Base: `601dc619`
Worktree: `/data/tmp/frankensqlite-codex-frontier-profile-20260510`
Target dir: `/data/tmp/frankensqlite-codex-frontier-profile-target`

## Commands

Build:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-codex-frontier-profile-target CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete --bin comprehensive-bench
```

Focused compare:

```bash
/data/tmp/frankensqlite-codex-frontier-profile-target/release-perf/perf-update-delete 100 1000 delete compare standard
```

Profile:

```bash
perf record -F 997 --call-graph dwarf -o tests/artifacts/perf/codex-frontier-profile-20260510T0148Z/perf-delete100.data -- /data/tmp/frankensqlite-codex-frontier-profile-target/release-perf/perf-update-delete 100 50000 delete fsqlite standard
perf report --stdio --no-children --sort comm,dso,symbol -i tests/artifacts/perf/codex-frontier-profile-20260510T0148Z/perf-delete100.data > tests/artifacts/perf/codex-frontier-profile-20260510T0148Z/perf-delete100-self.txt
perf report --stdio --children --sort comm,dso,symbol -i tests/artifacts/perf/codex-frontier-profile-20260510T0148Z/perf-delete100.data > tests/artifacts/perf/codex-frontier-profile-20260510T0148Z/perf-delete100-children.txt
```

## Focused compare result

```text
fsqlite: total=136ms populate=29ms update=0ms delete=11ms  |  per-row-update=0ns  per-row-delete=2272ns
sqlite:  total=74ms populate=29ms update=0ms delete=2ms  |  per-row-update=0ns  per-row-delete=441ns
fsqlite/sqlite time ratio: total=1.83x populate=1.00x update=0.00x delete=5.15x
```

## Profile run result

```text
perf-update-delete: rows=100 iters=50000 which=delete engine=fsqlite mode=standard (do_update=false do_delete=true update_count=10 delete_count=5)
  (first iter complete)
fsqlite: total=7385ms populate=1465ms update=0ms delete=577ms  |  per-row-update=0ns  per-row-delete=2310ns
```

`perf record` captured 7,712 samples with no lost samples. Kernel symbols were restricted by the host, so unresolved kernel frames remain in the child report.

## Top self-time frames

From `perf-delete100-self.txt`:

```text
6.06% __memmove_avx_unaligned_erms
5.61% _int_malloc
4.48% Connection::try_serialize_prepared_direct_simple_insert_record
2.06% cfree
1.93% malloc
1.76% Connection::execute_prepared_direct_simple_insert
1.57% Connection::open_with_env_and_pager
1.39% malloc_consolidate
1.36% ShardedPageCache::with_max_buffers_for_initial_pages
1.34% SharedMvccState::new
1.27% FastPageArray::insert
0.95% PublishedPagerState::new
0.93% Connection::eval_prepared_direct_simple_insert_expr
0.92% Connection::execute_prepared_with_params_after_background_status
0.85% __memset_avx2_unaligned_erms
```

## Decision

No code patch from this profile.

The standard 100-row DELETE red row still has a large DELETE-body gap in the focused compare, but the sampled wall-clock profile is dominated by benchmark setup/populate and allocator-heavy direct INSERT record construction. The hottest FrankenSQLite frame is `try_serialize_prepared_direct_simple_insert_record`, not the delete leaf mutation path. That makes another tombstone/physical DELETE overlay patch low expected value unless a lower-level counter proves the candidate path fires and isolates delete-only wall time from setup/populate cost.

Next useful proof should either:

1. isolate DELETE-only body cost with setup/populate excluded from the sampling envelope, or
2. attack the small-row setup/direct-INSERT allocation path with a lever not already fenced by the negative ledger.
