# E4 Composition Input for bd-db300.7.8.4

**Purpose:** Crisp handoff from the E4 admission/tail-guardrail owner to the
G8.4 controller-composition proof owner. This note states what the E4
controller promises, what it forbids, and what the composition proof must
verify — so pane 3 can consume it mechanically without re-deriving E4 internals.

**Source artifacts:**
- `docs/design/admission-control-and-tail-latency-guardrails.md` (E4.3)
- `docs/design/queue-depth-wake-to-run-and-helper-lane-budgets.md` (E4.2)
- `docs/design/inline-offload-classification-and-metadata-publication.md` (E4.1)

---

## 1. E4 Action-Space Boundaries

The E4 guardrail controller writes to exactly these outputs and no others:

| Output | Type | Who Reads |
|--------|------|-----------|
| `GuardrailAction` (Admit/Defer/Backpressure/ShrinkHelper/SafeMode/EmergencyGC/TriggerCheckpoint) | Per-commit decision | Transaction admission path |
| `max_active_writers` (effective) | u32, possibly halved in safe mode | BEGIN handler |
| OA→IF promotion flag | bool, per safe-mode state | Commit finalization path |
| OB suspension flag | bool, per safe-mode state | GC/checkpoint schedulers |
| Evidence detail level | enum (full/reduced), per G8 shrink | Evidence lane |

**E4 does NOT write to:**
- SSI abort thresholds (owned by DRO, bd-3t52f)
- WAL journal-mode or checkpoint-mode selection (owned by D1)
- Core-to-lane affinity or NUMA placement (owned by E6)
- CommitIndex or PageLockTable publication primitives (owned by E3)
- `concurrent_mode_default` (project invariant — never written by any controller)

**Composition rule for G8.4:** If a controller pair both write to the same
output, the composition is rejected. E4 outputs are exclusive.

---

## 2. Timescale Separation Contract

| Controller | Decision timescale | Evidence window | Max decision rate |
|------------|-------------------|-----------------|-------------------|
| **E4 guardrail** | Per-commit | 1-second trailing | ~λ decisions/sec (one per BEGIN or COMMIT) |
| **D1 WAL policy** | Per-checkpoint | 1–10 second trailing | ~0.1–1 decisions/sec |
| **E6 placement** | Per-reconfiguration | Minutes | ~0.01 decisions/sec |
| **DRO abort** (bd-3t52f) | Per-1s window | 1-second trailing | ~1 decision/sec |

**Separation invariant:** E4 is the fastest controller. It must never wait for
a slower controller's decision before producing its own output. Specifically:

- E4 may *trigger* a checkpoint (G2 action), but it does not wait for the
  checkpoint to complete before admitting the next transaction.
- E4 may *observe* that DRO has changed the abort threshold, but it does not
  request or depend on a specific DRO decision.
- E4 may *report* writer_saturation and tail_stress signals to E6, but E6's
  response arrives minutes later and E4 treats current placement as given.

**Composition rule for G8.4:** If controller A's decision latency exceeds
controller B's evidence window, A must be the slower controller. If both
operate at the same timescale on the same bottleneck, the composition must
prove no positive-feedback amplification.

---

## 3. Forbidden Interactions

These are specific interaction patterns the composition proof must reject.

### F1: E4 ↔ DRO Positive Feedback

**Scenario:** E4 observes high abort_rate → Defer → fewer BEGINs → DRO sees
lower abort_rate → loosens SSI threshold → burst of new txns → abort storm →
E4 Defer again → oscillation.

**Prevention:** E4 Defer does not change the DRO controller's evidence window.
DRO reads abort_rate from committed evidence cards (OA lane), not from
admission events. The 1-second DRO window absorbs E4 Defer transients because
Defer parks for ≤1ms, well below the 1-second window.

**Proof obligation for G8.4:** Replay a 10-second oscillation scenario with
both controllers active. Assert that the oscillation period (if any) is > 5
seconds (damped) rather than < 1 second (amplified). If amplified, the
composition must be rejected.

### F2: E4 Safe Mode ↔ D1 Checkpoint Starvation

**Scenario:** Safe mode suspends OB work including checkpoint. WAL grows
unboundedly. When safe mode exits, a massive checkpoint blocks writers.

**Prevention:** E4 safe mode suspends *scheduled* checkpoints but the G2
emergency rule (WAL > 10K frames) still fires even in safe mode. G2 is
priority 1, above G10/G11 (safe mode is priority 4). So WAL growth is
bounded even during safe mode.

