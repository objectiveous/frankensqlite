# Interference Results → Primitive-Selection Decision Map

**Bead:** `bd-db300.7.8.3` (G8.3)
**Date:** 2026-03-23
**Status:** Decision map built from current evidence; G8.2/G8.4 gaps noted
**Consumes:** G5.3 (evidence ownership), G6.3 (artifact manifests), E4 composition input,
E4.1/E4.2/E4.3 (scheduling/budgets/guardrails), E3.1 (metadata classes), E1 (state placement)

---

## Purpose

Convert measured or analyzed interference behavior into durable keep / tune /
budget-limit / safe-mode-only / reject / new-bead decisions for every
primitive and tuning candidate in the performance program. This is the
decision leverage layer — interference evidence that doesn't change a
plan decision is wasted work.

---

## 1. Decision Outcomes

| Outcome | Code | Meaning | Operator Action |
|---------|------|---------|-----------------|
| **Keep** | `K` | Primitive is correct and performant across tested placement profiles | Ship as default |
| **Tune** | `T` | Primitive works but needs parameter adjustment for specific regimes | Adjust threshold/size/interval, retest |
| **Budget-Limit** | `B` | Primitive works under budget but degrades without bound if uncapped | Add explicit budget cap and on-exhaustion behavior |
| **Safe-Mode-Only** | `S` | Primitive works only under safe-mode / conservative settings | Gate behind feature flag or PRAGMA; default off |
| **Reject** | `R` | Primitive causes unacceptable interference, correctness risk, or regression | Do not ship; document why; mark as dead end |
| **New-Bead** | `N` | Insufficient evidence to decide; create tracked work to gather it | Create child bead with specific measurement plan |

---

## 2. Decision Map — Synchronization Primitives

### 2.1 CommitIndex (M1)

| Primitive | Placement | Evidence | Outcome | Rationale | Affected Beads |
|-----------|-----------|----------|---------|-----------|----------------|
| **Flat AtomicU64 array** (pages ≤ 65536) | baseline_unpinned | E4.1 classification: IC, single Acquire load per read, Relaxed store after Release fence | **K** | Zero contention on read path. Write path is one fence + N stores, already optimal. No interference measured. | — |
| **Flat AtomicU64 array** | adversarial_cross_node | E1 state placement: NUMA-sensitive for hot pages | **T** | Cross-NUMA store→load latency is ~100ns vs ~5ns local. Tune: mirror hot-page subset to NUMA-local read cache if measured remote-HITM > threshold. | bd-db300.5.3.2 (E3.2) |
| **LeftRight sharded tier** (pages > 65536) | baseline_unpinned | E3.1 classification: rare path, Mutex overhead | **T** | Extend flat array to 262144 entries (2 MiB) to eliminate LeftRight for common workloads. LeftRight Mutex adds ~50ns per cold-page lookup under contention. | bd-db300.5.3.2 |
| **LeftRight sharded tier** | adversarial_cross_node | Not yet measured (G8.2 OPEN) | **N** | Create: "Measure LeftRight cross-NUMA contention at c8 with pages > 65536" | bd-db300.7.8.2 gap |

### 2.2 PageLockTable (M2)

| Primitive | Placement | Evidence | Outcome | Rationale | Affected Beads |
|-----------|-----------|----------|---------|-----------|----------------|
| **Flat AtomicU64 CAS array** (pages ≤ 65536) | baseline_unpinned | E4.1: IC, single CAS per acquire/release | **K** | Lock-free fast path. CAS is ~10ns uncontended. No primitive change viable — routing (E5) is the lever for contention. | — |
| **Flat AtomicU64 CAS array** | adversarial_cross_node | E1: NUMA-sensitive for contested pages | **B** | Cross-NUMA CAS on a contested cache line is ~200ns. Budget: page-lock wait is already bounded by E4.3 guardrail (G3 backpressure at saturation > 0.95). The budget IS the E4.3 admission control. | bd-db300.5.4.3 |
| **Shard Mutex fallback** (pages > 65536) | any | E3.1: rare, sharded | **K** | Shard count is sufficient. Mutex hold time is brief. Not a contention source in any measured workload. | — |

### 2.3 Commit Sequence Counter (M3)

