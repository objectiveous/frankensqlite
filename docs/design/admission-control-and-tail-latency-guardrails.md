# Admission Control and Tail-Latency Guardrails

**Bead:** `bd-db300.5.4.3` (E4.3)
**Date:** 2026-03-22
**Status:** Design contract — ready for implementation
**Depends on:** E4.2 queue-depth/helper-lane budgets, E4.1 inline/offload classification, ADR-0002

---

## Purpose

Define the admission-control and tail-latency guardrail policy as an explicit
state machine with observable decisions, so throughput gains never come from
uncontrolled queue growth, retry storms, or degraded user-visible tails.

This is the operational contract downstream of the E4.2 lane budgets. Every
threshold, action, and fallback rule references a lane from E4.2.

---

## 1. Control State Space

The guardrail controller makes decisions based on a fixed evidence vector
sampled once per commit attempt and once per admission event.

### 1.1 Evidence Vector

```rust
/// Sampled once per guardrail decision point.
struct GuardrailEvidence {
    // ── Writer lane (Lane 0) ──
    /// Number of writers currently in the publish window (Stage 5 IC).
    publish_window_occupancy: u32,
    /// p99 publish-window duration over the trailing 1-second window (ns).
    publish_window_p99_ns: u64,
    /// Active concurrent writers (across all stages).
    active_writers: u32,
    /// Available physical cores for writer lanes.
    available_cores: u32,

    // ── Wakeup lane (Lane 1) ──
    /// Current wakeup queue depth.
    wakeup_queue_depth: u32,
    /// Whether wakeup lane has been promoted from inline to dedicated thread.
    wakeup_lane_promoted: bool,

    // ── Evidence lane (Lane 2) ──
    /// Current evidence queue depth.
    evidence_queue_depth: u32,
    /// Evidence drops in the trailing 1-second window.
    evidence_drops_1s: u32,
    /// Active evidence workers.
    evidence_workers: u32,

    // ── GC lane (Lane 3) ──
    /// Maximum version chain depth across all tracked pages.
    max_chain_depth: u32,
    /// Whether GC is currently in escalated mode.
    gc_escalation_tier: GcTier,
    /// Whether an inline GC promotion is active (IC, not OB).
    gc_inline_active: bool,

    // ── Checkpoint lane (Lane 4) ──
    /// Current WAL frame count.
    wal_frames: u32,
    /// Whether a checkpoint is currently running.
    checkpoint_active: bool,

    // ── Invalidation lane (Lane 5) ──
    /// Current invalidation queue depth.
    invalidation_queue_depth: u32,
    /// Inline fallback invocations in the trailing 1-second window.
    invalidation_inline_fallbacks_1s: u32,

    // ── Cross-cutting ──
    /// Commit-level retry rate in trailing 1-second window (0.0–1.0).
    retry_rate_1s: f32,
    /// Abort rate in trailing 1-second window (0.0–1.0).
    abort_rate_1s: f32,
    /// User-visible p50 commit latency in trailing 1-second window (ns).
    user_p50_ns: u64,
    /// User-visible p99 commit latency in trailing 1-second window (ns).
    user_p99_ns: u64,
    /// User-visible p99.9 commit latency in trailing 1-second window (ns).
    user_p999_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GcTier {
    /// Normal: GC on timer interval.
    Normal,
    /// Elevated: doubled GC frequency.
    Elevated,
    /// Urgent: GC after every commit.
    Urgent,
    /// Critical: inline GC before commit (IC promotion).
    Critical,
}
```

### 1.2 Derived Signals

```rust
/// Writer saturation ratio: how close to full the writer lane is.
fn writer_saturation(ev: &GuardrailEvidence) -> f32 {
    ev.active_writers as f32 / ev.available_cores.max(1) as f32
}

/// Publish window contention: writers competing for the serialized region.
fn publish_contention(ev: &GuardrailEvidence) -> f32 {
    ev.publish_window_occupancy as f32 / ev.available_cores.max(1) as f32
}

/// Tail stress: ratio of p99 to p50 (healthy < 5.0, stressed > 10.0).
fn tail_stress(ev: &GuardrailEvidence) -> f32 {
    if ev.user_p50_ns == 0 { return 1.0; }
    ev.user_p99_ns as f32 / ev.user_p50_ns as f32
}
```

---

