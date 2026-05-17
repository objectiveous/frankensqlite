# DML HEAD Refresh

- Date: 2026-05-17 02:05:26 UTC.
- Source: `main @ e97fac94b88e0b5bdead27fa34386678fc1d31da`.
- Tracked source state before the run: clean. The benchmark reports `Git dirty: yes`
  because this checkout still has unrelated untracked files plus the artifact
  directory created for this run.
- Command:
  `FSQLITE_BENCH_PROFILE_DML=1 /data/tmp/frankensqlite-dml-head-e97fac94-target/release-perf/comprehensive-bench --quick --filter update-delete --json-out tests/artifacts/perf/codex-dml-head-e97fac94-20260517T020044Z/update-delete.json --no-html`
- Raw artifacts:
  - `tests/artifacts/perf/codex-dml-head-e97fac94-20260517T020044Z/run.log`
  - `tests/artifacts/perf/codex-dml-head-e97fac94-20260517T020044Z/update-delete.json`

## Matrix

| Scenario | C SQLite | FrankenSQLite | Ratio | CV C | CV F |
| --- | ---: | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 4.2 us | 9.9 us | 2.34x slower | 2.9% | 5.7% |
| 100 rows / delete 5 rows | 3.6 us | 7.2 us | 2.01x slower | 23.4% | 46.7% |
| 1000 rows / update 100 rows | 37.2 us | 29.3 us | 1.27x faster | 15.7% | 2.3% |
| 1000 rows / delete 50 rows | 16.2 us | 31.0 us | 1.91x slower | 1.8% | 24.5% |
| 10000 rows / update 1000 rows | 357.0 us | 260.8 us | 1.37x faster | 11.4% | 12.7% |
| 10000 rows / delete 500 rows | 164.0 us | 259.6 us | 1.58x slower | 5.9% | 1.0% |

Summary: 6 scenarios, FrankenSQLite faster/comparable/C-SQLite-faster = `2/0/4`,
average ratio `1.56x`, geomean ratio `1.42x`.

## 10K/500 DELETE Hotspot Table

| Rank | Location / counter | Metric | Value | Category | Evidence |
| ---: | --- | ---: | ---: | --- | --- |
| 1 | retained active leaf-run path | cumulative | `delete_leaf_active=433/496`, `delete_leaf_active_ns=140985` | CPU | `run.log` `fs_delete_10000` profile |
| 2 | retained leaf search | cumulative | `delete_leaf_search=560/40581` | CPU | `run.log` `fs_delete_10000` profile |
| 3 | retained leaf materialization | cumulative | `delete_leaf_materialize=64/40017` | CPU/copy | `run.log` `fs_delete_10000` profile |
| 4 | ordinary fallback seeks/deletes | cumulative | `delete_seek_ns=34880`, `delete_physical_ns=13896` | CPU/B-tree | `run.log` `fs_delete_10000` profile |
| 5 | commit envelope | cumulative | `commit_us=42.5`, `commit_roundtrip_ns=22873`, `pager_mem_flush_ns=9448` | commit | `run.log` `fs_delete_10000` profile |

## Hypothesis Ledger

- Missing direct DELETE fast path: rejects. `direct_delete=500`, `slow=0`, and
  `vdbe_opcodes=0` prove the benchmark is already on the dedicated prepared
  direct path.
- Staged-run-only bottleneck: rejects. `delete_leaf_miss_staged=0` again.
- Retained leaf-run micro-tweak: rejects as a next source lever. The current
  profile still spreads cost across active-run search, materialization, fallback
  seeks, and commit; the negative ledger already fences standalone search,
  duplicate-check, compactness, materializer, parent-separator, last-cell,
  direct-flush, dense-rowid, and staged-run patches.
- Broader transaction-local row/key mutation operator: supports. The measured
  gap is still the cumulative per-row retained-leaf ceremony plus page-image
  publication. A keepable source change must replace that with a logical
  row/key mutation operator that proves affected counts, read-your-writes,
  rollback/savepoint ownership, and MVCC/page-cell publication before the focused
  matrix and full quick gate.

## Next Decision

Do not implement another retained leaf-run micro-optimization from this profile.
The next source attempt should start at the transaction-local row/key DML
mutation boundary or pivot to another measured red workload such as large-row
INSERT or low-thread shared-table concurrency.