**Proof obligation for G8.4:** Run safe mode for 30 seconds with sustained
writes. Assert WAL never exceeds 10K frames. Assert checkpoint fires via G2
and completes without deadlock.

### F3: E4 Safe Mode ↔ E6 Placement Feedback

**Scenario:** Safe mode halves max_active_writers. E6 observes low
writer_saturation → concludes system is under-utilized → widens placement
to more NUMA nodes → increases remote-ownership traffic → p99 spikes → safe
mode persists indefinitely.

**Prevention:** E6 must ignore writer_saturation changes that coincide with
safe mode. E4 exports a `control_mode` field in its log events. E6 must
filter evidence collected during `control_mode != "normal"`.

**Proof obligation for G8.4:** Define a `safe_mode_active` signal that E6
reads. Assert E6 never changes placement while this signal is true.

### F4: E4 GC Emergency ↔ Publish Window Inflation

**Scenario:** G1 triggers inline GC (IC promotion). GC takes 50ms inside the
publish window. publish_window_p99 spikes → G11 triggers safe mode → OB
suspended → GC can't run in OB → GC stays inline → permanent publish window
inflation.

**Prevention:** G1 (inline GC) runs *before* the publish window, not inside
it. The IC promotion means GC runs on the writer thread before SSI validation
starts, not between SSI validation and lock release. This keeps it outside the
publish-window timing instrumentation.

**Proof obligation for G8.4:** Instrument GC inline placement. Assert that
inline GC execution does not overlap with the publish_window_occupancy counter
increment. If it does, the E4.1 classification is wrong and must be corrected
before the composition is accepted.

### F5: Multiple E4 Defer Actions Compounding

**Scenario:** G4 (publish contention) and G6 (retry rate) both match. G4
wants Defer{500μs}, G6 wants Defer{200μs}. Which wins?

**Prevention:** E4 uses first-match priority. G4 (priority 2) fires before G6
(priority 2, but lower in the rule list). Only one Defer action is taken per
decision. The park budget from the first matching Defer rule is used.

**Proof obligation for G8.4:** Unit test all 12 rules in every pairwise
combination. Assert that exactly one action is produced per evidence vector.

---

## 4. Safe-Mode Dominance Rules

Safe mode (G10/G11) is the most aggressive E4 action. When active, it
constrains all other controllers:

| Other Controller | What Safe Mode Constrains | What It Does NOT Constrain |
|-----------------|--------------------------|---------------------------|
| **DRO (bd-3t52f)** | Nothing. DRO reads evidence independently. | SSI thresholds are DRO's authority. E4 does not touch them. |
| **D1 WAL policy** | Suspends scheduled checkpoints. G2 emergency overrides. | Journal mode selection. WAL frame size. fsync policy. |
| **E6 placement** | Exports `safe_mode_active` signal. E6 must not rebalance during safe mode. | Core affinity. NUMA policy. Helper thread placement. |
| **GC controller** | Suspends OB-class GC. G1 emergency overrides. | GC algorithm. Chain-depth thresholds. Reclamation strategy. |

**Dominance hierarchy (from E4.3 §3):**
```
G1/G2 (emergency) > G3–G7 (backpressure) > G8–G9 (helper-budget) > G10–G11 (safe-mode) > G12 (admit)
```

**Cross-controller dominance:**
```
E4 emergency (G1/G2) > E4 safe mode (G10/G11) > DRO/D1/E6 normal operation
```

E4 emergency actions override safe mode. No other controller can override E4
emergency actions. This prevents deadlock where safe mode prevents the work
that would fix the condition that triggered safe mode.

---

## 5. User-Visible Fallback Semantics

From the user's perspective, the E4 guardrail produces exactly three
observable behaviors:

| User Observation | E4 State | What User Sees |
|-----------------|----------|---------------|
| **Normal** | G12 Admit | COMMIT returns at normal latency. No SQLITE_BUSY from guardrail. |
| **Slowed** | G4–G7 Defer | BEGIN takes up to 1ms longer than usual (park budget). COMMIT latency unchanged. No error. |
| **Rejected** | G3 Backpressure | BEGIN returns SQLITE_BUSY immediately. User retries per standard SQLite busy-handler protocol. |

Safe mode (G10/G11) is NOT directly user-visible. It changes internal
scheduling but does not add latency to COMMIT or return errors. Users may
observe slightly higher p50 commit latency (because OA work runs inline) but
this is bounded by the IF budget (≤5μs).