## 2. Action Set

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum GuardrailAction {
    /// Allow the commit/transaction to proceed normally.
    Admit,
    /// Delay the BEGIN by parking in admission queue for up to `park_budget_us`.
    Defer { park_budget_us: u32 },
    /// Apply backpressure: reject new BEGINs with SQLITE_BUSY until load drops.
    ApplyBackpressure,
    /// Shrink helper-lane budget: reduce evidence workers or disable optional OA work.
    ShrinkHelperBudget { target_lane: u8 },
    /// Force safe mode: disable all OA/OB offloading, run everything IF or IC.
    ForceSafeMode,
    /// Trigger emergency GC: promote GC from OB to IC for the current commit.
    TriggerEmergencyGc,
    /// Trigger checkpoint: force WAL checkpoint to reduce per-commit W.
    TriggerCheckpoint,
}
```

---

## 3. Escalation Thresholds

The controller evaluates thresholds in priority order. First matching rule wins.

### 3.1 Emergency Tier (correctness protection)

| ID | Condition | Action | Rationale |
|----|-----------|--------|-----------|
| G1 | `max_chain_depth > 256` | `TriggerEmergencyGc` | Version chains are pathologically long. GC must run inline to prevent memory exhaustion. Maps to E4.2 Lane 3 critical tier. |
| G2 | `wal_frames > 10000 && !checkpoint_active` | `TriggerCheckpoint` | WAL is dangerously large. Checkpoint reduces per-commit W and prevents disk exhaustion. Maps to E4.2 Lane 4 RESTART tier. |

### 3.2 Backpressure Tier (tail protection)

| ID | Condition | Action | Rationale |
|----|-----------|--------|-----------|
| G3 | `writer_saturation > 0.95` | `ApplyBackpressure` | Writer lanes are saturated. Additional BEGINs will only increase contention. |
| G4 | `publish_contention > 0.5` | `Defer { park_budget_us: 500 }` | Publish window is contested. Brief parking reduces collision probability. |
| G5 | `tail_stress > 15.0` | `Defer { park_budget_us: 1000 }` | p99/p50 ratio indicates pathological tailing. Slow admissions to let queues drain. |
| G6 | `retry_rate_1s > 0.3` | `Defer { park_budget_us: 200 }` | High retry rate wastes compute. Slow admissions to reduce collision rate. |
| G7 | `abort_rate_1s > 0.2` | `Defer { park_budget_us: 500 }` | High abort rate signals hot-page contention or SSI pressure. Let competing txns finish. |

### 3.3 Helper-Budget Tier (resource protection)

| ID | Condition | Action | Rationale |
|----|-----------|--------|-----------|
| G8 | `evidence_drops_1s > 100` | `ShrinkHelperBudget { target_lane: 2 }` | Evidence lane is overwhelmed. Reduce evidence detail level per E4.2 Lane 2 safe-mode. |
| G9 | `invalidation_inline_fallbacks_1s > 10` | `ShrinkHelperBudget { target_lane: 5 }` | Invalidation queue saturated. Coalesce invalidations more aggressively. |

### 3.4 Safe-Mode Tier (forced fallback)

| ID | Condition | Action | Rationale |
|----|-----------|--------|-----------|
| G10 | `user_p99_ns > 100_000_000` (100ms) | `ForceSafeMode` | User-visible p99 exceeds 100ms. Disable all offloading and run everything inline to eliminate scheduling uncertainty. |
| G11 | `gc_inline_active && publish_window_p99_ns > 10_000_000` (10ms) | `ForceSafeMode` | Inline GC is dominating the publish window. Switch to pure inline execution until GC completes. |

### 3.5 Normal Tier

| ID | Condition | Action | Rationale |
|----|-----------|--------|-----------|
| G12 | Default (no condition matched) | `Admit` | System is healthy. Allow normal operation. |

---

## 4. Budget Knobs

These are operator-tunable parameters with safe defaults. All exposed via
PRAGMA or environment variable.

```rust
struct GuardrailConfig {
    // ── Admission ──
    /// Max active writers before backpressure. Default: available_cores.
    max_active_writers: u32,
    /// Max admission queue depth before SQLITE_BUSY. Default: 2 × available_cores.
    max_admission_queue: u32,

    // ── Tail targets ──
    /// User-visible p99 commit latency target (ns). Default: 10_000_000 (10ms).
    p99_target_ns: u64,
    /// Tail stress ratio (p99/p50) above which Defer activates. Default: 15.0.
    tail_stress_threshold: f32,

