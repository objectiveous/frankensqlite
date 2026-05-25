# Swarm SLO Resource Governor Overlap Manifest

Date: 2026-05-24
Bead: `bd-swarm-slo-resource-governor-qb256.2`
Status: current-state manifest for the Swarm SLO governor epic

## Purpose

This manifest fixes the reuse points and boundaries for the Swarm SLO resource
governor before implementation starts. The governor must join existing
guardrail, replay, metrics, and operator surfaces into one operational control
plane for high-core agent-swarm workloads.

The governor must preserve FrankenSQLite's core invariant: concurrent-writer
mode stays on by default, `BEGIN` keeps promoting to `BEGIN CONCURRENT`, and no
SQLite-style serialized writer lock is introduced as a resource-control
shortcut.

## Non-Goals

- Do not replace the closed admission-control guardrail design.
- Do not replace the closed deterministic agent-swarm replay lab.
- Do not create a competing production metrics endpoint.
- Do not create a competing periodic doctor scheduler.
- Do not fix the rch/background wedge or UBS large-file scanner bugs inside the
  governor track.
- Do not move governor logic into a giant `connection.rs` patch unless a later
  implementation pass proves that is the smallest correct edit surface.

## Source Map

| Source | Status | Reuse Point | Boundary or Gap |
|---|---:|---|---|
| `bd-db300.5.4.3` and `docs/design/admission-control-and-tail-latency-guardrails.md` | closed design | Canonical guardrail inputs, decisions, actions, safe-mode rules, structured log fields, and verification expectations. | It is a design contract, not a shipped Swarm SLO policy engine. The new governor should implement or adapt its policy vocabulary without weakening its invariants. |
| `docs/design/queue-depth-wake-to-run-and-helper-lane-budgets.md` | design | Queue depth, wake-to-run, evidence-lane budgets, GC depth ladder, and helper-lane scaling rules. | Runtime sampling and adapters still need to decide which signals are present in replay, live harnesses, and production telemetry. |
| `docs/design/bounded-many-core-scheduling-and-offload-rules.md` | design | Work classes, lane budgets, safe-mode circuit breaker, helper-thread limits, and the E4 one-paragraph policy. | The governor must use these as limits on helper work. It must not convert safe mode into file-level writer serialization. |
| `bd-agent-swarm-replay-lab-1cc5y` | closed epic | Deterministic replay lab, trace schema, sanitized traces, cross-backend reports, scorecards, and evidence manifests. | Replay artifacts do not yet carry a governor policy decision or admission output. That mapping belongs to the adapter bead. |
| `crates/fsqlite-harness/src/agent_swarm_trace.rs` | shipped code | Trace schema constants, synthetic scenarios, replay reports, backend rows, resource profiles, scorecards, evidence manifests, and CI smoke metadata. | Current scorecards expose p50/p95/p99; p999 and some live-only pressure signals must be marked degraded or absent until a live source exists. |
| `scripts/verify_bd_073kf_swarm_harness.sh` and `crates/fsqlite-e2e/tests/bd_073kf_swarm_harness.rs` | shipped harness | Existing swarm binary, determinism, JSONL, heartbeat, and required-field gates. | Do not copy bare long-running cargo command patterns for agent runs. Heavy verification in this repo should use foreground `timeout ... rch exec -- ...` commands. |
| `crates/fsqlite-harness/conformance/agent_swarm_trace_sanitized_golden.json` | shipped fixture | Golden trace identity and replay fixture for adapter tests. | It is a fixture, not a policy oracle. Expected governor decisions should be asserted in new golden adapter tests. |
| `bd-zywqc.11` | open | Production telemetry target: metrics registry, callable API, Prometheus export, and privacy constraints. | The governor should integrate with this metrics surface when available and keep any interim internal sampling private. It must not publish SQL text, table names, row values, or user data as labels. |
| `bd-316l0` | open | Periodic doctor concepts: workspace config, rolling logs, thresholds, notification severity, and operator-facing diagnostics. | The governor should expose doctor-readable state after policy/adapters exist, not implement a second scheduler. |
| `bd-17uo0` | open bug | Operational rule for avoiding wedged agent runs: no backgrounded long `rch` or script commands; use foreground timeouts and clear process ownership. | The governor proof pack must use this as verification guidance, not attempt to fix Codex process supervision. |
| `bd-yxsqo` | open bug | UBS can hang on very large `connection.rs` scans. Keep governor implementation and tests in small files where practical and use scoped checks. | Do not treat the UBS workaround as a reason to skip review of affected hunks. Any `connection.rs` edit needs manual inspection and scoped verification. |
| `docs/critical-invariants.md` and `crates/fsqlite-core/src/connection.rs` | shipped invariant and tests | `INV-C1` plus `test_concurrent_mode_default_on` are the hard regression guard. | Every governor rollout gate must prove defaults remain concurrent and no file-level writer serialization was added. |
| `docs/bench-methodology-concurrent-writers.md` | shipped methodology | Current concurrent-writer claims require one connection per worker thread, a shared file-backed database, disjoint rowid ranges, and matched baselines. | Governor performance claims must cite artifacts that measure this workload shape, or explicitly say the claim is unmeasured. |

