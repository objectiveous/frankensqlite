# Queue-Depth, Wake-to-Run, and Helper-Lane Budgets

**Bead:** `bd-db300.5.4.2` (E4.2)
**Date:** 2026-03-22
**Status:** Design artifact — ready for admission control (E4.3) and implementation
**Depends on:** E4.1 inline/offload classification, E1 state-placement map, ADR-0002

---

## Purpose

Define explicit queue-depth and wake-to-run budgets for every offloaded work
class, derive them from Little's Law and the measured pipeline geometry, and
specify the helper-lane structure with starvation policies, safe-mode fallbacks,
and tail-latency guardrails.

The E4.1 artifact partitioned commit-path work into four execution classes
(IC, IF, OA, OB) and sketched preliminary lane budgets. This document refines
those into implementable budgets with explicit derivations, safe-mode triggers,
and monitoring contracts.

---

## Queueing Model

### System Parameters

| Symbol | Meaning | Value Range | Source |
|--------|---------|-------------|--------|
| C | Physical writer cores available | 1–64 | Hardware topology (bd-db300.1.6) |
| λ | Aggregate commit arrival rate (commits/sec) | 10K–200K | Benchmark measurement |
| W_ic | Mean publish-window duration (IC operations) | 0.2μs–50μs | Profile: in-memory vs file-backed |
| W_if | Mean inline-fast tail duration | 1μs–5μs | Profile |
| W_oa | Mean offload-async service time | 10μs–100μs | Profile |
| W_ob | Mean offload-background service time | 1ms–1s | Profile |
| σ² | Variance of service time (per class) | measured | For M/G/1 tail bounds |

### Little's Law Applied Per Lane

For each lane, the steady-state occupancy is:

```
L_lane = λ_lane · W_lane
```

where `λ_lane` is the arrival rate of work items to that lane and `W_lane` is
the mean service time.

The queue depth bound `Q_max` must satisfy:

```
Q_max > L_lane  (otherwise queue grows unboundedly)
```

When `Q_max` is reached, backpressure must be applied to the upstream stage.

### Kingman's Formula for Tail Latency

For the GI/G/1 queues (evidence and GC lanes), the expected wait time is
approximately:

```
E[wait] ≈ (ρ / (1 - ρ)) · ((c²_a + c²_s) / 2) · W

where:
  ρ = λ·W (utilization)
  c²_a = squared coefficient of variation of inter-arrival times
  c²_s = squared coefficient of variation of service times
```

The p99 wait time is approximately:

```
W_p99 ≈ -ln(0.01) / (μ(1 - ρ))  ≈ 4.6 / (μ(1 - ρ))

where μ = 1/W is the service rate.
```

This formula is used below to derive queue depth bounds that protect p99
latency.

---

## Lane Definitions

### Lane 0: Writer Lane (Per-Core)

**Scope:** Stages 1–5 IC and IF work. One writer per core.

| Property | Value | Derivation |
|----------|-------|-----------|
| Queue depth | 0 (no queue) | The writer IS the lane. There is no separate queue — the writer thread executes IC/IF work inline. |
| Wake-to-run | N/A | Writer is already running on its core. |
| Starvation policy | Page-lock wait ordering (FIFO per page) | Writers blocked at Stage 4 wait in per-page FIFO queues. |
| Max occupancy | C (one per physical core) | Bounded by hardware. |
| Backpressure to upstream | N/A | Writers are the source of all work. Admission control (E4.3) is the upstream throttle. |

**Budget formula:**
```
L_writer = min(C, λ · W_ic)

If L_writer > C:
  → admission control must throttle λ to keep L_writer ≤ C
  → this is the E4.3 trigger condition
```

**Safe mode:** If a writer lane is stuck (W_ic > 10ms for a single commit),
emit a `tracing::warn!` and do NOT block other lanes. The stuck writer handles
its own timeout via the existing busy-retry loop.

---

### Lane 1: Wakeup Dispatch Lane