    // ── Retry/abort ──
    /// Retry rate above which Defer activates. Default: 0.3.
    retry_rate_threshold: f32,
    /// Abort rate above which Defer activates. Default: 0.2.
    abort_rate_threshold: f32,

    // ── GC ──
    /// Chain depth tiers: [normal_max, elevated_max, urgent_max].
    /// Default: [16, 64, 256].
    gc_chain_depth_tiers: [u32; 3],

    // ── WAL ──
    /// WAL frame thresholds: [passive, full, restart].
    /// Default: [1000, 5000, 10000].
    wal_frame_tiers: [u32; 3],

    // ── Safe mode ──
    /// p99 above which safe mode activates (ns). Default: 100_000_000 (100ms).
    safe_mode_p99_threshold_ns: u64,
    /// Publish window p99 above which safe mode activates (ns). Default: 10_000_000 (10ms).
    safe_mode_publish_window_threshold_ns: u64,

    // ── Hysteresis ──
    /// Seconds of healthy evidence required to exit escalated modes. Default: 5.
    recovery_holdoff_s: u32,
}
```

### PRAGMA Exposure

```sql
-- Read current config:
PRAGMA fsqlite.guardrail_config;

-- Set individual knobs:
PRAGMA fsqlite.guardrail_max_active_writers = 8;
PRAGMA fsqlite.guardrail_p99_target_ns = 5000000;
PRAGMA fsqlite.guardrail_tail_stress_threshold = 10.0;
PRAGMA fsqlite.guardrail_retry_rate_threshold = 0.25;
PRAGMA fsqlite.guardrail_gc_chain_depth_tiers = '16,64,256';
PRAGMA fsqlite.guardrail_wal_frame_tiers = '1000,5000,10000';
PRAGMA fsqlite.guardrail_safe_mode_p99_threshold_ns = 50000000;
```

---

## 5. Forced-Fallback / Safe-Mode Semantics

### 5.1 What Safe Mode Does

When `ForceSafeMode` activates:

1. **All OA work becomes IF.** Evidence recording, VTab notification, and
   invalidation run inline before COMMIT returns.
2. **All OB work is deferred.** GC, checkpoint, and snapshot capture are
   suspended until safe mode exits.
3. **Helper threads are parked.** Evidence and wakeup workers drain their
   queues and then park.
4. **Admission is throttled.** `max_active_writers` is halved.

### 5.2 Safe Mode Exit

Safe mode exits when ALL of:
- `user_p99_ns < safe_mode_p99_threshold_ns / 2` for `recovery_holdoff_s`
- `gc_inline_active == false`
- `tail_stress < tail_stress_threshold / 2`

The `/2` hysteresis prevents oscillation. The holdoff period ensures the
recovery is sustained, not a transient dip.

### 5.3 Safe Mode Invariants

- Safe mode NEVER disables concurrent-writer mode (INV-E1.1-1).
- Safe mode NEVER serializes writers at the file level.
- Safe mode only changes WHERE work executes (inline vs offloaded),
  not WHETHER work executes.

---

## 6. Timescale Separation Statement

This controller operates at the **per-commit** and **1-second trailing window**
timescales. It composes with other controllers as follows:

| Controller | Timescale | Interface | Separation |
|------------|-----------|-----------|------------|
| **E4.3 Guardrail** (this) | Per-commit + 1s window | Admits/defers/backpressures individual transactions | Fastest: reacts to instantaneous load |
| **D1 WAL policy** | Per-checkpoint (~1–10s) | Checkpoint triggers reduce per-commit W, which changes L | Slower: guardrail triggers checkpoint; checkpoint reduces guardrail pressure |
| **E6 Placement/locality** | Per-reconfiguration (~minutes) | Lane-to-core affinity changes affect W and contention | Slowest: guardrail reports pressure; placement policy adapts topology |
| **DRO abort controller** (bd-3t52f) | Per-1s window | Adjusts SSI abort thresholds based on expected loss | Parallel: reads same evidence vector (retry_rate, abort_rate), writes SSI policy knobs |

**Composition rule:** Each controller only writes to its own action space.
Conflicts are resolved by priority: G1/G2 (emergency) > G3–G7 (backpressure)
> G8–G9 (helper-budget) > G10–G11 (safe-mode) > G12 (admit). The DRO
controller adjusts SSI thresholds independently and does not override
guardrail admission decisions.

---

## 7. Structured Log Schema

Every guardrail decision emits a structured log event.

```rust
tracing::info!(
    target: "fsqlite::guardrail::decision",

    // ── Identity ──
    trace_id = %trace_id,
    scenario_id = %scenario_id,
    policy_id = "e4.3.v1",
    decision_id = %decision_counter,
    guardrail_id = %matched_rule_id,  // "G1"–"G12"

    // ── Evidence snapshot ──
    publish_window_occupancy = ev.publish_window_occupancy,
    publish_window_p99_ns = ev.publish_window_p99_ns,
    active_writers = ev.active_writers,
    available_cores = ev.available_cores,
    evidence_queue_depth = ev.evidence_queue_depth,
    evidence_drops_1s = ev.evidence_drops_1s,
    max_chain_depth = ev.max_chain_depth,
    gc_escalation_tier = %ev.gc_escalation_tier,
    wal_frames = ev.wal_frames,
    retry_rate_1s = ev.retry_rate_1s,
    abort_rate_1s = ev.abort_rate_1s,
    user_p50_ns = ev.user_p50_ns,
    user_p99_ns = ev.user_p99_ns,
    user_p999_ns = ev.user_p999_ns,

    // ── Derived signals ──
    writer_saturation = %writer_saturation(&ev),
    publish_contention = %publish_contention(&ev),
    tail_stress = %tail_stress(&ev),

    // ── Decision ──
    control_mode = %current_mode,  // "normal", "defer", "backpressure", "safe_mode"
    action = %chosen_action,       // serialized GuardrailAction
    park_budget_us = park_budget.unwrap_or(0),

    // ── Counterfactual (for offline analysis) ──
    counterfactual_action = %what_admit_would_have_done,
    regret_delta_ns = regret_estimate,
);
```

### Log Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `trace_id` | String | Unique ID for the transaction triggering this decision |
| `scenario_id` | String | Benchmark scenario or workload identifier |
| `policy_id` | String | Version of the guardrail policy (`"e4.3.v1"`) |
| `decision_id` | u64 | Monotonic counter of guardrail decisions |
| `guardrail_id` | String | ID of the matched rule (`"G1"`–`"G12"`) |
| `control_mode` | String | Current overall mode of the guardrail controller |
| `counterfactual_action` | String | What would have happened under `Admit` (for regret analysis) |
| `regret_delta_ns` | i64 | Estimated ns difference between chosen and counterfactual action |

---

## 8. Verification Entrypoints

### 8.1 Unit Tests

Located in `crates/fsqlite-e2e/tests/` (or inline `#[cfg(test)]` in the
implementation module):

