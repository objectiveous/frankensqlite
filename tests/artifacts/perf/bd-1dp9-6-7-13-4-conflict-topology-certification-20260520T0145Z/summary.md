# bd-1dp9.6.7.13.4 Conflict-Topology Certification

- Result: `pass`
- Run ID: `bd-1dp9.6.7.13.4-20260520T0145Z-67134`
- Scenario ID: `T6-7-13-4-CONFLICT-TOPOLOGY-CERT`
- Fixed seed: `67134`
- Real-path workload: `mt-mvcc-bench --rows-per-thread=300 --threads=8 --iters=3`
- Structured events: `/data/projects/frankensqlite/tests/artifacts/perf/bd-1dp9-6-7-13-4-conflict-topology-certification-20260520T0145Z/events.jsonl`
- Report: `/data/projects/frankensqlite/tests/artifacts/perf/bd-1dp9-6-7-13-4-conflict-topology-certification-20260520T0145Z/report.json`

## Current Replay

- 8 threads: p50 21.618->50.785 ms, p95 22.273->65.139 ms, p99 22.331->66.415 ms, wps 111020->47258, improved=False

## Cumulative Replayable Evidence

- 2 threads: p50 7.146->2.976 ms, p95 7.196->7.050 ms, p99 7.200->7.413 ms, wps 83963->201631, improved=True
- 4 threads: p50 7.539->8.475 ms, p95 11.361->10.591 ms, p99 11.701->10.779 ms, wps 159173->141595, improved=False
- 8 threads: p50 25.680->21.486 ms, p95 27.122->29.965 ms, p99 27.251->30.718 ms, wps 93459->111700, improved=True

## Replay Commands

```text
bash scripts/verify_t6_7_conflict_topology.sh
```