**Scope:** IF-class waiter wakeup after page-lock release. Moved out of the
publish window per E4.1 promotion analysis.

| Property | Value | Derivation |
|----------|-------|-----------|
| Arrival rate (λ₁) | ≤ λ · avg_pages_per_commit | One wakeup per released page that has waiters |
| Service time (W₁) | ~200ns per wakeup (futex or condvar signal) | Measured: single unpark operation |
| L₁ = λ₁ · W₁ | At λ=40K, 2 pages/commit: L₁ ≈ 0.016 | Well below 1 — wakeup is not a bottleneck |
| Queue depth bound | 2 × C | At most C writers can release locks simultaneously, each touching at most a few pages |
| Wake-to-run budget | ≤ 10μs | Wakeup must reach the waiting writer within 10μs of lock release |
| Starvation policy | Drain-on-idle | When no commits are in flight, drain any remaining wakeup items |

**Implementation options (choose one):**

1. **Inline batched wakeup (preferred for ≤ 16 cores):**
   Writer batches all wakeups from its commit into a single post-publish sweep.
   No separate thread. This is essentially the IF path from E4.1.
   - Pro: Zero thread overhead, zero queue overhead.
   - Con: Adds ~200ns × pages_with_waiters to COMMIT return latency.
   - When: L₁ < 0.1 (almost always true).

2. **Dedicated wakeup thread (for > 16 cores or high-contention):**
   A single wakeup dispatcher thread reads from a bounded MPSC channel.
   - Pro: Removes wakeup latency from COMMIT return path.
   - Con: One extra thread, one cache-line handoff per wakeup batch.
   - When: L₁ > 0.1 or measured wakeup batch latency > 5μs.

**Decision rule:** Start with option 1. Monitor wakeup batch latency via
`tracing::debug!(target: "fsqlite::wakeup", batch_size, duration_ns)`.
If p99 batch duration > 5μs at c8+, switch to option 2.

**Safe-mode fallback:** If the wakeup channel is full (option 2), the writer
falls back to inline wakeup (option 1). No wakeups are lost.

---

### Lane 2: Evidence Lane

**Scope:** OA-class SSI evidence card recording, abort analytics, commit
observability events.

| Property | Value | Derivation |
|----------|-------|-----------|
| Arrival rate (λ₂) | ≤ λ | At most one evidence card per commit/abort |
| Service time (W₂) | ~5μs (struct copy + ring buffer append) | Measured: no I/O, purely in-memory |
| L₂ = λ₂ · W₂ | At λ=200K: L₂ ≈ 1.0 | Single worker can sustain 200K/sec |
| Queue depth bound | 64 entries | Sized for burst absorption at c8 commit rates |
| Wake-to-run budget | ≤ 1ms | Evidence is informational. 1ms delay is invisible. |
| Starvation policy | Drop oldest on overflow | Evidence loss is acceptable (M8 classification: None snapshot semantics) |

**Derivation of queue depth = 64:**
```
At c8 with λ = 200K commits/sec:
  λ₂ = 200K/sec
  W₂ = 5μs
  L₂ = 1.0

Burst factor: assume 8 writers commit simultaneously in a 50μs window:
  burst_arrivals = 8
  burst_service_during_burst = 50μs / 5μs = 10
  net burst growth = 8 - 10 = 0  (no growth — lane keeps up)

Conservative bound: 64 entries absorbs 64 simultaneous commits arriving
in a single batch before any are processed. This is 8× the core count
at c8, providing margin for scheduling jitter and GC pauses.
```

**p99 latency bound (Kingman):**
```
ρ₂ = L₂ = 1.0 at 200K — dangerously close to 1.0

At λ = 100K (c4 target):
  ρ₂ = 0.5
  W_p99 ≈ 4.6 / (200K · 0.5) = 46μs  ✓ within 100μs OA budget

At λ = 200K (c8 peak):
  ρ₂ = 1.0 — queue becomes unstable with a single worker

Mitigation: at ρ₂ > 0.8 (λ₂ > 160K/sec), spawn a second evidence
worker. With 2 workers, ρ₂ = 0.5 at 200K → stable.
```