| Primitive | Placement | Evidence | Outcome | Rationale | Affected Beads |
|-----------|-----------|----------|---------|-----------|----------------|
| **CommitSequenceCombiner** (flat combining) | baseline_unpinned | E4.1: IC, ~1ns atomic, batched 16× by combiner | **K** | Combiner reduces cache-line traffic from O(N) to O(N/batch). Already optimal for single-socket. | — |
| **CommitSequenceCombiner** | adversarial_cross_node | E4 composition: combiner_lock Mutex is single-threaded | **T** | Cross-NUMA combiner lock acquisition adds latency. Tune: per-NUMA combiner with global reconciliation if measured combiner_lock contention > 5% of publish window. | bd-77l3t (HTM fast-path) |
| **HTM fast-path** (proposed) | recommended_pinned | bd-77l3t design: TSX/TME transaction around batch | **S** | HTM is safe-mode-only until: (1) CPU stepping is validated, (2) abort-rate telemetry is wired, (3) dynamic disable threshold is calibrated. Ship behind `PRAGMA fsqlite_disable_htm`. | bd-77l3t |

### 2.4 ConcurrentRegistry (M4)

| Primitive | Placement | Evidence | Outcome | Rationale | Affected Beads |
|-----------|-----------|----------|---------|-----------|----------------|
| **Arc<Mutex<HashMap>>** (current) | baseline_unpinned | E4.1: IC during SSI validation, IF during session recycle. E4.3 analysis: primary c4 contention source (assumption A2). | **T** | Mutex contention scales linearly with writer count. At c4, SSI validation holds the lock for O(active_txns) scan. Tune: RCU snapshot for SSI reads (readers never block writers). | bd-db300.5.3.2 |
| **RCU snapshot** (proposed for SSI reads) | baseline_unpinned | E3.1: strict snapshot semantics, 1:2 R:W, immediate reclamation | **K** (conditional) | Keep if: (1) RCU copy cost ≤ 64 sessions × 128 bytes = 8 KiB, (2) epoch-based reclamation doesn't add GC pressure. Reject if copy cost exceeds 1μs at c8. | bd-db300.5.3.2 |
| **RCU snapshot** | adversarial_cross_node | Not yet measured (G8.2 OPEN) | **N** | Create: "Benchmark RCU ConcurrentRegistry at c8 cross-NUMA" | bd-db300.7.8.2 gap |

### 2.5 PagerPublishedSnapshot (M5)

| Primitive | Placement | Evidence | Outcome | Rationale | Affected Beads |
|-----------|-----------|----------|---------|-----------|----------------|
| **Mutex + atomics** (current) | baseline_unpinned | E3.1: N:1 R:W, relaxed snapshot, retryable | **T** | Mutex on BEGIN path adds unnecessary serialization. Migrate to SeqLockPair (already implemented in codebase). | bd-db300.5.3.2 |
| **SeqLockPair** (proposed) | baseline_unpinned | Codebase has SeqLock/SeqLockPair in `seqlock.rs` | **K** (conditional) | Keep if: sub-nanosecond reads confirmed, no writer starvation under c8 BEGIN bursts. SeqLock readers never block writers. | bd-db300.5.3.2 |

---

## 3. Decision Map — Scheduling and Offload Primitives

### 3.1 Helper Lanes (E4.2)

| Lane | Evidence | Outcome | Rationale | Affected Beads |
|------|----------|---------|-----------|----------------|
| **Lane 0: Writer** (inline IC/IF) | E4.1/E4.2: publish window ≤ 1μs (memory), ≤ 50μs (file) | **K** | Writer IS the lane. No offload possible for IC work. | — |
| **Lane 1: Wakeup** (inline default) | E4.2: L₁ ≈ 0.016 at c4 | **K** | Utilization < 0.1. Inline wakeup is correct. Dedicated thread only at c16+. | bd-db300.5.4.2 |
| **Lane 2: Evidence** (1 worker) | E4.2: L₂ ≈ 0.5 at c4, ≈ 1.0 at c8 | **B** | Budget-limit at queue depth 64 with auto-scale to 2 workers. Drop-oldest on overflow is acceptable (M8: None snapshot semantics). | bd-db300.5.4.2 |
| **Lane 3: GC** (1 worker) | E4.2: L₃ ≈ 0.1, chain-depth escalation | **B** | Budget-limit at queue depth 1. Chain-depth escalation ladder (Normal→Elevated→Urgent→Critical) prevents memory exhaustion. | bd-wnk1r, bd-bolsv |
| **Lane 4: Checkpoint** (on-demand) | E4.2: L₄ ≈ 0.1, WAL size trigger | **K** | Single outstanding checkpoint, WAL size tiers. No interference with publish window for PASSIVE mode. | — |
| **Lane 5: Invalidation** (inline default) | E4.2: L₅ ≈ 0.2 at c8 | **K** | Low utilization. Inline with coalesce-on-overflow fallback. | — |

