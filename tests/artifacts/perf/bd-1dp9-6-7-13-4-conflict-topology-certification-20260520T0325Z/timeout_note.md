# bd-1dp9.6.7.13.4 Partial Certification Timeout

- Run ID: `bd-1dp9.6.7.13.4-20260520T0325Z-67134`
- Command: `timeout 25m bash scripts/verify_t6_7_conflict_topology.sh`
- Result: partial certification evidence captured; local `mt_mvcc_build` was stopped before the full replay completed.
- Concurrent default: verified `concurrent_mode_default: RefCell::new(true)`.
- Passed phases: source log contract, MVCC adversarial overlap heat graph, B-tree certification matrix, hot-page deflection, and heat-driven split policy.
- Stopped phase: `mt_mvcc_build` local release-perf build for `mt-mvcc-bench`; no benchmark rows were produced in this run.

Replay evidence retained from the bounded reruns:

- `tests/artifacts/perf/bd-1dp9-6-7-13-4-conflict-topology-certification-20260520T0255Z/mt_mvcc_baseline.json`
  recorded an 8-thread clean baseline with FSQLite p50/p95/p99 `28.790/36.089/36.737 ms`,
  `83363` rows/s, `2.083x` throughput versus SQLite, and zero failed rows.
- `tests/artifacts/perf/bd-1dp9-6-7-13-4-conflict-topology-certification-20260520T0145Z/report.json`
  completed the certification report with required structured fields and cumulative replayable evidence.
- Cumulative child evidence remains the acceptance proof: `.13.1` heat telemetry and overlap graph at `62f2b8d9`,
  `.13.2` topology split policy at `9ef87fdc`, and `.13.3` bounded hot-page deflection at `8aae2b17`.