**Worker scaling rule:**
```
evidence_workers = max(1, ceil(λ₂ · W₂ / 0.75))

This keeps per-worker utilization ≤ 75%, ensuring p99 < 4× service time.
```

**Safe-mode fallback:** If evidence queue overflows, drop the oldest entry
and increment `evidence_drops_total` counter. If drops exceed 100 in any
1-second window, emit `tracing::warn!` and reduce evidence detail level
(omit per-page conflict lists, keep only summary fields). This reduces W₂
by ~3× without losing the abort-rate signal.

---

### Lane 3: GC Lane

**Scope:** OB-class version-chain reclamation via MVCC `gc_tick`.

| Property | Value | Derivation |
|----------|-------|-----------|
| Arrival rate (λ₃) | 1 per GC interval (currently ~10ms default) | GC is triggered periodically or by chain-depth threshold |
| Service time (W₃) | 100μs–10ms (depends on version chain depth) | Measured: proportional to pruned versions |
| L₃ = λ₃ · W₃ | At 100Hz trigger, 1ms service: L₃ ≈ 0.1 | Low utilization — not a bottleneck |
| Queue depth bound | 1 (at most one outstanding sweep) | Concurrent GC sweeps are wasteful and risk double-free |
| Wake-to-run budget | ≤ 100ms | GC delay is invisible to writers as long as version chains stay bounded |
| Starvation policy | Trigger on chain-depth threshold | If max chain depth > 64, force immediate GC regardless of interval |

**Why queue depth = 1:**

GC sweeps are coarse-grained and internally iterate the version store. Running
two sweeps simultaneously doubles memory pressure without proportional benefit.
A new GC trigger while one is in progress should be coalesced (increment a
"pending" flag, not enqueue a second sweep).

**Chain-depth emergency budget:**

| Chain depth | Response | Rationale |
|-------------|----------|-----------|
| ≤ 16 | Normal: GC runs on timer interval | Healthy operating range |
| 17–64 | Elevated: double GC frequency (halve interval) | Prevent runaway growth |
| 65–256 | Urgent: GC runs after every commit | Chains are pathologically long. Sacrifice some commit throughput to prevent memory blowout. |
| > 256 | Critical: inline GC before commit returns | This is a bug or degenerate workload. GC MUST run now. Equivalent to promoting GC from OB to IC temporarily. |

**Safe-mode fallback:** If GC sweep takes > 50ms (10× the trigger interval),
emit `tracing::warn!(target: "fsqlite::gc", sweep_duration_ms, pruned_count)`
and cap the sweep at the oldest 1024 versions per page to prevent unbounded
GC pauses. Resume remaining work on the next tick.

**GC and snapshot horizon invariant (INV-E1.1-7):**
GC must NEVER prune a version that is still visible to any active snapshot.
The `VersionGuardRegistry` horizon is the hard floor. The queue depth bound
(1) ensures only one sweep checks the horizon at a time, preventing TOCTOU
races on the active-snapshot set.

---

### Lane 4: Checkpoint Lane

**Scope:** OB-class WAL checkpoint.

| Property | Value | Derivation |
|----------|-------|-----------|
| Arrival rate (λ₄) | ~0.1–1 Hz | Triggered by WAL size threshold or explicit PRAGMA |
| Service time (W₄) | 10ms–5s (proportional to WAL size) | Measured: depends on dirty page count and I/O speed |
| L₄ = λ₄ · W₄ | At 1Hz, 100ms: L₄ ≈ 0.1 | Low utilization |
| Queue depth bound | 1 (at most one outstanding checkpoint) | Concurrent checkpoints are dangerous (WAL truncation races) |
| Wake-to-run budget | ≤ 1s | Checkpoint timing is best-effort. 1s delay is acceptable. |
| Starvation policy | WAL size trigger | If WAL exceeds 1000 frames, force checkpoint regardless of other work |

