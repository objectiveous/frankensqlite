# Prepared Direct DELETE candidate ledger - 2026-05-11

Baseline:

- Artifact: `tests/artifacts/perf/codex-current-dml-profile-20260511T205339Z/`
- Commit: `d18caf88d7f83858915c487deefa90dc90eee8a9`
- Focused command: `FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update-delete --no-html`
- DELETE medians: 100 rows `0.006863 ms`, 1000 rows `0.044454 ms`, 10000 rows `0.262732 ms`
- 10k DELETE counters: `delete_leaf_active_ns=50217`, `delete_leaf_flush_ns=86854`, `delete_leaf_materialize=64/73639`

Rejected candidates:

1. Materialization threshold (`SMALL_DELETE_INCREMENTAL_LIMIT=2`)
   - Artifact: `tests/artifacts/perf/codex-delete-threshold2-profile-20260511T210322Z/`
   - Result: rejected; 10k `delete_leaf_materialize` worsened to `64/134592 ns`, and 10k ratio worsened to `1.60369`.

2. Direct writer flush
   - Artifact: `tests/artifacts/perf/codex-delete-direct-writer-20260511T211346Z/`
   - Result: rejected; 10k DELETE median worsened to `0.267832 ms` and `delete_leaf_flush_ns` worsened to `95087 ns`.

3. Retained-leaf search hint
   - Artifacts: `tests/artifacts/perf/codex-delete-search-hint-20260511T212300Z/`, `tests/artifacts/perf/codex-delete-search-hint-repeat-20260511T212420Z/`
   - Result: rejected; active-search counters improved, but 10k DELETE did not hold in the repeat run (`0.267151 ms`) and 100-row DELETE regressed/noised upward.

Conclusion:

- The current retained same-leaf DELETE implementation is already close to its local optimum for the focused quick matrix.
- The remaining DELETE gap is not a small threshold, cursor-wrapper, or search-bound issue. The next credible source-level target is the broader transaction-local many-leaf mutation representation described in `tests/artifacts/perf/codex-frontier-boundary-20260510T221343Z/summary.md`.
