# Agent-Swarm Coordination Transparency Contract

Date: 2026-05-24
Bead: `bd-agent-swarm-coordination-transparency-8jr6u.1`
Status: overlap contract and architecture map for the coordination/transparency track

## Purpose

This document fixes the boundary for the `bd-agent-swarm-coordination-transparency-8jr6u`
track before implementation starts. The track exists because FrankenSQLite now
has durable agent-swarm replay and resource-governor planning, but still lacks a
SQL-visible coordination layer for applications that want the database itself to
mediate queue claims, leases, range ownership, contention diagnostics, and
fallback visibility.

The target user is running many autonomous workers against one database on
64+ core, 256GB+ RAM machines. They need to answer operational questions with
SQL and evidence artifacts:

- Which worker owns this unit of work?
- Why did this claim, lease, or transaction fail?
- Which page, key range, or statement shape is hot?
- Which statements are still using compatibility-backed execution?
- Did the replay lab and SLO governor see the same coordination behavior the
  operator saw?

This track must preserve FrankenSQLite's core invariant: concurrent-writer mode
stays on by default, plain `BEGIN` keeps promoting to `BEGIN CONCURRENT`, and no
SQLite-style serialized writer lock is introduced as a shortcut.

## Non-Goals

- Do not replace the closed deterministic agent-swarm replay lab.
- Do not replace the Swarm SLO resource governor.
- Do not create a second replay scorecard, trace schema, privacy scrubber, or
  evidence manifest stack.
- Do not implement Agent Mail or Beads inside FrankenSQLite. This track is for
  user/application SQL coordination, not for repo-agent coordination.
- Do not make single-writer mode the default or add a file/connection-level
  writer bottleneck for queue, lease, or range ownership.
- Do not add a compatibility shim for deprecated APIs. Specify and implement the
  right SQL/operator surface directly.
- Do not publish numeric performance claims unless a cited benchmark artifact
  measures the exact workload shape.

## Source Map