| Test Name | What It Verifies |
|-----------|-----------------|
| `test_guardrail_g1_emergency_gc_triggers_at_chain_depth_257` | G1 fires when max_chain_depth > 256 |
| `test_guardrail_g3_backpressure_at_saturation` | G3 fires when writer_saturation > 0.95 |
| `test_guardrail_g4_defer_at_publish_contention` | G4 fires when publish_contention > 0.5 |
| `test_guardrail_g10_safe_mode_at_high_p99` | G10 fires when user_p99_ns > 100ms |
| `test_guardrail_safe_mode_exit_hysteresis` | Safe mode does not exit until all exit conditions hold for recovery_holdoff_s |
| `test_guardrail_priority_ordering` | Higher-priority rules override lower-priority ones |
| `test_guardrail_config_pragma_round_trip` | PRAGMA set/get for all knobs |
| `test_guardrail_admit_is_default` | G12 (Admit) when all conditions are healthy |
| `test_guardrail_concurrent_mode_preserved_in_safe_mode` | Safe mode never sets concurrent_mode_default to false |

### 8.2 Integration Tests

| Test Name | What It Verifies |
|-----------|-----------------|
| `test_guardrail_burst_admission_defers_correctly` | 100 simultaneous BEGINs at c4, verify Defer activates when saturation > 0.95 |
| `test_guardrail_gc_escalation_ladder` | Insert chain-depth-increasing workload, verify GC tiers escalate Normal→Elevated→Urgent→Critical |
| `test_guardrail_evidence_drop_triggers_shrink` | Flood evidence lane at 200K/sec, verify G8 fires and drops are counted |
| `test_guardrail_safe_mode_round_trip` | Push p99 above threshold, verify safe mode activates, reduce load, verify safe mode exits after holdoff |

