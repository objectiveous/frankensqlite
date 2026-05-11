# Current DELETE CPU Profile

Date: 2026-05-11

## Purpose

Refresh CPU-symbol evidence for the remaining `UPDATE/DELETEThroughput` DELETE
red rows after `396e055b`, without changing source. The current full quick
matrix still shows DELETE at `2.838x`, `1.829x`, and `1.595x` F/C for the
5-row, 50-row, and 500-row cases.

## Build

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/codex-current-delete-cpu-profile-target \
  CARGO_BUILD_JOBS=8 cargo build --profile release-perf -p fsqlite-e2e \
  --bin perf-update-delete
```

The release-perf build completed on worker `ts2`; artifact retrieval took about
214 seconds.

## Profiles

Commands:

```bash
perf record -F 999 --call-graph fp \
  -o delete-isolated-5000.perf.data -- \
  /data/tmp/codex-current-delete-cpu-profile-target/release-perf/perf-update-delete \
  10000 5000 delete fsqlite isolated

perf report --stdio --no-children --sort overhead,symbol \
  -i delete-isolated-5000.perf.data > delete-isolated-5000.perf-report.txt

perf record -F 999 --call-graph fp \
  -o delete-rollback-isolated-10000.perf.data -- \
  /data/tmp/codex-current-delete-cpu-profile-target/release-perf/perf-update-delete \
  10000 10000 delete fsqlite rollback-isolated

perf report --stdio --no-children --sort overhead,symbol \
  -i delete-rollback-isolated-10000.perf.data \
  > delete-rollback-isolated-10000.perf-report.txt
```

Kernel symbols were restricted by the host, but user-space symbols resolved.

## Key Results

The long isolated run captured 2,510 samples. It reported `total=2415ms`,
`populate=777ms`, and `delete=1506ms`, or `603 ns` per deleted row. The top
delete-relevant symbols were:

| Symbol | Overhead |
|---|---:|
| `TransactionKind::get_page` | 17.39% |
| `TableLeafDeleteRun::delete_rowid_with_reason` | 3.71% |
| `TransactionKind::write_page_data` | 3.67% |
| `TableLeafPayloadPatchRun::table_leaf_rowid_at` | 2.20% |
| `TransactionKind::free_page` | 1.91% |
| `execute_prepared_direct_simple_delete` | 1.11% |
| `flush_pending_direct_delete_leaf_run` | 0.37% |
| `TableLeafDeleteRun::materialize_deletions` | 0.33% |

The rollback-isolated run captured 13,815 samples and reported `delete=1864ms`,
or `373 ns` per deleted row, but its top symbols were dominated by rollback
MemDB reload (`parse_record_into`, `reload_memdb_from_txn_with_mode`, and
related allocation/free work). That profile is useful as a rollback-path screen,
not as direct evidence for the standard full-quick DELETE row.

## Conclusion

No source patch was attempted. The current CPU evidence points at the same
families already fenced in `docs/progress/perf-negative-results.md`:
`TransactionKind` page access/publication, delete-run rowid lookup/materialize,
and flush/publication mechanics. The earlier standalone
`TransactionKind` force-inline retry regressed, and the retained delete-run
admission/materialization/search-hint/direct-flush variants are already
negative-ledgered.

The next credible DELETE attempt needs to be a broader transaction-local DML
mutation operator or pager/MVCC representation change, proven against focused
DELETE and the full quick primary score in the same window.
