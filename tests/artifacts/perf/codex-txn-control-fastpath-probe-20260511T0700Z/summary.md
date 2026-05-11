# Exact Transaction-Control Fast Path Probe - 2026-05-11

Purpose: rejected retry of the exact `BEGIN` / `COMMIT` / `ROLLBACK`
`Connection::execute` fast path against the post-scratch-reset frontier.

Candidate shape:

- Detect exact transaction-control SQL after `background_status()`.
- Bypass parser, rewrite, and generic statement dispatch when tracing and
  `trace_v2` observability are inactive.
- Route directly to existing transaction helpers and preserve successful
  statement-count accounting.

Correctness proof before rejection:

- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-head-profile-target CARGO_BUILD_JOBS=8 cargo test -p fsqlite-core test_lifecycle_exact_transaction_control_execute_bypasses_parser_when_untraced -- --nocapture`
  passed.
- The source patch and targeted test were removed after the full-quick gate
  rejected the candidate.

Focused DML comparison against
`../codex-current-head-dml-hotpath-20260511T0652Z/update-delete.json`:

| Scenario | Baseline F ms | Candidate run1 F ms | Candidate run2 F ms | Baseline ratio | Run1 ratio | Run2 ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.006582 | 0.005951 | 0.006292 | 1.5172890733 | 1.3878264925 | 1.5168756027 |
| 100 rows / delete 5 rows | 0.007835 | 0.007334 | 0.007574 | 3.5055928412 | 3.1694036301 | 3.2590361446 |
| 1000 rows / update 100 rows | 0.030597 | 0.030567 | 0.031068 | 0.8455246359 | 0.8095074153 | 0.8387915440 |
| 1000 rows / delete 50 rows | 0.032220 | 0.031769 | 0.030768 | 2.0303736845 | 1.9573039246 | 1.9205992509 |
| 10000 rows / update 1000 rows | 0.269325 | 0.273653 | 0.274454 | 0.7301530929 | 0.7021983069 | 0.7465312440 |
| 10000 rows / delete 500 rows | 0.291786 | 0.290814 | 0.291746 | 1.8302054846 | 1.7850658319 | 1.8247924993 |

Full-quick rejection against the kept scratch-reset artifact
`../codex-delete-scratch-guard-probe-20260511T051951Z/fullquick2/full-quick.json`:

| Metric | Kept baseline | Candidate fullquick1 | Candidate fullquick2 |
| --- | ---: | ---: | ---: |
| Weighted score | 0.3737218607 | 0.3796332426 | 0.3767633146 |
| Average ratio | 0.4986768718 | 0.4996856920 | 0.5048516714 |
| Geomean ratio | 0.2728731125 | 0.2794292909 | 0.2773252427 |
| Median ratio | 0.2938174301 | 0.3030926291 | 0.2966057780 |
| p90 ratio | 1.0392045236 | 1.0869783941 | 1.0777721249 |
| p99 ratio | 3.4486373166 | 3.1637149028 | 3.2107969152 |
| F/comparable/C rows | 80/4/9 | 80/2/11 | 79/4/10 |

Decision: reject. The focused DML micro-effect was real but noisy, and both
full-quick runs worsened the primary weighted score plus broad distribution
metrics. This strengthens the existing "do not retry standalone exact
transaction-control execute bypass" rule.