## Shared Governor Input Schema

All implementation beads should converge on one input record so replay, live
harness, and production paths can be compared without translation drift.

Required identity fields:

- `run_id`, `trace_id`, `scenario_id`, `backend`, `profile_id`,
  `control_mode`
- `artifact_path` and `artifact_hash` when sourced from replay evidence
- `sample_ts` and `sample_window_ms` when sourced from live telemetry

Required workload fields:

- `actor_count`, `connection_count`, `writer_count`, `reader_count`
- `statement_count`, `transaction_count`, `concurrency_level`
- `schedule_seed` or `schedule_fingerprint` for replayed workloads

Required resource fields:

- `available_cores`, `configured_helper_threads`, `active_helper_threads`
- `memory_limit_bytes`, `memory_high_water_bytes`, `page_cache_bytes`
- `cpu_utilization_pct`, `resource_profile`

Required pressure fields:

- `active_writers`
- `publish_window_occupancy` and `publish_window_p99_ms` when available
- `retry_count`, `retry_rate`, `abort_count`, `abort_rate`
- `evidence_queue_depth`, `evidence_queue_drops`, `evidence_worker_count`
- `wakeup_queue_depth`, `wakeup_to_run_p95_ms`, `wakeup_to_run_p99_ms`
- `max_chain_depth`, `gc_tier`
- `wal_frames_pending_checkpoint`, `checkpoint_active`
- `invalidation_queue_depth`, `invalidation_fallback_count`
- `build_or_test_saturation` and `agent_wedge_risk` for operator proof runs

Required coordination bridge fields:

- `queue_claim_count`, `queue_release_count`, and derived pending queue depth
- `lease_acquire_count`, `lease_renew_count`, and `lease_expiration_count`
- `range_allocation_count` and `range_imbalance_reason`
- `explain_concurrency_row_count` and `fallback_reason_count`
- `resource_governor_decision_count`
- `coordination_correctness_per_mille`
- `conflict_transparency_per_mille`
- `fairness_resource_pressure_per_mille`
- `privacy_scrubber_preserved`

Required latency fields:

- `latency_p50_ms`, `latency_p95_ms`, `latency_p99_ms`
- `latency_p999_ms` only when measured; otherwise add a degraded flag

Required degraded-signal fields:

- `missing_p999`
- `missing_publish_window`
- `missing_live_metrics`
- `stale_metrics`
- `replay_only_input`
- `privacy_redacted_input`

## Decision Vocabulary

The shadow policy engine must reuse the E4.3 action vocabulary unless a later
design pass records an explicit reason to change it:

- `Admit`
- `Defer`
- `ApplyBackpressure`
- `ShrinkHelperBudget`
- `ForceSafeMode`
- `TriggerEmergencyGc`
- `TriggerCheckpoint`

Every decision should carry:

- `policy_id`, `decision_id`, `guardrail_id`
- `evidence`
- `action`
- `counterfactual`
- `regret`
- `degraded_signals`
- `concurrent_mode_default_observed`

## Ownership Boundaries

`bd-swarm-slo-resource-governor-qb256.1` owns the pure shadow-mode policy
engine. It should be deterministic, testable without I/O, and cover thresholds,
hysteresis, missing signals, degraded telemetry, and concurrent-mode default
guards.

