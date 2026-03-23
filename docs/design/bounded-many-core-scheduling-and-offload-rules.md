# Bounded Many-Core Scheduling and Offload Rules — E4 Parent Design Record

**Bead:** `bd-db300.5.4` (E4)
**Date:** 2026-03-23
**Status:** Accepted parent design record synthesizing E4.1 + E4.2 + E4.3
**Children:** E4.1 (work classification), E4.2 (lane budgets), E4.3 (guardrails)
**Cross-inputs:** G8.3 (interference decision map), E4 composition input (for G8.4)

---

## 1. One-Paragraph Policy

Writers do all correctness-critical work inline on their own core. Post-commit
cleanup runs on 2 shared helper threads (evidence + GC) with bounded queues.
When writer lanes saturate or tails blow up, the system applies graduated
backpressure — first parking new BEGINs, then rejecting with SQLITE_BUSY —
rather than growing queues or spawning threads. Safe mode collapses all
offloaded work back to inline execution as a circuit breaker. Concurrent-writer
mode is never disabled by any scheduling decision.

---

## 2. Work Classification (from E4.1)

Every commit-path operation is one of four classes:

| Class | Contract | Budget | Example |
|-------|----------|--------|---------|
| **IC** (Inline-Critical) | Must complete before commit is visible | ≤ 500ns/page p99 | SSI validation, CommitIndex publish, lock release |
| **IF** (Inline-Fast) | Must complete before COMMIT returns | ≤ 5μs total p99 | Waiter wakeup, session recycle, txn cleanup |
| **OA** (Offload-Async) | May complete after COMMIT returns | ≤ 100μs amortized | Evidence recording, invalidation, VTab notify |
| **OB** (Offload-Background) | Best-effort, bounded queue | Budgeted | GC sweep, checkpoint, snapshot capture |

**Publish window** (the IC region):
```
SSI validation → pager WAL write → commit clock (1 atomic) →
CommitIndex batch_update (1 fence + N stores) → lock release (N CAS)
```
The WAL write dominates for file-backed DBs. For `:memory:`, the atomic
operations dominate. Waiter wakeup is IF (outside the window).

**Fallback default:** Any unclassified operation defaults to IF (safe but
observable). Never default to OA/OB — incorrect offloading can silently violate
visibility invariants.

---

## 3. Lane Budgets (from E4.2)

| Lane | Queue Depth | Wake-to-Run | Workers | Backpressure |
|------|-------------|-------------|---------|-------------|
| **0: Writer** | 0 (inline) | N/A | C cores | Admission control |
| **1: Wakeup** | 2C | 10μs | 0 (inline) or 1 | Fallback to inline |
| **2: Evidence** | 64 | 1ms | 1–2 (auto-scale at ρ > 0.75) | Drop oldest |
| **3: GC** | 1 | 100ms | 1 | Chain-depth escalation |
| **4: Checkpoint** | 1 | 1s | 1 (on-demand) | WAL size trigger |
| **5: Invalidation** | 16 | 100μs | 0 (inline) or 1 | Inline promotion |

**Total helper threads at steady state:** 2 (evidence + GC).
Maximum under load: 3 (evidence auto-scales to 2 workers at c8).

**Little's Law derivation (c4 target, λ = 40K commits/sec):**
- Publish window: L = λ·W = 40K × 1μs = 0.04 (memory) — not a bottleneck
- Publish window: L = λ·W = 40K × 50μs = 2.0 (file) — 2 writers competing
- Evidence lane: L = 40K × 5μs = 0.2 — single worker sufficient
- GC lane: L = 100Hz × 1ms = 0.1 — single worker sufficient

**GC chain-depth escalation ladder:**

| Chain Depth | GC Response |
|-------------|------------|
| ≤ 16 | Normal: timer-interval GC |
| 17–64 | Elevated: double GC frequency |
| 65–256 | Urgent: GC after every commit |
| > 256 | Critical: inline GC before commit (IC promotion) |

---

## 4. Admission Control and Backpressure (from E4.3)

### 4.1 Evidence Vector (sampled per decision)

Key signals: `publish_window_occupancy`, `active_writers`, `available_cores`,
`evidence_queue_depth`, `max_chain_depth`, `wal_frames`, `retry_rate_1s`,
`abort_rate_1s`, `user_p50_ns`, `user_p99_ns`, `user_p999_ns`.

Derived: `writer_saturation = active_writers / available_cores`,
`publish_contention = publish_window_occupancy / available_cores`,
`tail_stress = user_p99_ns / user_p50_ns`.

### 4.2 Escalation Rules (priority order, first match wins)