**Why queue depth = 1:**

SQLite's checkpoint protocol (PASSIVE, FULL, RESTART, TRUNCATE) is inherently
single-threaded per database. Running two checkpoints simultaneously is
undefined behavior in the SQLite specification and violates WAL-index
invariants. FrankenSQLite preserves this constraint.

**Checkpoint and publish-window interaction:**

Checkpoint does NOT hold the publish-window lock. It operates on committed
WAL frames and page cache state that are already visible. However:

- Checkpoint MAY briefly compete for pager page-cache Mutex entries.
- Checkpoint MUST NOT start while a writer is in the publish window if the
  checkpoint mode is RESTART or TRUNCATE (these truncate the WAL).
- For PASSIVE checkpoints (the default for adaptive autocheckpoint), there
  is no publish-window interaction.

**Safe-mode fallback:** If checkpoint takes > 5s, emit
`tracing::warn!(target: "fsqlite::checkpoint", duration_s, pages_written)`
and abort the checkpoint. The WAL will continue to grow until the next
successful checkpoint. This prevents checkpoint from becoming a hidden
p99 tail-latency contributor.

---

### Lane 5: Invalidation Lane (New — Split from Evidence)

**Scope:** OA-class differential commit invalidation emission.

| Property | Value | Derivation |
|----------|-------|-----------|
| Arrival rate (λ₅) | ≤ λ | One invalidation batch per commit with writes |
| Service time (W₅) | ~1μs (channel send of page-set) | Measured: bounded by channel capacity |
| L₅ = λ₅ · W₅ | At λ=200K: L₅ ≈ 0.2 | Low utilization |
| Queue depth bound | 16 | Sized for burst: 8 simultaneous commits, 2× margin |
| Wake-to-run budget | ≤ 100μs | Readers will pick up invalidation on next snapshot bind |
| Starvation policy | Coalesce on overflow | If queue is full, merge new invalidation into the most recent entry |

**Rationale for separate lane:**

Invalidation and evidence have different starvation policies (coalesce vs drop)
and different consumers (pager snapshot bind vs abort-rate controller). Keeping
them separate prevents evidence drops from delaying invalidation, and vice
versa.

**Safe-mode fallback:** If the invalidation channel is full, the writer
publishes the invalidation inline (promotes from OA to IF). This adds ~1μs
to commit return but guarantees readers see invalidation promptly.

---

## Budget Summary Table

| Lane | Queue Depth | Wake-to-Run | Workers | ρ_max | Backpressure |
|------|-------------|-------------|---------|-------|-------------|
| 0: Writer | 0 | N/A | C | 1.0 | Admission control (E4.3) |
| 1: Wakeup | 2C | 10μs | 0 (inline) or 1 | < 0.1 | Fallback to inline |
| 2: Evidence | 64 | 1ms | 1–2 (auto-scale) | ≤ 0.75 | Drop oldest |
| 3: GC | 1 | 100ms | 1 | < 0.5 | Chain-depth escalation |
| 4: Checkpoint | 1 | 1s | 1 | < 0.1 | WAL size trigger |
| 5: Invalidation | 16 | 100μs | 0 (inline) or 1 | < 0.5 | Inline promotion |

**Total helper threads at steady state:** 1 (evidence) + 1 (GC) = 2 threads.
Wakeup and invalidation are inline by default. Checkpoint is triggered
on-demand. At c8+ with high commit rates, evidence may auto-scale to 2
threads, for a maximum of 3 helper threads.

**Thread placement contract:** Helper threads (evidence, GC, checkpoint) should
be placed on the same NUMA node as the writer lanes they serve. On
single-socket machines this is automatic. On multi-socket machines, the
placement is governed by E6 (bd-db300.5.6.1).

---

## Tail-Latency Protection

### p50 Protection