| Source | Status | Reuse Point | Boundary or Gap |
|---|---:|---|---|
| `bd-agent-swarm-replay-lab-1cc5y` | closed epic | Owns deterministic agent-swarm trace schema, privacy scrubber, synthetic traces, replay report, resource scorecard, evidence manifest, minimized failure slice, and CI smoke artifact. | New coordination fields should extend or adapt these artifacts. Do not fork replay infrastructure. |
| `crates/fsqlite-harness/src/agent_swarm_trace.rs` | shipped code | Canonical constants and structs for `trace_id`, `run_id`, `scenario_id`, trace statements, replay commands, high-capacity resource profiles, scorecards, evidence manifests, and structured logs. | It already has task-queue synthetic statements, but not first-class queue/lease/range ownership outcomes or EXPLAIN/fallback diagnostic rows. |
| `crates/fsqlite-harness/conformance/agent_swarm_trace_sanitized_golden.json` | shipped fixture | Golden sanitized trace for adapter and replay smoke tests. | New coordination fixtures should keep privacy scrubber expectations and add only sanitized identifiers and stable reason codes. |
| `docs/design/swarm-slo-resource-governor-overlap-manifest.md` | shipped design | Owns governor input schema, decision vocabulary, degraded-signal flags, proof-pack shape, and rch command expectations. | Coordination primitives should feed the governor through adapters and scorecards, not implement governor decisions directly. |
| `docs/concurrency-contract.md` | shipped contract | Caller-facing guarantees for single-process/multi-connection MVCC WAL, cross-process partial hardening, visibility boundaries, busy timeout, and weaker-than-stock surfaces. | New diagnostics should make these boundaries easier to inspect, not silently expand claims beyond harness evidence. |
| `docs/coverage-slo-policy.md` | shipped policy | Coverage and realism tiers for unit, property, file-backed, and e2e proof. | Downstream beads must attach concrete tests to each SQL surface instead of treating coverage as implied follow-up. |
| `crates/fsqlite-core/src/connection.rs` | shipped runtime | Connection integration point for transaction state, PRAGMAs, EXPLAIN, fallback paths, MVCC conflict observability, and concurrent-mode ratchet tests. | It is very large. Keep downstream edits narrow or factor helper modules where possible; never use it as a dumping ground. |
| `crates/fsqlite-e2e/src/lib.rs` | shipped harness | `HarnessSettings::default()` sets `concurrent_mode: true` and exports fsqlite PRAGMAs/config. | New harness settings must preserve concurrent mode by default. |
| `crates/fsqlite-e2e/src/fsqlite_executor.rs` | shipped executor | `FsqliteExecConfig::default()` sets `concurrent_mode: true` and records requested concurrent mode in reports. | New E2E configs must make any opt-out explicit and visible in reports. |
| `crates/fsqlite-e2e/src/fairness.rs` | shipped benchmark config | `benchmark_settings()` sets `concurrent_mode: true`. | New fairness/coordination benchmarks must not measure a silently serialized mode. |
| `crates/fsqlite-parser`, `crates/fsqlite-ast` | shipped SQL front end | Own syntax and AST if this track chooses new syntax such as `EXPLAIN CONCURRENCY`. | Prefer PRAGMA/table-valued surfaces when they satisfy the contract; new syntax must be justified by operator ergonomics. |
| `crates/fsqlite-planner`, `crates/fsqlite-vdbe`, `crates/fsqlite-core` | shipped execution path | Planner/VDBE/core own statement fingerprints, plan ids, execution dispatch, diagnostics, and fallback boundaries. | Diagnostics must be stable and golden-testable, not ad hoc trace-only output. |
| `crates/fsqlite-mvcc`, `crates/fsqlite-pager`, `crates/fsqlite-wal`, `crates/fsqlite-btree` | shipped storage layers | Own page conflict, version visibility, WAL publication, B-tree page/range behavior, and crash/durability interactions. | Queue/lease/range primitives may consume storage signals, but must not bypass storage invariants. |

## Downstream Surface Map