| ID | Tier | Condition | Action |
|----|------|-----------|--------|
| G1 | Emergency | `max_chain_depth > 256` | TriggerEmergencyGc |
| G2 | Emergency | `wal_frames > 10000 && !checkpoint_active` | TriggerCheckpoint (RESTART) |
| G3 | Backpressure | `writer_saturation > 0.95` | ApplyBackpressure (SQLITE_BUSY) |
| G4 | Backpressure | `publish_contention > 0.5` | Defer (park ≤ 500μs) |
| G5 | Backpressure | `tail_stress > 15.0` | Defer (park ≤ 1ms) |
| G6 | Backpressure | `retry_rate_1s > 0.3` | Defer (park ≤ 200μs) |
| G7 | Backpressure | `abort_rate_1s > 0.2` | Defer (park ≤ 500μs) |
| G8 | Helper-Budget | `evidence_drops_1s > 100` | ShrinkHelperBudget (lane 2) |
| G9 | Helper-Budget | `invalidation_inline_fallbacks_1s > 10` | ShrinkHelperBudget (lane 5) |
| G10 | Safe-Mode | `user_p99_ns > 100ms` | ForceSafeMode |
| G11 | Safe-Mode | `gc_inline_active && publish_window_p99_ns > 10ms` | ForceSafeMode |
| G12 | Normal | (default) | Admit |

### 4.3 User-Visible Behavior

| State | What User Sees |
|-------|---------------|
| Normal (G12) | COMMIT returns at normal latency |
| Slowed (G4–G7) | BEGIN takes ≤ 1ms longer (park). No error. |
| Rejected (G3) | BEGIN returns SQLITE_BUSY immediately |

Safe mode is invisible to users — it only changes internal scheduling.

---

## 5. p50 Protection

p50 commit latency is protected by keeping ALL helper-lane work OFF the
writer's critical path:

- **:memory: DB:** IC budget ≤ 1μs + IF budget ≤ 5μs = p50 ≈ 6μs
- **File-backed DB:** IC budget ≤ 50μs (WAL dominated) + IF ≤ 5μs = p50 ≈ 55μs

No OA or OB work runs on the writer thread during COMMIT. GC only promotes to
IC at chain_depth > 256 (emergency circuit breaker). Checkpoint never runs
inline.

**p99 protection:** Queue depth bounds prevent unbounded queueing. GC
escalation prevents version-chain blowout. Checkpoint time-cap (5s) prevents
background I/O starvation.

---

## 6. Safe-Mode Semantics

When ForceSafeMode activates (G10 or G11):

1. All OA work becomes IF (evidence, invalidation run inline before COMMIT returns)
2. All OB work is suspended (GC, checkpoint, snapshot deferred)
3. Helper threads park after draining queues
4. `max_active_writers` is halved

**Exit conditions (all must hold for `recovery_holdoff_s` = 5 seconds):**
- `user_p99_ns < safe_mode_threshold / 2`
- `gc_inline_active == false`
- `tail_stress < tail_stress_threshold / 2`

**Invariants:** Safe mode NEVER disables concurrent-writer mode. NEVER
serializes writers at the file level. Only changes WHERE work executes.

**Emergency override:** G1/G2 (emergency) fire even during safe mode.
Emergency > Safe-Mode > everything else.

---

## 7. Controller Composition (from E4 composition input + G8.3)

### 7.1 Action-Space Exclusivity

E4 writes to: admission decisions, OA/OB promotion flags, evidence detail
level, effective max_active_writers. It does NOT write to SSI thresholds
(DRO), WAL policy (D1), or core affinity (E6).

### 7.2 Timescale Separation

E4 (per-commit) > DRO (per-1s) > D1 (per-checkpoint) > E6 (per-reconfig).
E4 never waits for a slower controller's decision.

### 7.3 Forbidden Interactions (from G8.3 decision map)

- **E4↔DRO:** Keep. 1-second window absorption prevents oscillation.
- **E4↔D1 WAL:** Keep. G2 emergency overrides safe-mode checkpoint suspension.
- **E4↔E6:** Tune. E6 must filter writer_saturation during safe_mode_active.
- **GC inline↔Publish window:** Keep (conditional). GC runs before, not inside, the publish window.

---

## 8. Interference-Informed Primitive Decisions (from G8.3)

| Primitive | E4 Decision | Rationale |
|-----------|------------|-----------|
| CommitIndex flat array | Keep | Zero-contention read path, IC-classified |
| PageLockTable CAS | Budget-Limit | Cross-NUMA CAS bounded by E4.3 G3 backpressure |
| CommitSequenceCombiner | Keep (tune for cross-NUMA if measured) | Combiner batching already reduces cache-line traffic 16× |
| ConcurrentRegistry Mutex | Tune → RCU snapshot | Primary c4 contention source; RCU eliminates reader blocking |
| PagerPublishedSnapshot | Tune → SeqLockPair | Eliminates Mutex contention on BEGIN path |
| HTM fast-path | Safe-Mode-Only | Ship behind PRAGMA until CPU validation + abort telemetry |