p50 commit latency is protected by keeping ALL helper-lane work OFF the
writer's critical path (Lane 0). The only work on Lane 0 is IC and IF, which
have combined budgets of ≤ 55μs (50μs WAL write + 5μs IF).

For `:memory:` databases, the IC budget is ≤ 1μs, and the IF budget is ≤ 5μs,
giving p50 commit latency of ~6μs.

### p99 Protection

p99 commit latency is protected by:

1. **Queue depth bounds** prevent unbounded queueing in any lane.
2. **Backpressure escalation** prevents queue overflow from turning into
   contention on the writer lane.
3. **GC chain-depth escalation** prevents version chain traversal from becoming
   a hidden p99 contributor on the read path.
4. **Checkpoint time-cap** (5s max) prevents checkpoint from becoming a
   background resource hog that affects writer scheduling.

### p99.9 Protection (Adversarial)

Under adversarial workloads (hot-page contention, skewed access), the primary
p99.9 risk is page-lock wait time at Stage 4. This is governed by:

- **Fair page-lock wait ordering** (FIFO per page)
- **Admission control** (E4.3) throttling λ when L_writer approaches C
- **Abort-rate controller** (bd-3t52f) adjusting SSI policy under skew

The helper-lane budgets do NOT interact with p99.9 because all helper work
is already off the critical path.

---

## Monitoring Contract

Each lane must emit telemetry sufficient for the evidence ledger (WS5) to
attribute latency to specific lanes and detect budget violations.

| Lane | Metric | Alert Threshold |
|------|--------|-----------------|
| 0: Writer | `publish_window_duration_ns` | p99 > 10 × W_ic_measured |
| 1: Wakeup | `wakeup_batch_duration_ns`, `wakeup_batch_size` | p99 > 10μs |
| 2: Evidence | `evidence_queue_depth`, `evidence_drops_total` | depth > 48 (75% of 64) |
| 3: GC | `gc_sweep_duration_ms`, `max_chain_depth` | depth > 64 |
| 4: Checkpoint | `checkpoint_duration_s`, `checkpoint_pages_written` | duration > 5s |
| 5: Invalidation | `invalidation_queue_depth`, `invalidation_inline_fallbacks` | inline_fallbacks > 10/sec |

**Structured tracing targets:**
```
fsqlite::lane::writer       — IC/IF timing
fsqlite::lane::wakeup       — batch size and duration
fsqlite::lane::evidence     — queue depth and drops
fsqlite::lane::gc           — sweep duration and chain depth
fsqlite::lane::checkpoint   — checkpoint duration and pages
fsqlite::lane::invalidation — queue depth and inline fallbacks
```

---

## Decision Rules for Runtime Adaptation

These rules govern how the system adapts lane budgets at runtime without
operator intervention.

### Rule R1: Evidence Worker Auto-Scale

```
IF evidence_queue_utilization > 0.75 for > 1 second:
  spawn additional evidence worker (up to max 4)
IF evidence_queue_utilization < 0.25 for > 10 seconds AND workers > 1:
  drain one evidence worker
```

**Rationale:** Evidence is OA-class (loss-tolerant). Scaling up prevents drops.
Scaling down reclaims threads when load subsides. The 1s/10s hysteresis
prevents oscillation.

### Rule R2: Wakeup Lane Promotion

```
IF wakeup_batch_p99 > 5μs for > 100 commits:
  switch from inline wakeup to dedicated wakeup thread
IF wakeup_queue_utilization < 0.01 for > 60 seconds:
  switch back to inline wakeup
```

**Rationale:** Most workloads have few page-lock waiters. The dedicated thread
is only needed under sustained contention.

### Rule R3: GC Frequency Escalation

```
IF max_chain_depth > 16: gc_interval = gc_interval / 2
IF max_chain_depth > 64: gc_after_every_commit = true
IF max_chain_depth > 256: gc_inline_before_commit = true (IC promotion)
IF max_chain_depth < 8 for > 30 seconds: restore default gc_interval
```

