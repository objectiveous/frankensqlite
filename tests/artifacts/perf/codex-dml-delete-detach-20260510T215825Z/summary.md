# DML Delete Leaf-Run Detach Candidate

Date: 2026-05-10

Candidate: detach the current btree leaf stack entry into `TableLeafDeleteRun`
instead of cloning it, then move the owned page into the write at flush time.

Verdict: rejected; source reverted.

The focused `UPDATE/DELETEThroughput` gate used
`FSQLITE_BENCH_PROFILE_DML=1 comprehensive-bench --quick --filter update`.
The baseline binary was the current code-equivalent `98aee4f8` release-perf
build. The candidate was rebuilt from the dirty worktree containing only this
candidate.

| Scenario | Baseline F ms | Candidate F ms | Result |
| --- | ---: | ---: | --- |
| 100 rows / delete 5 rows | 0.007845 | 0.008466 | worse |
| 1000 rows / delete 50 rows | 0.055053 | 0.033663 | better but noisy |
| 10000 rows / delete 500 rows | 0.308358 | 0.351609 | worse |

The profile showed the underlying problem: detaching the leaf image did not
reduce the total flush/mutate path. The final candidate still spent
`104742 ns` in delete leaf-run flush for the 500-delete row and regressed the
median from `0.308358 ms` to `0.351609 ms`.

Artifacts:

- `baseline-dml.json`, `baseline-dml.stdout`, `baseline-dml.stderr`
- `candidate-dml.json`, `candidate-dml.stdout`, `candidate-dml.stderr`
- `candidate-move-dml.json`, `candidate-move-dml.stdout`,
  `candidate-move-dml.stderr`
