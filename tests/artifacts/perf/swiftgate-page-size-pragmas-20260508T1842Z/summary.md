# Default page-size PRAGMA setup skip rejection

Date: 2026-05-08

Candidate: skip explicit default `PRAGMA page_size = 4096` in
`comprehensive_bench.rs` benchmark setup, while preserving non-default page-size
experiments. The standalone `perf_update_delete` harness also dropped its fixed
page-size PRAGMA from both engines.

Outcome: rejected and source restored.

## Evidence

- Clean baseline build: local fallback from detached `HEAD` worktree
  `/data/tmp/frankensqlite-clean-page-size-pragmas-20260508T1842Z` using target
  `/data/tmp/frankensqlite-page-size-skip-clean-target`.
- Candidate build: `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-swiftgate-page-size-skip-target CARGO_BUILD_JOBS=12 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete`
  passed; the runnable candidate binary was retrieved under
  `/data/projects/frankensqlite/.rch-target/release-perf/comprehensive-bench`.
- Baseline INSERT:
  `baseline-insert.json`
- Candidate INSERT:
  `candidate-insert-rerun.json`
- Baseline UPDATE:
  `baseline-update.json`
- Candidate UPDATE:
  `candidate-update.json`

## Focused summaries

| Gate | Run | Avg | Geomean | Weighted | P90 | P99 | Faster / Comparable / C faster |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| INSERT | baseline | 0.8196315648 | 0.7957682955 | 0.7701934312 | 1.1152752480 | 1.1218613772 | 17 / 3 / 5 |
| INSERT | candidate | 0.8020402350 | 0.7762532618 | 0.7921862328 | 1.1105214365 | 1.1533843404 | 18 / 2 / 5 |
| UPDATE | baseline | 0.9880400096 | 0.9661435934 | 0.9661435934 | 1.2963600941 | 1.2963600941 | 4 / 0 / 2 |
| UPDATE | candidate | 1.0352160705 | 1.0133651619 | 1.0133651619 | 1.3488429318 | 1.3488429318 | 4 / 0 / 2 |

The INSERT gate had lower average/geomean but worse weighted and P99 ratios.
The UPDATE/DELETE gate worsened across aggregate and tail metrics, so the
candidate did not proceed to a full quick matrix.

## Notable slower rows

- Baseline UPDATE: `100 rows / update 10 rows` ratio `1.2929143229`;
  `100 rows / delete 5 rows` ratio `1.2963600941`.
- Candidate UPDATE: `100 rows / update 10 rows` ratio `1.3441904384`;
  `100 rows / delete 5 rows` ratio `1.3488429318`.
- Candidate INSERT still had five C-faster rows, including
  `large_10col` 100-row single transaction at ratio `1.1533843404` and
  `small_3col` 100-row batched at ratio `1.1464539924`.

## Retry condition

Do not retry default page-size PRAGMA skipping as a standalone cleanup. Revisit
only if benchmark setup is moved outside the measured closure entirely and a
same-window run improves focused INSERT, focused UPDATE/DELETE, and the full
quick matrix weighted and tail ratios.