| Bead | Surface | Likely owner files or crates | Tests | Logs and artifacts |
|---|---|---|---|---|
| `.2` SQL coordination catalog and virtual table API | Contract for `fsqlite_queue`, `fsqlite_lease`, `fsqlite_worker_ranges`, diagnostics, and reason codes. | `docs/design`, then `fsqlite-parser`, `fsqlite-ast`, `fsqlite-core`, `fsqlite-vdbe::pragma` depending on chosen shape. | API examples, parser/PRAGMA tests, golden row-shape tests. | Stable reason-code registry, statement fingerprint field list, artifact contract. |
| `.3` Atomic queue claim/release | Transactional claim, release, retry, abandon, and no-double-claim behavior. | `fsqlite-core` execution integration; ordinary-table shim first, then virtual catalog table. | `crates/fsqlite-core/tests/agent_swarm_queue_claim_contract.rs` covers empty queue, already claimed row, idempotent retry, owner/generation release, abandon, rollback, and concurrent claim race. | Executable trace points carry `queue_name`, `worker_id`, `claim_attempt_id`, `statement_fingerprint`, `conflict_reason`, and `elapsed_ms`. |
| `.4` Lease heartbeat/expiration | Acquire, renew, transfer, release, expire, takeover. | `fsqlite-core` plus deterministic time/test hooks; storage integration only where durability requires it. | `crates/fsqlite-core/tests/agent_swarm_lease_contract.rs` covers missing-key acquire, non-expired takeover rejection, renew/transfer/release owner-token-generation checks, deterministic expiration boundaries, rollback, and concurrent expired-owner takeover. | `lease_key`, `owner_id`, `lease_token`, `renew_interval_ms`, `expiration_reason`, `conflict_reason`, `elapsed_ms`. |
| `.5` Worker range allocator | Disjoint rowid/key range allocation and introspection. | Planner/core first; storage/B-tree hooks only after contract proves needed. | `crates/fsqlite-core/tests/agent_swarm_worker_range_contract.rs` covers stable reason codes, allocation, renewal, release, exhausted allocation, rollback, split/merge, overlap rejection, invalid bounds, introspection, and a deterministic naive-vs-range-aware conflict model. | `range_id`, `table`, `index`, `start_key`, `end_key`, `owner_id`, `predicted_page_start`, `predicted_page_end`, `imbalance_reason`, conflict delta. |
| `.6` EXPLAIN CONCURRENCY and contention diagnostics | User-visible reason rows for expected/observed contention. | `connection.rs` EXPLAIN/PRAGMA area, planner diagnostics, MVCC conflict logs. | `crates/fsqlite-core/tests/agent_swarm_explain_concurrency_contract.rs` covers stable reason codes, low-conflict, hot-page, queue/lease/range, fallback-heavy, external-wait, summary, and rollback rows. | `trace_id`, `run_id`, `scenario_id`, `statement_fingerprint`, `plan_id`, `hotspot_kind`, `fallback_reason`, `external_wait`, `coordination_strategy`. |
| `.7` Compatibility-path fallback transparency | Aggregated fallback reasons by statement, plan, table, workload lane. | Existing compatibility/fallback dispatch paths in `fsqlite-core`, planner/VDBE lowering diagnostics. | `crates/fsqlite-core/tests/agent_swarm_fallback_transparency_contract.rs` covers stable reason codes, supported fast path, unsupported shape, mixed workload aggregation, rollback, and reset. | Stable fallback reason code, concurrency/durability/memory/latency impact class, diagnostics availability, first failure diagnostic. |
| `.8` Replay-lab and SLO bridge | Extend replay/scorecard/governor adapters to consume coordination fields. | `crates/fsqlite-harness/src/agent_swarm_trace.rs` and `crates/fsqlite-harness/src/slo_governor_adapters.rs`. | Inline harness tests cover `coordination_metrics` extraction and proof-pack propagation. | Existing replay evidence manifest plus queue/lease/range/diagnostic/fallback/governor fields, coordination correctness, conflict transparency, fairness/resource pressure, and scrubber status. |
| `.9` Unit/property/regression tests | Fast proof matrix for all coordination surfaces. | Same crates as implementation; property tests where interleavings matter. | `crates/fsqlite-core/tests/agent_swarm_coordination_test_matrix_contract.rs` pins the queue, lease, range, EXPLAIN/PRAGMA, and fallback matrix with deterministic interleavings and canonical golden rows. | Fixed seeds, exact `rch` commands, invariant ids, regression names, structured first-failure context. |
| `.10` E2E proof pack/runbook | End-to-end replay and operator interpretation. | `fsqlite-harness`, `fsqlite-e2e`, docs/runbook. | rch-backed replay and graph-health closeout. | Artifact paths, hashes, commands, score summaries, limitations. |

## Required Shared Fields

Downstream code and artifacts should converge on these field families so replay,
SLO governance, and operator diagnostics join cleanly:

- Identity: `trace_id`, `run_id`, `scenario_id`, `statement_fingerprint`,
  `plan_id`, `worker_id`, `connection_id`, `transaction_id`.
- Coordination: `queue_key`, `claim_attempt_id`, `lease_key`, `lease_owner`,
  `lease_expiration_reason`, `range_id`, `range_start`, `range_end`.
- Contention: `hotspot_kind`, `page_number`, `page_start`, `page_end`,
  `table`, `index`, `conflict_reason`, `retry_count`, `abort_count`,
  `busy_family`, `external_wait`, `coordination_strategy`.
- Fallback: `fallback_reason`, `fallback_surface`, `impact_class`,
  `supported_fast_path`, `diagnostics_available`.
- Artifact: `artifact_path`, `artifact_hash`, `replay_command`,
  `heavy_rch_command`, `first_failure_diag`.