`bd-swarm-slo-resource-governor-qb256.4` owns replay and live signal adapters.
It maps replay scorecards, evidence manifests, live harness samples, and later
production metrics into the shared schema above. It must integrate with
`bd-zywqc.11` instead of exposing a new metrics endpoint.

`bd-swarm-slo-resource-governor-qb256.5` owns operator output. It should present
SLO budgets, current decisions, degraded signal quality, fallback-path risk,
kill switches, and safe-mode guidance without implying unmeasured performance
wins.

`bd-swarm-slo-resource-governor-qb256.6` owns the proof pack. It should collect
smoke, heavy replay, graph, and invariant evidence into artifact-backed output.
CPU-heavy proof commands must be foreground `rch` runs with timeouts.

The agent-swarm coordination closeout
`bd-agent-swarm-coordination-transparency-8jr6u.10` reuses that same proof-pack
surface. It adds `operator_runbook` rows to
`SwarmSloReplayStressProofPack`, not a new governor or replay artifact. The
runbook must expose the focused proof command, trace/proof-pack artifact paths,
coverage counts for queue/lease/range/diagnostic/fallback/governor surfaces,
operator query shapes, graph-health commands, and explicit unmeasured-claim
limits.

`bd-swarm-slo-resource-governor-qb256.3` owns enforced-mode rollout. It should
remain opt-in until the proof pack passes, include a kill switch, and prove that
missing or stale telemetry fails toward shadow-only recommendations rather than
unsafe enforced throttling.

## Evidence Requirements

Before enforced mode can be considered, the track needs these artifacts:

- Unit tests for policy thresholds, hysteresis, safe mode, emergency actions,
  missing signals, stale metrics, and concurrent-mode default observation.
- Golden adapter tests using
  `crates/fsqlite-harness/conformance/agent_swarm_trace_sanitized_golden.json`.
- Replay-smoke evidence from the existing swarm replay smoke target.
- Coordination E2E proof-pack runbook evidence from:

```bash
timeout 900 rch exec -- env CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/data/tmp/frankensqlite-target-agent-swarm-proof-pack cargo test -p fsqlite-harness --lib coordination_e2e_proof_pack_covers_operator_runbook -- --nocapture
```

- Heavy replay or scorecard evidence using a foreground offloaded command, for
  example:

```bash
timeout 1200 rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-agent-swarm-scorecard cargo test -p fsqlite-harness agent_swarm_scorecard -- --nocapture
```

- Graph-health evidence from robot-only commands such as
  `bv --robot-insights --label swarm-slo`.
- Beads export evidence from `br sync --flush-only` after status changes.
- UBS or equivalent scoped checks for files changed by each implementation
  pass. If `connection.rs` is touched, manually inspect the hunk and avoid
  relying on a whole-file UBS scan.
- README performance-claim updates only when the cited artifact measures the
  exact workload shape being claimed.

## Current Risks and Mitigations

`bd-db300.5.3` remains open in the broader admission-control chain. If a later
implementation depends on metadata-publication behavior from that track, the
governor must either feature-gate the integration or record the missing signal
as degraded.

Replay scorecards currently provide p50/p95/p99 but not a universal p999.
Adapters must not invent p999. They should set `missing_p999` and let policy
tests cover the degraded path.

The production telemetry bead is open. Until it lands, adapters can use local
internal sampling for tests and harnesses, but public metrics exposure belongs
to `bd-zywqc.11`.

Existing verification scripts include useful field checks but may use local
cargo directly. New heavy proof recipes for this repo should use foreground
`rch` execution and a timeout.

The giant `connection.rs` file is a review and tooling risk. Prefer a small
policy module and adapter tests when practical; if integration requires touching
`connection.rs`, keep the hunk narrow and cite the invariant tests that guard
concurrent-mode defaults.

## Recommended Implementation Order

1. Finish this manifest and close `bd-swarm-slo-resource-governor-qb256.2`.
2. Implement the pure shadow policy engine in
   `bd-swarm-slo-resource-governor-qb256.1`.
3. Wire replay and live-harness adapters in
   `bd-swarm-slo-resource-governor-qb256.4`.
4. Expose operator output in `bd-swarm-slo-resource-governor-qb256.5`.
5. Build the artifact-backed proof pack in
   `bd-swarm-slo-resource-governor-qb256.6`.
6. Gate opt-in enforced rollout in `bd-swarm-slo-resource-governor-qb256.3`.
