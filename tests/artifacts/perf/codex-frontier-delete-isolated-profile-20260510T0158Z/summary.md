# Codex frontier profile: isolated DELETE body

Date: 2026-05-10
Base: `601dc619`
Worktree: `/data/tmp/frankensqlite-codex-frontier-profile-20260510`
Target dir: `/data/tmp/frankensqlite-codex-frontier-profile-target`

## Commands

Focused isolated comparisons:

```bash
/data/tmp/frankensqlite-codex-frontier-profile-target/release-perf/perf-update-delete 100 10000 delete compare isolated
/data/tmp/frankensqlite-codex-frontier-profile-target/release-perf/perf-update-delete 1000 2000 delete compare isolated
/data/tmp/frankensqlite-codex-frontier-profile-target/release-perf/perf-update-delete 100 10000 update compare isolated
```

Profile:

```bash
perf record -F 997 --call-graph dwarf -o tests/artifacts/perf/codex-frontier-delete-isolated-profile-20260510T0158Z/perf-delete100-isolated.data -- /data/tmp/frankensqlite-codex-frontier-profile-target/release-perf/perf-update-delete 100 100000 delete fsqlite isolated
perf report --stdio --no-children --sort comm,dso,symbol -i tests/artifacts/perf/codex-frontier-delete-isolated-profile-20260510T0158Z/perf-delete100-isolated.data > tests/artifacts/perf/codex-frontier-delete-isolated-profile-20260510T0158Z/perf-delete100-isolated-self.txt
perf report --stdio --children --sort comm,dso,symbol -i tests/artifacts/perf/codex-frontier-delete-isolated-profile-20260510T0158Z/perf-delete100-isolated.data > tests/artifacts/perf/codex-frontier-delete-isolated-profile-20260510T0158Z/perf-delete100-isolated-children.txt
```

## Focused compare results

```text
100 rows, isolated DELETE:
fsqlite: total=96ms populate=15ms delete=79ms  |  per-row-delete=1594ns
sqlite:  total=31ms populate=16ms delete=15ms  |  per-row-delete=304ns
fsqlite/sqlite delete ratio: 5.25x

1000 rows, isolated DELETE:
fsqlite: total=214ms populate=31ms delete=181ms  |  per-row-delete=1814ns
sqlite:  total=65ms populate=35ms delete=29ms    |  per-row-delete=294ns
fsqlite/sqlite delete ratio: 6.18x

100 rows, isolated UPDATE:
fsqlite: total=14ms update=12ms  |  per-row-update=130ns
sqlite:  total=30ms update=30ms  |  per-row-update=303ns
fsqlite/sqlite update ratio: 0.43x
```

The isolated DELETE gap is real; the isolated UPDATE body is already faster than C SQLite in this harness.

## Profile run result

```text
perf-update-delete: rows=100 iters=100000 which=delete engine=fsqlite mode=isolated (do_update=false do_delete=true update_count=10 delete_count=5)
  (first isolated delete iter complete)
fsqlite: total=2043ms populate=150ms update=0ms delete=1885ms  |  per-row-update=0ns  per-row-delete=3771ns
```

`perf record` captured 2,258 samples with no lost samples. Kernel symbols were restricted by the host, so unresolved kernel frames remain in the child report.

## Top self-time frames

From `perf-delete100-isolated-self.txt`:

```text
35.96% TransactionKind::get_page
23.12% TransactionKind::write_page_data
3.44%  BtCursor<SharedTxnPageIo>::delete
3.03%  BtCursor<SharedTxnPageIo>::table_seek_for_insert
2.99%  __memmove_avx_unaligned_erms
2.78%  _int_malloc
1.58%  Connection::try_serialize_prepared_direct_simple_insert_record
1.26%  read_cell_pointers_into
1.18%  BtCursor<SharedTxnPageIo>::load_page
0.91%  SharedTxnPageIo::read_btree_page_data
0.87%  Connection::execute_prepared_direct_simple_delete
```

`perf annotate` for `TransactionKind::get_page` places almost all local samples in the loop over transaction-local `freed_pages` membership. This matches the already-recorded 2026-05-09 negative results for pager freed-page sorted/adaptive lookup: those changes helped the long isolated delete microcase but failed the focused UPDATE/DELETE matrix by worsening the small target rows.

## Decision

No code patch from this profile.

The isolated DELETE profile identifies a real long-transaction freed-page scan cost, but the exact standalone optimization family has already been tried and rejected by the project keep gate. Retrying it would optimize the diagnostic harness while regressing the authoritative small-row UPDATE/DELETE matrix. The next viable DELETE design needs a broader same-leaf DML run operator that reduces mutation and page-state work together, not another standalone `freed_pages` lookup or retained-cursor shell.
