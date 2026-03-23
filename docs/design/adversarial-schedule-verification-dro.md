# Adversarial Schedule Verification for DRO SSI Abort Policy

**Bead:** `bd-1uguv`
**Date:** 2026-03-23
**Status:** Design + specification — ready for implementation
**Depends on:** bd-3t52f (DRO epic), existing DroLiveController, DroLossMatrix, T3 integration
**Coordinates with:** BlueLake's DRO work (bd-3vxje T3 integration, bd-1scmu Wasserstein tracker)

---

## Purpose

Prove that the DRO-based SSI abort policy outperforms the static/baseline
threshold under adversarial, non-stationary workloads — specifically on p99
commit latency, abort rate, and throughput — with deterministic replay,
explicit fail criteria, and structured logging.

This bead is the empirical verification surface for the DRO epic. Without it,
the DRO policy is a plausible mathematical construction without evidence that
it helps under adversarial conditions.

---

## 1. Adversarial Schedule Generator

### 1.1 Design: Deterministic Regime-Switching Workload

The generator produces a sequence of transaction intents with deterministic
page-access patterns that alternate between regimes designed to stress the DRO
controller's adaptation speed.

```rust
/// A single regime in the adversarial schedule.
#[derive(Debug, Clone)]
pub struct AdversarialRegime {
    /// Regime name for structured logging.
    pub name: &'static str,
    /// Duration of this regime in number of transactions.
    pub txn_count: u32,
    /// Page-access distribution.
    pub page_distribution: PageDistribution,
    /// Read:write ratio (0.0 = all writes, 1.0 = all reads).
    pub read_ratio: f32,
    /// Number of concurrent writers in this regime.
    pub concurrency: u16,
}

/// Page access distribution for a regime.
#[derive(Debug, Clone)]
pub enum PageDistribution {
    /// Uniform random across [1, max_page].
    Uniform { max_page: u32 },
    /// Zipfian with exponent s (s=0 is uniform, s=2 is extreme skew).
    Zipfian { max_page: u32, exponent: f64 },
    /// Single hot page (worst case for page-level MVCC).
    SingleHotPage { hot_page: u32, cold_pages: u32 },
    /// Bimodal: fraction p hits hot set, rest hits cold set.
    Bimodal { hot_pages: u32, cold_pages: u32, hot_fraction: f64 },
}
```

### 1.2 Canonical Adversarial Schedule

The verification uses a fixed 6-regime schedule that covers the DRO
controller's critical transitions:

| Phase | Regime | Txn Count | Distribution | Concurrency | Purpose |
|-------|--------|-----------|-------------|-------------|---------|
| 1 | `warm_uniform` | 500 | Uniform(1000) | 4 | Baseline: low contention, DRO should be calm |
| 2 | `sudden_skew` | 500 | Zipfian(1000, 2.0) | 4 | Shock: extreme skew, abort rate spikes, DRO must tighten |
| 3 | `sustained_skew` | 1000 | Zipfian(1000, 2.0) | 4 | DRO has time to adapt; radius should stabilize at high |
| 4 | `sudden_calm` | 500 | Uniform(1000) | 4 | Recovery: skew disappears, DRO must relax without overshooting |
| 5 | `hot_page_storm` | 500 | SingleHotPage(42, 1000) | 8 | Extreme: 8 writers on one page. Abort cascade guaranteed. DRO should recognize futility and clamp. |
| 6 | `recovery` | 500 | Uniform(1000) | 4 | Return to baseline. DRO must recover within this window. |

**Determinism:** The schedule is parameterized by a single `seed: u64`. All
page selections use a seeded `rand::rngs::StdRng`. Given the same seed,
the schedule is byte-identical across runs.

### 1.3 Generator API

```rust
/// Generate the canonical adversarial schedule.
pub fn canonical_adversarial_schedule(seed: u64) -> Vec<AdversarialRegime> { ... }

/// Generate transaction intents for a regime.
pub fn generate_regime_intents(
    regime: &AdversarialRegime,
    seed: u64,
) -> Vec<TxnIntent> { ... }

/// A single transaction intent.
pub struct TxnIntent {
    pub read_pages: Vec<PageNumber>,
    pub write_pages: Vec<PageNumber>,
    pub regime_name: &'static str,
    pub regime_index: u32,
    pub txn_index_in_regime: u32,
}
```

---

## 2. Verification Harness