---

## 9. Configuration Knobs

All operator-tunable via PRAGMA:

| Knob | Default | What It Controls |
|------|---------|-----------------|
| `guardrail_max_active_writers` | `available_cores` | G3 backpressure threshold |
| `guardrail_p99_target_ns` | 10ms | Target for tail stress calculation |
| `guardrail_tail_stress_threshold` | 15.0 | G5 Defer trigger |
| `guardrail_retry_rate_threshold` | 0.3 | G6 Defer trigger |
| `guardrail_abort_rate_threshold` | 0.2 | G7 Defer trigger |
| `guardrail_gc_chain_depth_tiers` | 16,64,256 | GC escalation ladder |
| `guardrail_wal_frame_tiers` | 1000,5000,10000 | Checkpoint urgency |
| `guardrail_safe_mode_p99_threshold_ns` | 100ms | G10 safe-mode trigger |
| `guardrail_recovery_holdoff_s` | 5 | Safe-mode exit hysteresis |

**Regime safety:** Under conservative defaults, the guardrail never activates
for healthy workloads with ≤ C writers, retry rate < 10%, and p99 < 10ms.

---

## 10. Verification Entrypoints

### Unit Tests

| Test | File | What It Verifies |
|------|------|-----------------|
| `test_guardrail_g1_emergency_gc_triggers_at_chain_depth_257` | `ssi_abort_policy.rs` | G1 fires at threshold |
| `test_guardrail_g3_backpressure_at_saturation` | (E4.3 impl) | G3 fires when writer_saturation > 0.95 |
| `test_guardrail_safe_mode_exit_hysteresis` | (E4.3 impl) | Safe mode doesn't exit until all conditions hold for holdoff |
| `test_guardrail_priority_ordering` | (E4.3 impl) | Higher-priority rules override lower |
| `test_guardrail_concurrent_mode_preserved_in_safe_mode` | (E4.3 impl) | concurrent_mode_default stays true |
| `test_adversarial_schedule_generator_determinism` | `ssi_abort_policy.rs` | Seeded schedule is reproducible |
| `test_dro_adversarial_adaptation_evidence` | `ssi_abort_policy.rs` | DRO radius expands under skew |
| `test_dro_vs_static_p99_regime_shift_detection` | `ssi_abort_policy.rs` | DRO adapts, static doesn't |

### Integration Tests

| Test | What It Verifies |
|------|-----------------|
| Burst admission (100 simultaneous BEGINs at c4) | Defer activates at saturation |
| GC escalation ladder | Chain-depth tiers escalate and recover |
| Evidence drop → ShrinkHelperBudget | G8 fires under evidence flood |
| Safe-mode round-trip | Activate, sustain, exit with hysteresis |

### E2E Entrypoints

| Script | What It Runs |
|--------|-------------|
| `scripts/verify_e4_3_tail_guardrails.sh` | 3-phase workload: ramp → sustain → cool-down. Verifies guardrail transitions G12→G4→G3→G12. |
| `scripts/verify_adversarial_dro.sh` | 6-regime adversarial schedule under DRO vs static. Verifies p99 non-regression, adaptation, abort suppression. |

### Structured Log Targets

| Target | What It Captures |
|--------|-----------------|
| `fsqlite::guardrail::decision` | Per-commit guardrail decisions with evidence vector + counterfactual |
| `fsqlite::dro::adversarial` | Per-txn T3 decisions under adversarial schedule |
| `fsqlite::lane::*` | Per-lane queue depth, drops, escalation events |
| `fsqlite::commit::publish_window` | Publish window duration and occupancy |

---

## 11. Child Artifact Index

| Child | Artifact | Scope |
|-------|----------|-------|
| E4.1 | `docs/design/inline-offload-classification-and-metadata-publication.md` | Work classification + metadata classes |
| E4.2 | `docs/design/queue-depth-wake-to-run-and-helper-lane-budgets.md` | Lane budgets + Little's Law + adaptation rules |
| E4.3 | `docs/design/admission-control-and-tail-latency-guardrails.md` | Evidence vector + 12 rules + safe mode + PRAGMA knobs |
| E4→G8.4 | `docs/design/e4-composition-input-for-g8-4.md` | Action-space boundaries + 5 forbidden interactions |
| G8.3 (consumed) | `docs/design/interference-to-primitive-selection-decision-map.md` | Keep/Tune/Budget/Safe/Reject decisions per primitive |