**Rationale:** Chain depth is a leading indicator of memory pressure. Gradual
escalation prevents sharp transitions. The inline fallback (>256) is a
circuit-breaker — it sacrifices throughput to prevent memory exhaustion.

### Rule R4: Checkpoint Urgency

```
IF wal_frames > 1000: trigger checkpoint (PASSIVE)
IF wal_frames > 5000: trigger checkpoint (FULL)
IF wal_frames > 10000: trigger checkpoint (RESTART) + tracing::warn!
```

**Rationale:** WAL growth is a trailing indicator of commit throughput
exceeding checkpoint throughput. Progressive escalation from PASSIVE (no
writer blocking) to RESTART (writers wait for checkpoint) matches the
urgency gradient.

---

## Assumptions Ledger

| ID | Assumption | Verification | Failure Response |
|----|-----------|-------------|-----------------|
| B1 | Evidence service time W₂ ≈ 5μs | Profile evidence recording at c4/c8 | If W₂ > 20μs, increase queue depth to 128 and reduce per-card detail |
| B2 | GC sweep completes within 10ms for typical chain depths ≤ 16 | Profile gc_tick at c4 with mixed workload | If W₃ > 10ms, cap sweep at 1024 versions and split across ticks |
| B3 | Wakeup dispatch adds < 1μs per waiter at c4 | Profile futex/condvar signal latency | If > 1μs, investigate kernel scheduling or switch to eventfd |
| B4 | 2 helper threads (evidence + GC) are sufficient for c8 | Monitor at c8 sustained load for 60s | If evidence drops > 0 or GC depth > 64, increase helpers |
| B5 | Checkpoint PASSIVE does not interfere with publish window | Profile overlapping checkpoint + commit at c4 | If interference detected, gate checkpoint start on publish-window quiescence |

---

## Consequences for Downstream Beads

| Downstream Bead | What This Artifact Provides |
|-----------------|---------------------------|
| **bd-db300.5.4.3** (E4.3: admission control) | Backpressure trigger formula (L_writer > C), admission queue depth bound (2C), escalation sequence, p50 protection guarantee |
| **bd-db300.5.3.2** (E3.2: primitive mapping) | Per-lane publication timing constraints that constrain which primitives are viable for each metadata class |
| **bd-77l3t** (HTM fast-path) | Lane 0 IC budget (500ns/page) as the target for combiner optimization |
| **bd-wnk1r / bd-bolsv** (GC) | Lane 3 budget (1 outstanding sweep, chain-depth escalation table, 50ms sweep cap) |
| **bd-3t52f** (DRO abort policy) | Lane 2 evidence delivery guarantee (≤ 1ms, drop-oldest under overload) as input to abort-rate controller sample freshness |

---

## Verification Plan

### Unit Tests

1. **Queue depth enforcement:** For each lane with a bounded queue, verify that
   the queue rejects/drops/coalesces when full. No silent growth.

2. **GC escalation ladder:** Property-test that chain depth thresholds trigger
   the correct GC frequency response. Verify that the inline fallback (>256)
   activates and deactivates correctly.

3. **Evidence auto-scale:** Simulate burst arrival at 200K/sec and verify that
   a second evidence worker is spawned within 1s. Simulate quiescence and
   verify drain within 10s.

### E2E Scenarios

1. **c4 steady-state:** Run c4 disjoint for 60s. Verify all lane metrics stay
   within budget. Evidence queue depth should stay < 16. GC chain depth < 16.

2. **c8 burst:** Run c8 mixed with 1-second burst of 50K commits. Verify
   evidence queue absorbs burst without drops. GC chain depth escalation
   triggers and recovers.

3. **Adversarial hot-page:** Run c4 hot-page with 4 writers touching the same
   page. Verify wakeup batch latency stays < 10μs. Verify admission control
   (E4.3) activates if needed.

### Logging Artifacts

All lane metrics must be emitted as structured tracing events with the targets
listed in the Monitoring Contract section. Each metric must include a monotonic
timestamp for time-series analysis.