### 2.1 Dual-Policy Comparison

Each run executes the same deterministic schedule under two policies:

| Policy | Description | Implementation |
|--------|-------------|----------------|
| **DRO** | DroLiveController with adaptive Wasserstein radius | Live controller, default config |
| **Static** | Fixed threshold (the old baseline before DRO) | DroLiveController with `window_commit_budget = u32::MAX` (never adapts) |

Both policies execute against the same page-lock table, commit index, and
version arena. The schedule generator produces identical intents for both runs.
Only the T3 decision differs.

### 2.2 Metrics Collected Per Regime

```rust
/// Per-regime metrics for comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeMetrics {
    pub regime_name: String,
    pub regime_index: u32,
    pub policy: String,  // "dro" or "static"

    // ── Throughput ──
    pub committed_txns: u64,
    pub aborted_txns: u64,
    pub abort_rate: f64,
    pub throughput_txns_per_sec: f64,

    // ── Latency (ns) ──
    pub p50_commit_ns: u64,
    pub p95_commit_ns: u64,
    pub p99_commit_ns: u64,
    pub p999_commit_ns: u64,
    pub max_commit_ns: u64,

    // ── DRO-specific ──
    pub wasserstein_radius_at_end: f64,
    pub dro_matrix_generation: u64,
    pub dro_matrix_swaps_in_regime: u64,
    pub cvar_penalty_mean: f64,
    pub cvar_penalty_max: f64,

    // ── Retry/contention ──
    pub page_lock_waits: u64,
    pub retry_count: u64,
}
```

---

## 3. Fail Criteria

The verification passes if and only if ALL of the following hold:

### 3.1 p99 Non-Regression (Primary)

```
For each regime R in {sustained_skew, hot_page_storm, recovery}:
  DRO.p99_commit_ns(R) <= Static.p99_commit_ns(R) * 1.10

  i.e., DRO p99 is no worse than 10% above static baseline.
```

**Rationale:** DRO's value proposition is tail-latency protection under skew.
If it makes p99 worse, the construction is counterproductive regardless of
throughput.

### 3.2 Throughput Non-Regression

```
For the entire schedule (all 6 regimes combined):
  DRO.throughput >= Static.throughput * 0.95

  i.e., DRO overall throughput is no worse than 5% below static.
```

**Rationale:** DRO may abort more conservatively during transitions, but over
the full schedule it should not lose material throughput.

### 3.3 DRO Adaptation Evidence

```
At the end of regime 3 (sustained_skew):
  DRO.wasserstein_radius > DRO.wasserstein_radius at end of regime 1

At the end of regime 6 (recovery):
  DRO.wasserstein_radius < DRO.wasserstein_radius at end of regime 3
```

**Rationale:** DRO must actually adapt. If the radius doesn't expand under
skew and contract during recovery, the controller is dead weight.

### 3.4 Abort Storm Suppression

```
For regime 5 (hot_page_storm):
  DRO.abort_rate <= Static.abort_rate

  i.e., DRO does not abort MORE than static under extreme contention.
```

**Rationale:** Under single-hot-page contention, DRO should recognize that
aborting is futile (the page will always conflict) and clamp rather than
cascade.

### 3.5 No Correctness Regression

```
For every regime:
  committed_txns + aborted_txns == total_txns_in_regime
  No MVCC corruption (CommitIndex monotonicity holds)
  No page-lock deadlock (all txns complete or abort within timeout)
```

---

## 4. Structured Logging

Every T3 decision during the adversarial schedule emits:

```rust
tracing::info!(
    target: "fsqlite::dro::adversarial",

    // ── Identity ──
    trace_id = %trace_id,
    scenario_id = "bd-1uguv.adversarial.v1",
    policy_id = %policy,           // "dro" or "static"
    regime_name = %regime,
    regime_index = regime_idx,
    txn_index = txn_idx,

    // ── T3 Decision ──
    t3_decision = %decision,       // "allow" or "abort"
    cvar_penalty = cvar,
    threshold = thresh,
    p_anomaly = p_anom,

    // ── DRO State ──
    wasserstein_radius = radius,
    dro_generation = gen,
    abort_rate_window = abort_rate,
    edge_rate_window = edge_rate,

    // ── Latency ──
    commit_latency_ns = latency,

    // ── Contention ──
    write_set_size = ws_size,
    pages_conflicted = conflicts,
    page_lock_wait_ns = lock_wait,
);
```