Reason codes must be stable enough for golden tests. Human text can evolve, but
automation should key on the code.

## Concurrency Invariants

Every downstream bead must preserve these current-state facts:

- `crates/fsqlite-core/src/connection.rs` initializes
  `concurrent_mode_default: RefCell::new(true)` for connection construction
  paths.
- Plain `BEGIN` promotes to concurrent mode unless the user explicitly turns
  `PRAGMA fsqlite.concurrent_mode` off.
- `HarnessSettings::default()` in `crates/fsqlite-e2e/src/lib.rs` keeps
  `concurrent_mode: true`.
- `FsqliteExecConfig::default()` in
  `crates/fsqlite-e2e/src/fsqlite_executor.rs` keeps
  `concurrent_mode: true`.
- `benchmark_settings()` in `crates/fsqlite-e2e/src/fairness.rs` keeps
  `concurrent_mode: true`.
- Queue, lease, and range ownership may create logical conflicts, but those
  conflicts must be mediated through page-level MVCC/storage semantics. They
  must not introduce a global writer lock.
- File-lock fallback remains a loud correctness fallback, not the normal
  coordination path for massive agent swarms.

## Rejected or Avoided Overlaps

- Replay trace schema fork: rejected. Extend
  `crates/fsqlite-harness/src/agent_swarm_trace.rs` or its adapters.
- Second resource governor: rejected. Feed the existing Swarm SLO governor
  schema and decision vocabulary.
- Production metrics endpoint: rejected for this track. Integrate with the
  existing production telemetry bead when it exists, and keep interim sampling
  private to tests/harnesses.
- Agent Mail clone: rejected. Agent Mail coordinates coding agents in this repo;
  FrankenSQLite coordination primitives coordinate user workload rows.
- Beads clone: rejected. Beads tracks development issues; SQL queue/lease/range
  primitives are generic application building blocks.
- Whole-file `connection.rs` refactor: rejected as a prerequisite. Use narrow
  integration points or extracted helpers when implementation requires code.

## Verification Expectations

Each implementation bead must include:

- Unit tests for normal, empty, boundary, rollback, and error paths.
- Property or deterministic interleaving tests where ownership races matter.
- Golden tests for diagnostic row/JSON output, including rollback behavior for
  unpublished diagnostic rows.
- E2E/replay tests only where cross-layer behavior is required.
- Structured logging with `trace_id`, `run_id`, `scenario_id`, timing/counter
  fields, stable reason codes, and first-failure diagnostics.
- Heavy verification commands routed through foreground `timeout ... rch exec --`
  commands. Local cargo-only examples may appear as developer smoke commands,
  but closeout proof for CPU-heavy replay belongs on rch.
- `br dep cycles` plus robot-only `bv` graph evidence after Beads edits.

## Recommended Order

1. Use this document as the manifest for `.2`.
2. Finalize the SQL/API shape before parser, VDBE, or storage work starts.
3. Build queue and lease primitives as small vertical slices before range
   allocation.
4. Add EXPLAIN/PRAGMA concurrency and fallback transparency against those
   primitives.
5. Bridge the new fields into the replay lab and SLO governor.
6. Close with unit/property/golden tests, rch-backed replay evidence, and an
   operator runbook.

## Open Questions

- Whether the first public surface should be PRAGMA/table-valued diagnostics or
  new SQL syntax. The default should be the smaller parser surface unless
  operator examples show `EXPLAIN CONCURRENCY` materially improves usability.
- Whether queue/lease/range primitives live as virtual tables, table-valued
  functions, or recognized ordinary DML patterns. The `.2` API contract owns
  that decision.
- Which fallback reason codes are already available in planner/VDBE/core logs
  and which need new instrumentation.
- Whether range allocation should be advisory-only at first or enforceable in
  the same initial slice. The safer path is advisory plus diagnostics before
  enforcement.