**Composition rule for G8.4:** No other controller may make the user-visible
behavior worse than what E4 has already decided. If DRO aborts a transaction,
that is a separate user-visible event (SQLITE_BUSY_SNAPSHOT) with its own
error code, independent of E4's admission decision.

---

## 6. Shared Signals Inventory

These are the signals that E4 reads or writes and that other controllers
also touch:

| Signal | E4 Role | Other Controller Role | Conflict Risk |
|--------|---------|----------------------|---------------|
| `retry_rate_1s` | Reads (G6 threshold) | DRO reads (abort policy input) | **None**: both read, neither writes |
| `abort_rate_1s` | Reads (G7 threshold) | DRO reads and indirectly writes (by changing SSI thresholds) | **Low**: E4 reads trailing window; DRO's SSI changes affect future abort_rate, not current window |
| `wal_frames` | Reads (G2 threshold) | D1 reads (checkpoint trigger) | **Low**: both trigger checkpoint but only one checkpoint can run at a time (Lane 4 queue depth = 1) |
| `writer_saturation` | Computes (G3) | E6 reads (placement input) | **Medium**: E4 safe mode halves max_active_writers, distorting E6's signal. Mitigated by `safe_mode_active` filter. |
| `max_chain_depth` | Reads (G1 threshold) | GC controller reads (escalation input) | **None**: E4 only triggers emergency GC; GC controller owns the escalation ladder independently |
| `publish_window_p99_ns` | Reads (G11) | Not shared | **None** |
| `user_p99_ns` | Reads (G5, G10) | Not shared (E4 owns this measurement) | **None** |

---

## 7. Log Fields E4 Commits to Emit

For the G8.4 composition proof, E4 guarantees these fields in every guardrail
decision log event (target `fsqlite::guardrail::decision`):

```
trace_id, scenario_id, policy_id="e4.3.v1", decision_id, guardrail_id,
control_mode, action, park_budget_us,
publish_window_occupancy, active_writers, available_cores,
writer_saturation, publish_contention, tail_stress,
retry_rate_1s, abort_rate_1s, user_p50_ns, user_p99_ns, user_p999_ns,
max_chain_depth, gc_escalation_tier, gc_inline_active,
wal_frames, checkpoint_active,
evidence_queue_depth, evidence_drops_1s,
counterfactual_action, regret_delta_ns
```

The G8.4 proof can join on `trace_id` + `scenario_id` to correlate E4
decisions with DRO/D1/E6 decisions from their respective log targets.

---

## 8. What G8.4 Must Verify From the E4 Side

| Verification | Method | Pass Condition |
|-------------|--------|----------------|
| E4 action-space exclusivity | Static: enumerate all outputs of E4, D1, E6, DRO. Assert no overlap. | No two controllers write to the same output variable. |
| Timescale separation | Replay: 60-second workload with all controllers active. Measure decision rate per controller. | E4 > DRO > D1 > E6 in decisions/sec. No inversions. |
| F1 oscillation bound | Replay: oscillation scenario. Measure amplitude and period. | Period > 5s or amplitude < 10% of steady-state throughput. |
| F2 WAL bound under safe mode | Replay: 30s safe mode with writes. | WAL never exceeds 10K frames. |
| F3 placement stability | Replay: safe mode activation. | E6 makes zero placement changes while safe_mode_active. |
| F4 GC placement | Instrumented run: inline GC timing vs publish window timing. | Zero overlap between GC execution and publish_window_occupancy counter. |
| F5 single-action guarantee | Unit test: all 66 pairwise rule combinations. | Exactly one GuardrailAction per evidence vector. |
| Safe-mode dominance | Integration test: G1 fires during safe mode. | G1 (emergency) executes. Safe mode does not block it. |
| User-visible monotonicity | Integration test: E4 Defer followed by DRO abort. | User sees Defer delay then BUSY_SNAPSHOT. Not double-penalized (Defer + rejection). |

---

## 9. Summary for Pane 3

**E4 is the fastest controller.** It decides per-commit. It reads shared
signals but writes only to its own action space. Safe mode is the most
aggressive E4 state but is overridden by E4's own emergency rules (G1/G2).

**The three things most likely to cause interference are:**
1. **F1 (E4 Defer ↔ DRO abort rate):** Mediated by 1-second window absorption.
2. **F3 (safe mode ↔ E6 placement):** Mediated by `safe_mode_active` filter.
3. **F4 (inline GC ↔ publish window):** Mediated by GC placement before, not inside, the publish window.

**If any of these mediations are violated by implementation drift, the
composition proof must flag it as a blocker.** The structured log fields
listed in §7 provide the join surface for replay-based verification.