### Replay Artifact Schema

At the end of each run, a JSONL file is emitted:

```
artifacts/adversarial_dro/
├── schedule_v1.jsonl           # TxnIntent sequence (deterministic replay)
├── dro_decisions.jsonl         # Per-txn T3 decisions under DRO policy
├── static_decisions.jsonl      # Per-txn T3 decisions under static policy
├── regime_metrics.jsonl        # RegimeMetrics per regime per policy
├── comparison_summary.json     # Pass/fail verdict with fail criteria results
└── counterexample_bundle.json  # First-failure diagnostics (if any fail)
```

---

## 5. Named Entrypoints

### 5.1 Rust Test

```rust
// In crates/fsqlite-mvcc/src/ssi_anomaly_tests.rs or a new test module

#[test]
fn test_dro_adversarial_schedule_p99_non_regression() { ... }

#[test]
fn test_dro_adversarial_adaptation_evidence() { ... }

#[test]
fn test_dro_adversarial_abort_storm_suppression() { ... }

#[test]
fn test_adversarial_schedule_generator_determinism() { ... }
```

### 5.2 E2E Script

```bash
scripts/verify_adversarial_dro.sh
```

This script:
1. Builds with `--profile release-perf`.
2. Runs the canonical adversarial schedule with seed=42 under both policies.
3. Emits artifacts to `artifacts/adversarial_dro/`.
4. Evaluates all 5 fail criteria.
5. Prints pass/fail summary to stdout.
6. Exits with code 0 on pass, 1 on fail.
7. On fail, emits `counterexample_bundle.json` with the first regime that
   violated a criterion, including the full decision trace for that regime.

---

## 6. Implementation Plan

### Files to Modify

| File | Change | Scope |
|------|--------|-------|
| `crates/fsqlite-mvcc/src/ssi_anomaly_tests.rs` | Add adversarial schedule tests | ~200 lines of test code |
| `crates/fsqlite-mvcc/src/ssi_abort_policy.rs` | Add `AdversarialRegime`, `PageDistribution`, generator functions | ~150 lines |
| `scripts/verify_adversarial_dro.sh` | New entrypoint script | ~50 lines |

### Files NOT Modified

- `ssi_validation.rs` — no changes to the T3 decision path itself
- `begin_concurrent.rs` — no changes to the commit flow
- `connection.rs` — no changes to concurrent-mode defaults (INV-E1.1-1)

### Dependency on Existing DRO Work

This bead READS from:
- `DroLiveController::current_matrix()` — to observe DRO state per regime
- `DroVolatilityTracker` — to read Wasserstein radius
- `DroHotPathDecision` — to capture per-txn T3 decisions

It does NOT WRITE to any DRO controller state. The adversarial schedule
exercises the existing DRO pipeline end-to-end; it does not modify it.

---

## 7. Assumptions Ledger

| ID | Assumption | Verification | If Wrong |
|----|-----------|-------------|----------|
| D1 | 500 txns per regime is enough for DRO to adapt | Check that DRO matrix swaps > 0 in regimes 2–5 | Increase to 1000 per regime |
| D2 | Zipfian(s=2.0) produces enough page conflicts to stress DRO | Check abort_rate > 0.1 in regime 2 under static policy | Increase exponent to 3.0 |
| D3 | SingleHotPage with c=8 produces abort cascades | Check abort_rate > 0.5 in regime 5 under static policy | If not, reduce to c=4 on same page |
| D4 | 10% p99 regression tolerance is tight enough to be meaningful | Review: if DRO adds 10% to p99, is it still worth the adaptation? | Tighten to 5% if DRO's adaptation window is fast enough |
| D5 | Deterministic seed produces reproducible results under concurrent execution | Run seed=42 twice, compare regime_metrics | If non-deterministic, pin threads to cores and retry |

---

## 8. Consequences

| Downstream | What This Provides |
|------------|-------------------|
| **bd-3t52f** (DRO epic) | Empirical proof artifact — the "does it actually help?" evidence |
| **bd-db300.7.5.5** (regime atlas) | Regime-specific DRO behavior data for atlas classification |
| **bd-db300.7.5.6** (shadow oracle) | Dual-policy comparison as a shadow-oracle instance |
| **E4.3 guardrail** | abort_rate and retry_rate signals under adversarial load for guardrail threshold validation |