### 3.2 Admission Control (E4.3)

| Mechanism | Evidence | Outcome | Rationale |
|-----------|----------|---------|-----------|
| **G3 Backpressure** (writer_saturation > 0.95) | E4.3 design, conservative default | **K** | Safe: rejects with SQLITE_BUSY, standard SQLite protocol. |
| **G4 Defer** (publish_contention > 0.5) | E4.3 design, park ≤ 500μs | **K** | Safe: brief park, invisible to most workloads. |
| **G10/G11 Safe Mode** (p99 > 100ms) | E4.3 design, OA→IF promotion | **K** | Last resort. Never disables concurrent mode. Hysteresis exit. |
| **G1 Emergency GC** (chain_depth > 256) | E4.3 design, IC promotion | **B** | Budget: inline GC runs before publish window, not inside it (E4 composition F4). Bounded by sweep cap. |

---

## 4. Decision Map — Controller Composition

| Controller Pair | Evidence | Outcome | Rationale | Affected Beads |
|-----------------|----------|---------|-----------|----------------|
| **E4 ↔ DRO** | E4 composition F1: 1-second window absorption | **K** | No positive feedback amplification. DRO reads committed evidence, not admission events. | — |
| **E4 ↔ D1 WAL** | E4 composition F2: G2 emergency overrides safe mode | **K** | WAL bounded even during safe mode. No starvation. | — |
| **E4 ↔ E6 Placement** | E4 composition F3: safe_mode_active filter | **T** | Tune: E6 must ignore writer_saturation during safe mode. Not yet implemented. | bd-db300.5.6.1 |
| **E4 GC inline ↔ Publish Window** | E4 composition F4: GC placement before, not inside | **K** (conditional) | Keep if instrumented verification confirms no overlap. | bd-db300.7.8.4 |
| **DRO ↔ E6 Placement** | Not yet analyzed (G8.4 OPEN) | **N** | Create: "Analyze DRO × placement interaction under regime shift" | G8.4 gap |

---

## 5. Decision Map — Topology-Sensitive Candidates

| Candidate | Placement Required | Evidence | Outcome | Locality Assumption | Mitigation if Assumption Fails |
|-----------|-------------------|----------|---------|--------------------|---------------------------------|
| **Per-NUMA CommitIndex mirror** | adversarial_cross_node | Not yet measured | **N** | Hot pages are accessed from remote NUMA nodes | Fallback: single global flat array (current) |
| **Per-NUMA combiner** | adversarial_cross_node | Not yet measured | **N** | Combiner lock contention is cross-NUMA dominant | Fallback: single combiner (current) |
| **LLC-aware page-lock routing** | recommended_pinned | E5 design (not yet implemented) | **N** | Page-lock CAS contention is LLC-domain sensitive | Fallback: global flat array (current) |
| **NUMA-local GC sweep** | adversarial_cross_node | Not yet designed | **N** | GC version-chain traversal causes remote memory access | Fallback: global GC (current) |

**All topology-sensitive candidates are outcome N (new-bead)** because G8.2
(cross-node interference cases) is OPEN. These rows become actionable when
G8.2 produces measured artifacts.

---

## 6. Gap-Conversion Rules

### I1: Missing Interference Evidence → New Bead

```
IF a primitive has outcome N (new-bead) due to missing measurement:
  → Create child bead under G8.2: "Measure {primitive} at {placement}"
  → The primitive remains at its current default until evidence arrives
  → No promotion to default without measured interference data
```

### I2: Rejected Primitive → Dead-End Record

