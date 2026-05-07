# UPDATE/DELETE Profile

- Agent: CrimsonGorge
- Commit: `0f6a2fd67c8ae578e5616922b27fe3d70666959c`
- Worktree: `/data/tmp/frankensqlite-txn-profile-crimsongorge-20260507T0845Z`
- Build target: `/data/tmp/frankensqlite-crimsongorge-main-release`
- Command: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update --json-out report-update-delete.json --no-html`

## Result

The remaining UPDATE/DELETE gap is fixed-cost dominated at the small row counts.
The direct mutation lanes themselves are already extremely small.

| Row | C SQLite | FrankenSQLite | Ratio | Profile signal |
| --- | ---: | ---: | ---: | --- |
| 100 rows / update 10 rows | 100.6 us | 134.0 us | 1.33x slower | `setup_us=57.3`, `prepare_us=21.0`, `mutate_us=12.0`, `commit_us=6.1` |
| 100 rows / delete 5 rows | 85.0 us | 124.9 us | 1.47x slower | `setup_us=60.7`, `prepare_us=18.1`, `mutate_us=9.1`, `commit_us=5.7` |
| 1000 rows / update 100 rows | 431.5 us | 461.2 us | 1.07x slower | direct lane stays fast; setup is now most of the row |
| 1000 rows / delete 50 rows | 394.7 us | 394.0 us | effectively tied | direct delete is not the current priority |
| 10000 rows / update 1000 rows | 3.87 ms | 4.19 ms | 1.08x slower | profile setup is about 2.78 ms; mutation is about 1.23 ms |
| 10000 rows / delete 500 rows | 3.63 ms | 3.77 ms | 1.04x slower | profile setup is about 2.79 ms; mutation is about 0.72 ms |

## Interpretation

- The DML profiler confirms UPDATE/DELETE rows are not primarily losing inside VDBE dispatch: `fast=count`, `slow=0`, `vdbe_opcodes=0`, and direct update/delete counters match the mutation count.
- The smallest rows are dominated by per-scenario setup and prepare costs. For 100-row update, the direct mutation loop is only about 12 us out of the profiled run.
- The 10K rows are close enough that setup/populate dominates the section score. This means INSERT transaction-strategy work remains the higher-EV performance target for closing the matrix gap.
- The actionable seam is still connection-level fixed overhead and direct INSERT setup/page-run behavior. `crates/fsqlite-core/src/connection.rs` was exclusively reserved by PurpleOtter during this profile, so this artifact records evidence without editing that file.