### 8.3 Named E2E Entrypoint

```bash
scripts/verify_e4_3_tail_guardrails.sh
```

This script:
1. Builds with `--profile release-perf`.
2. Runs a 3-phase workload: ramp-up → sustained pressure → cool-down.
3. For each phase, captures guardrail decision logs via JSONL.
4. Verifies:
   - Phase 1 (ramp): decisions transition from G12 → G4/G6 → G3 as load increases.
   - Phase 2 (sustained): backpressure or safe mode is active; p99 stays below 2× target.
   - Phase 3 (cool-down): decisions transition back to G12; safe mode exits after holdoff.
5. Emits first-failure diagnostics as a counterexample bundle if any assertion fails.
6. Outputs structured JSONL summary to `artifacts/e4_3_verification/`.

---

## 9. Regime-Safe Defaults

Every threshold has a conservative default that prioritizes correctness and
user-visible latency over throughput.

| Knob | Default | Why Conservative |
|------|---------|-----------------|
| `max_active_writers` | `available_cores` | Never over-subscribes. Under-subscription is safe (just slower). |
| `max_admission_queue` | `2 × available_cores` | Bounds memory. Rejecting early is safer than queuing indefinitely. |
| `p99_target_ns` | `10_000_000` (10ms) | 10ms p99 is generous for in-memory DBs, reasonable for file-backed. Tighter targets can be set via PRAGMA. |
| `tail_stress_threshold` | `15.0` | 15× p99/p50 is extreme. Conservative: avoids false positives from normal variance. |
| `retry_rate_threshold` | `0.3` | 30% retry rate is already severe. Conservative: doesn't trigger on occasional retries. |
| `gc_chain_depth_tiers` | `[16, 64, 256]` | Wide tiers prevent over-aggressive GC. Emergency at 256 is a hard safety net. |
| `wal_frame_tiers` | `[1000, 5000, 10000]` | SQLite default auto-checkpoint is at 1000 frames. Our defaults match. |
| `safe_mode_p99_threshold_ns` | `100_000_000` (100ms) | 100ms p99 is catastrophic. Safe mode is a last resort, not a normal operating mode. |
| `recovery_holdoff_s` | `5` | 5 seconds of sustained health before exiting escalated modes. Prevents flapping. |

**Regime-safety guarantee:** Under the conservative defaults, the guardrail
controller NEVER activates for healthy workloads with ≤ C concurrent writers,
retry rate < 10%, and p99 < 10ms. It only engages when the system is genuinely
under stress.

---

## 10. Assumptions Ledger

| ID | Assumption | Verification | If Wrong |
|----|-----------|-------------|----------|
| C1 | 1-second trailing window is sufficient for stable signal | Run c4 mixed for 60s, measure signal variance | If noisy: increase to 5-second window |
| C2 | Publish window occupancy is cheaply observable | Instrument with AtomicU32 increment/decrement at window entry/exit | If overhead > 10ns: sample instead of counting |
| C3 | Safe mode exit hysteresis prevents flapping | Simulate oscillating load in integration test | If flapping persists: increase recovery_holdoff_s or add exponential backoff |
| C4 | PRAGMA-exposed knobs do not create security risk | Review: PRAGMAs are per-connection, cannot affect other connections | If shared state: gate behind connection-level capability |
| C5 | Guardrail decisions do not need persistence across restart | Verify: all state is derived from trailing window, not historical | If persistence needed: add startup calibration phase |

---

## 11. Consequences for Downstream Beads

| Downstream | What This Artifact Provides |
|------------|---------------------------|
| **bd-3t52f** (DRO abort policy) | Shared evidence vector (retry_rate, abort_rate), timescale separation contract, no override guarantee |
| **bd-77l3t** (HTM fast-path) | publish_contention signal can trigger HTM disable if abort storms are detected |
| **bd-db300.5.6.1** (E6 lane placement) | writer_saturation and tail_stress signals for topology-aware rebalancing |
| **bd-db300.7.5.5** (regime atlas) | Guardrail ID (G1–G12) and control_mode as regime classification dimensions |
| **bd-db300.7.5.6** (shadow oracle) | counterfactual_action and regret_delta_ns fields for shadow-run comparison |