```
IF a primitive has outcome R (reject):
  → Document the rejection reason in this map
  → Mark the affected design bead as "not viable"
  → Future agents must not re-propose the same primitive without
    new evidence that addresses the original rejection reason
```

### I3: Tune Outcome → Threshold Bead

```
IF a primitive has outcome T (tune):
  → The tuning parameter, threshold, or size adjustment must be
    specified in the "Rationale" column
  → Create a child bead if the tuning requires implementation work
  → The tuned version must pass the same proof pack as the original
```

### I4: Budget-Limit Outcome → E4 Integration

```
IF a primitive has outcome B (budget-limit):
  → The budget must map to an E4.2 lane or E4.3 guardrail rule
  → The on-exhaustion behavior must be specified
  → The budget is enforced at runtime, not just documented
```

---

## 7. Structured Log Schema

When the decision map is evaluated (at design review, implementation gate,
or scorecard compilation):

```rust
tracing::info!(
    target: "fsqlite::interference::decision",
    trace_id = %trace_id,
    scenario_id = %scenario_id,
    primitive_class = %class,          // "commit_index", "page_lock_table", etc.
    placement_profile = %profile,
    interference_artifact_key = artifact.as_deref().unwrap_or("none"),
    decision_target_bead = %target_bead,
    decision_outcome = %outcome,       // "K", "T", "B", "S", "R", "N"
    threshold_breached = breached.as_deref().unwrap_or("none"),
    next_action = %action,
    locality_assumption = assumption.as_deref().unwrap_or("none"),
    mitigation = mitigation.as_deref().unwrap_or("none"),
);
```

---

## 8. Validation Entrypoint

```bash
scripts/verify_g8_3_interference_mapping.sh
```

This script:
1. Reads the decision map (this document or a future JSON registry).
2. For each primitive in the map, verifies:
   - An outcome code is assigned (no blank rows).
   - If outcome is N, a target bead or measurement plan exists.
   - If outcome is T or B, the tuning parameter or budget is specified.
   - If outcome is R, the rejection reason is documented.
3. Reports unmapped primitives (any primitive in the codebase's M1–M8
   classification that has no row in this map).
4. Emits `artifacts/g8_3_interference_mapping.json`.
5. Exits 0 if all mapped, 1 if gaps exist.

---

## 9. Dependency Gaps

| Blocking Bead | Status | Impact | Mitigation |
|--------------|--------|--------|------------|
| **bd-db300.7.8.2** (G8.2: cross-node cases) | OPEN | All topology-sensitive candidates (§5) are outcome N. No measured cross-NUMA interference data exists yet. | Candidates default to current primitives (safe). Decision map updates when G8.2 produces artifacts. |
| **bd-db300.7.8.4** (G8.4: composition proof) | OPEN (blocked) | Controller composition decisions (§4) rely on E4 composition input (delivered) but not yet on full replay-based proof. | E4 composition rules are used as-is. F1–F5 mediation claims are treated as provisional until G8.4 replay verification completes. |

---

## 10. Summary: Current Decision Posture

| Outcome | Count | Examples |
|---------|-------|---------|
| **K (Keep)** | 11 | Flat AtomicU64 arrays, inline wakeup/invalidation, checkpoint, E4↔DRO composition |
| **T (Tune)** | 5 | LeftRight→extend flat array, ConcurrentRegistry→RCU, PagerSnapshot→SeqLockPair, cross-NUMA combiner, E4↔E6 safe_mode filter |
| **B (Budget-Limit)** | 3 | Evidence lane (queue 64), GC lane (chain-depth ladder), page-lock CAS (E4.3 admission) |
| **S (Safe-Mode-Only)** | 1 | HTM fast-path |
| **R (Reject)** | 0 | (none yet — no primitive has been measured and found unviable) |
| **N (New-Bead)** | 7 | All topology-sensitive candidates, RCU cross-NUMA, DRO×E6 interaction |

**Key takeaway:** The 5 Tune outcomes (LeftRight extension, RCU registry,
SeqLockPair snapshot, per-NUMA combiner, E6 safe-mode filter) are the
highest-leverage primitive changes. They feed directly into bd-db300.5.3.2
(E3.2 primitive mapping) and should be the next implementation targets
after the structural pillars (E2 fused entry, D1 parallel WAL).
