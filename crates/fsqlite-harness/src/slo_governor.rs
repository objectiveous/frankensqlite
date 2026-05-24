//! Shadow-mode Swarm SLO resource governor policy engine.
//!
//! The policy in this module is intentionally pure: callers provide a sampled
//! evidence vector, and the evaluator returns an explainable decision without
//! mutating engine state. Enforced rollout and signal adapters live in later
//! beads.

#![allow(clippy::struct_excessive_bools)]

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Owning bead for the shadow policy engine.
pub const SWARM_SLO_POLICY_BEAD_ID: &str = "bd-swarm-slo-resource-governor-qb256.1";
/// Stable shadow policy identifier used in evidence artifacts.
pub const SWARM_SLO_POLICY_ID: &str = "swarm-slo-shadow.v1";
/// Machine-readable schema version for decisions emitted by this module.
pub const SWARM_SLO_DECISION_SCHEMA_VERSION: u32 = 1;

const PER_MILLE: u64 = 1_000;

/// Source of the sampled governor input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmSloSampleSource {
    /// Deterministic replay artifact or scorecard.
    Replay,
    /// Live harness run inside the test workspace.
    LiveHarness,
    /// Future production telemetry path.
    Production,
}

/// Control mode requested by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmSloControlMode {
    /// Observe only. Decisions are recommendations.
    Shadow,
    /// Enforced mode is a later rollout bead. The policy can still label input.
    Enforced,
}

/// GC escalation tier copied from the E4.3 guardrail design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmSloGcTier {
    /// Normal timer-driven cleanup.
    Normal,
    /// More frequent background cleanup.
    Elevated,
    /// Cleanup after every commit.
    Urgent,
    /// Inline cleanup before commit.
    Critical,
}

/// Helper lane affected by a budget-shrink decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmSloHelperLane {
    /// Replay/evidence lane.
    Evidence,
    /// Invalidation/coalescing lane.
    Invalidation,
    /// Agent build/test proof lane.
    AgentProof,
}

/// Recommendation emitted by the shadow policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SwarmSloAction {
    /// Allow work to proceed normally.
    Admit,
    /// Park briefly before admitting work.
    Defer {
        /// Maximum park budget in microseconds.
        park_budget_us: u32,
    },
    /// Fail closed with backpressure.
    ApplyBackpressure,
    /// Reduce optional helper work in one lane.
    ShrinkHelperBudget {
        /// Lane whose helper budget should be reduced.
        target_lane: SwarmSloHelperLane,
    },
    /// Recommend safe mode. This does not disable concurrent-writer defaults.
    ForceSafeMode,
    /// Promote GC into the critical path for the current pressure event.
    TriggerEmergencyGc,
    /// Trigger a checkpoint to reduce WAL pressure.
    TriggerCheckpoint,
}

/// Missing or degraded signal annotations carried with every decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmSloDegradedSignal {
    /// Replay scorecards do not yet expose p99.9 latency.
    MissingP999,
    /// Publish-window occupancy or p99 was unavailable.
    MissingPublishWindow,
    /// Live metrics were unavailable.
    MissingLiveMetrics,
    /// Sample age exceeded the stale-metrics threshold.
    StaleMetrics,
    /// Input came from replay only, not live telemetry.
    ReplayOnlyInput,
    /// Input labels were redacted for privacy.
    PrivacyRedactedInput,
}

/// Sampled evidence vector for one governor decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmSloGovernorInput {
    /// Stable run identifier.
    pub run_id: String,
    /// Trace identifier.
    pub trace_id: String,
    /// Scenario identifier.
    pub scenario_id: String,
    /// Backend label, such as `frankensqlite_concurrent`.
    pub backend: String,
    /// Resource profile identifier.
    pub profile_id: String,
    /// Sample source.
    pub sample_source: SwarmSloSampleSource,
    /// Requested control mode.
    pub control_mode: SwarmSloControlMode,
    /// Optional artifact path for replay-derived input.
    pub artifact_path: Option<String>,
    /// Optional artifact hash for replay-derived input.
    pub artifact_hash: Option<String>,
    /// Sample timestamp in milliseconds since caller-defined epoch.
    pub sample_ts_ms: u64,
    /// Sample age in milliseconds.
    pub sample_age_ms: u64,
    /// Sample window in milliseconds.
    pub sample_window_ms: u64,
    /// Actor count.
    pub actor_count: u32,
    /// Connection count.
    pub connection_count: u32,
    /// Writer count.
    pub writer_count: u32,
    /// Reader count.
    pub reader_count: u32,
    /// Statement count.
    pub statement_count: u64,
    /// Transaction count.
    pub transaction_count: u64,
    /// Concurrency level.
    pub concurrency_level: u32,
    /// Replay schedule seed or fingerprint.
    pub schedule_fingerprint: String,
    /// Available CPU cores.
    pub available_cores: u32,
    /// Configured helper threads.
    pub configured_helper_threads: u32,
    /// Active helper threads.
    pub active_helper_threads: u32,
    /// Memory limit for the resource profile.
    pub memory_limit_bytes: u64,
    /// Observed memory high-water mark.
    pub memory_high_water_bytes: u64,
    /// Page-cache footprint.
    pub page_cache_bytes: u64,
    /// CPU utilization in per-mille units.
    pub cpu_utilization_per_mille: u16,
    /// Active concurrent writers.
    pub active_writers: u32,
    /// Writers in the publish window, when measured.
    pub publish_window_occupancy: Option<u32>,
    /// Publish-window p99 in nanoseconds, when measured.
    pub publish_window_p99_ns: Option<u64>,
    /// Commit retry count.
    pub retry_count: u64,
    /// Retry rate in per-mille units.
    pub retry_rate_per_mille: u16,
    /// Abort count.
    pub abort_count: u64,
    /// Abort rate in per-mille units.
    pub abort_rate_per_mille: u16,
    /// Evidence queue depth.
    pub evidence_queue_depth: u32,
    /// Evidence queue drops in the sample window.
    pub evidence_queue_drops: u32,
    /// Evidence worker count.
    pub evidence_worker_count: u32,
    /// Wakeup queue depth.
    pub wakeup_queue_depth: u32,
    /// Wakeup-to-run p95 in nanoseconds.
    pub wakeup_to_run_p95_ns: Option<u64>,
    /// Wakeup-to-run p99 in nanoseconds.
    pub wakeup_to_run_p99_ns: Option<u64>,
    /// Maximum MVCC version-chain depth.
    pub max_chain_depth: u32,
    /// GC tier.
    pub gc_tier: SwarmSloGcTier,
    /// Whether inline GC is active.
    pub gc_inline_active: bool,
    /// WAL frames pending checkpoint.
    pub wal_frames_pending_checkpoint: u32,
    /// Whether a checkpoint is active.
    pub checkpoint_active: bool,
    /// Invalidation queue depth.
    pub invalidation_queue_depth: u32,
    /// Inline invalidation fallback count.
    pub invalidation_fallback_count: u32,
    /// Build/test saturation in per-mille units for proof lanes.
    pub build_or_test_saturation_per_mille: u16,
    /// Whether the operator path detected wedge risk.
    pub agent_wedge_risk: bool,
    /// User-visible p50 in nanoseconds.
    pub latency_p50_ns: u64,
    /// User-visible p95 in nanoseconds.
    pub latency_p95_ns: u64,
    /// User-visible p99 in nanoseconds.
    pub latency_p99_ns: u64,
    /// User-visible p99.9 in nanoseconds, when measured.
    pub latency_p999_ns: Option<u64>,
    /// Whether input labels were privacy-redacted.
    pub privacy_redacted: bool,
    /// Whether concurrent-writer default was observed as enabled.
    pub concurrent_mode_default_observed: bool,
    /// Degraded signals already known by the adapter.
    pub degraded_signals: Vec<SwarmSloDegradedSignal>,
}

impl SwarmSloGovernorInput {
    /// Build a healthy replay sample for deterministic tests and smoke adapters.
    pub fn healthy_replay(
        run_id: impl Into<String>,
        trace_id: impl Into<String>,
        scenario_id: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            trace_id: trace_id.into(),
            scenario_id: scenario_id.into(),
            backend: "frankensqlite_concurrent".to_owned(),
            profile_id: "high_capacity_server".to_owned(),
            sample_source: SwarmSloSampleSource::Replay,
            control_mode: SwarmSloControlMode::Shadow,
            artifact_path: Some(
                "crates/fsqlite-harness/conformance/agent_swarm_trace_sanitized_golden.json"
                    .to_owned(),
            ),
            artifact_hash: None,
            sample_ts_ms: 0,
            sample_age_ms: 0,
            sample_window_ms: 1_000,
            actor_count: 8,
            connection_count: 8,
            writer_count: 4,
            reader_count: 4,
            statement_count: 1_000,
            transaction_count: 100,
            concurrency_level: 8,
            schedule_fingerprint: "deterministic-smoke".to_owned(),
            available_cores: 64,
            configured_helper_threads: 3,
            active_helper_threads: 2,
            memory_limit_bytes: 256 * 1024 * 1024 * 1024,
            memory_high_water_bytes: 8 * 1024 * 1024,
            page_cache_bytes: 64 * 1024 * 1024,
            cpu_utilization_per_mille: 250,
            active_writers: 4,
            publish_window_occupancy: Some(1),
            publish_window_p99_ns: Some(1_000_000),
            retry_count: 0,
            retry_rate_per_mille: 0,
            abort_count: 0,
            abort_rate_per_mille: 0,
            evidence_queue_depth: 4,
            evidence_queue_drops: 0,
            evidence_worker_count: 1,
            wakeup_queue_depth: 0,
            wakeup_to_run_p95_ns: Some(100_000),
            wakeup_to_run_p99_ns: Some(150_000),
            max_chain_depth: 8,
            gc_tier: SwarmSloGcTier::Normal,
            gc_inline_active: false,
            wal_frames_pending_checkpoint: 128,
            checkpoint_active: false,
            invalidation_queue_depth: 0,
            invalidation_fallback_count: 0,
            build_or_test_saturation_per_mille: 0,
            agent_wedge_risk: false,
            latency_p50_ns: 1_000_000,
            latency_p95_ns: 2_000_000,
            latency_p99_ns: 4_000_000,
            latency_p999_ns: None,
            privacy_redacted: false,
            concurrent_mode_default_observed: true,
            degraded_signals: Vec::new(),
        }
    }
}

/// Tunable thresholds for the shadow policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmSloGovernorConfig {
    /// Writer saturation threshold in per-mille units.
    pub writer_saturation_threshold_per_mille: u64,
    /// Publish-window contention threshold in per-mille units.
    pub publish_contention_threshold_per_mille: u64,
    /// p99/p50 tail-stress threshold in per-mille units.
    pub tail_stress_threshold_per_mille: u64,
    /// Retry-rate threshold in per-mille units.
    pub retry_rate_threshold_per_mille: u16,
    /// Abort-rate threshold in per-mille units.
    pub abort_rate_threshold_per_mille: u16,
    /// Evidence drops above this threshold shrink the evidence budget.
    pub evidence_drop_threshold: u32,
    /// Invalidation fallbacks above this threshold shrink invalidation budget.
    pub invalidation_fallback_threshold: u32,
    /// Chain depth above this threshold triggers emergency GC.
    pub max_chain_depth_critical: u32,
    /// WAL frames above this threshold trigger checkpoint if none is active.
    pub wal_frames_restart_threshold: u32,
    /// User p99 above this threshold enters safe mode.
    pub safe_mode_p99_threshold_ns: u64,
    /// Inline-GC publish p99 above this threshold enters safe mode.
    pub safe_mode_publish_window_threshold_ns: u64,
    /// Build/test saturation threshold for proof-lane helper shrink.
    pub build_test_saturation_threshold_per_mille: u16,
    /// Metrics older than this are marked stale.
    pub stale_metrics_after_ms: u64,
    /// Consecutive healthy samples required to exit safe mode.
    pub recovery_healthy_decisions: u32,
}

impl Default for SwarmSloGovernorConfig {
    fn default() -> Self {
        Self {
            writer_saturation_threshold_per_mille: 950,
            publish_contention_threshold_per_mille: 500,
            tail_stress_threshold_per_mille: 15_000,
            retry_rate_threshold_per_mille: 300,
            abort_rate_threshold_per_mille: 200,
            evidence_drop_threshold: 100,
            invalidation_fallback_threshold: 10,
            max_chain_depth_critical: 256,
            wal_frames_restart_threshold: 10_000,
            safe_mode_p99_threshold_ns: 100_000_000,
            safe_mode_publish_window_threshold_ns: 10_000_000,
            build_test_saturation_threshold_per_mille: 900,
            stale_metrics_after_ms: 5_000,
            recovery_healthy_decisions: 3,
        }
    }
}

/// Stateful hysteresis for safe-mode exit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmSloGovernorState {
    next_decision_id: u64,
    safe_mode_active: bool,
    healthy_safe_mode_exit_samples: u32,
}

impl SwarmSloGovernorState {
    /// Whether safe mode is currently active.
    pub const fn safe_mode_active(&self) -> bool {
        self.safe_mode_active
    }

    /// Next monotonic decision identifier.
    pub const fn next_decision_id(&self) -> u64 {
        self.next_decision_id
    }
}

/// Derived pressure signals copied into each decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmSloDerivedSignals {
    /// Writer saturation in per-mille units.
    pub writer_saturation_per_mille: u64,
    /// Publish-window contention in per-mille units, when available.
    pub publish_contention_per_mille: Option<u64>,
    /// p99/p50 tail stress in per-mille units.
    pub tail_stress_per_mille: u64,
}

/// Explainable shadow governor decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmSloDecision {
    /// Decision schema version.
    pub schema_version: u32,
    /// Owning bead identifier.
    pub bead_id: String,
    /// Policy version.
    pub policy_id: String,
    /// Monotonic decision identifier.
    pub decision_id: u64,
    /// Matched guardrail or Swarm SLO rule.
    pub guardrail_id: String,
    /// Recommended action.
    pub action: SwarmSloAction,
    /// Caller-requested control mode.
    pub control_mode: SwarmSloControlMode,
    /// Full evidence snapshot used for the decision.
    pub evidence: SwarmSloGovernorInput,
    /// Derived pressure signals.
    pub derived: SwarmSloDerivedSignals,
    /// Decision counterfactual.
    pub counterfactual: String,
    /// Regret/risk explanation for the chosen action.
    pub regret: String,
    /// Degraded signals observed or inferred for the decision.
    pub degraded_signals: BTreeSet<SwarmSloDegradedSignal>,
    /// Observed concurrent-mode default at decision time.
    pub concurrent_mode_default_observed: bool,
    /// Safe-mode state after applying the decision.
    pub safe_mode_active_after: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuleMatch {
    guardrail_id: &'static str,
    action: SwarmSloAction,
    counterfactual: &'static str,
    regret: &'static str,
}

/// Evaluate one sample without preserving hysteresis between calls.
pub fn evaluate_swarm_slo_once(input: &SwarmSloGovernorInput) -> SwarmSloDecision {
    let mut state = SwarmSloGovernorState::default();
    evaluate_swarm_slo(input, &SwarmSloGovernorConfig::default(), &mut state)
}

/// Evaluate one sample with explicit config and state.
pub fn evaluate_swarm_slo(
    input: &SwarmSloGovernorInput,
    config: &SwarmSloGovernorConfig,
    state: &mut SwarmSloGovernorState,
) -> SwarmSloDecision {
    let decision_id = state.next_decision_id;
    state.next_decision_id = state.next_decision_id.saturating_add(1);

    let degraded_signals = normalize_degraded_signals(input, config);
    let derived = derive_signals(input);
    let raw_match = match_rule(input, config, &derived);
    let selected = apply_safe_mode_hysteresis(raw_match, state, input, config);

    SwarmSloDecision {
        schema_version: SWARM_SLO_DECISION_SCHEMA_VERSION,
        bead_id: SWARM_SLO_POLICY_BEAD_ID.to_owned(),
        policy_id: SWARM_SLO_POLICY_ID.to_owned(),
        decision_id,
        guardrail_id: selected.guardrail_id.to_owned(),
        action: selected.action,
        control_mode: input.control_mode,
        evidence: input.clone(),
        derived,
        counterfactual: selected.counterfactual.to_owned(),
        regret: selected.regret.to_owned(),
        degraded_signals,
        concurrent_mode_default_observed: input.concurrent_mode_default_observed,
        safe_mode_active_after: state.safe_mode_active,
    }
}

fn normalize_degraded_signals(
    input: &SwarmSloGovernorInput,
    config: &SwarmSloGovernorConfig,
) -> BTreeSet<SwarmSloDegradedSignal> {
    let mut signals = input
        .degraded_signals
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    if input.latency_p999_ns.is_none() {
        signals.insert(SwarmSloDegradedSignal::MissingP999);
    }

    if input.publish_window_occupancy.is_none() || input.publish_window_p99_ns.is_none() {
        signals.insert(SwarmSloDegradedSignal::MissingPublishWindow);
    }

    if input.sample_window_ms == 0 {
        signals.insert(SwarmSloDegradedSignal::MissingLiveMetrics);
    }

    if input.sample_age_ms > config.stale_metrics_after_ms {
        signals.insert(SwarmSloDegradedSignal::StaleMetrics);
    }

    if input.sample_source == SwarmSloSampleSource::Replay {
        signals.insert(SwarmSloDegradedSignal::ReplayOnlyInput);
    }

    if input.privacy_redacted {
        signals.insert(SwarmSloDegradedSignal::PrivacyRedactedInput);
    }

    signals
}

fn derive_signals(input: &SwarmSloGovernorInput) -> SwarmSloDerivedSignals {
    SwarmSloDerivedSignals {
        writer_saturation_per_mille: ratio_per_mille(
            u64::from(input.active_writers),
            u64::from(input.available_cores.max(1)),
        ),
        publish_contention_per_mille: input.publish_window_occupancy.map(|occupancy| {
            ratio_per_mille(
                u64::from(occupancy),
                u64::from(input.available_cores.max(1)),
            )
        }),
        tail_stress_per_mille: if input.latency_p50_ns == 0 {
            PER_MILLE
        } else {
            ratio_per_mille(input.latency_p99_ns, input.latency_p50_ns)
        },
    }
}

fn ratio_per_mille(numerator: u64, denominator: u64) -> u64 {
    numerator.saturating_mul(PER_MILLE) / denominator.max(1)
}

fn match_rule(
    input: &SwarmSloGovernorInput,
    config: &SwarmSloGovernorConfig,
    derived: &SwarmSloDerivedSignals,
) -> RuleMatch {
    if !input.concurrent_mode_default_observed {
        return rule(
            "G0_CONCURRENT_DEFAULT_OFF",
            SwarmSloAction::ApplyBackpressure,
            "pressure rules would run only after the concurrent-writer default is observed",
            "failing closed may reject useful work, but evaluating under a broken core invariant is worse",
        );
    }

    if input.max_chain_depth > config.max_chain_depth_critical {
        return rule(
            "G1",
            SwarmSloAction::TriggerEmergencyGc,
            "without emergency GC, version chains may keep growing",
            "inline GC adds latency, but bounds memory and visibility-chain pressure",
        );
    }

    if input.wal_frames_pending_checkpoint > config.wal_frames_restart_threshold
        && !input.checkpoint_active
    {
        return rule(
            "G2",
            SwarmSloAction::TriggerCheckpoint,
            "without checkpoint pressure relief, WAL scan and recovery cost can keep growing",
            "checkpoint work can contend with writers, but it reduces WAL pressure",
        );
    }

    if derived.writer_saturation_per_mille > config.writer_saturation_threshold_per_mille {
        return rule(
            "G3",
            SwarmSloAction::ApplyBackpressure,
            "additional admissions would increase writer-lane saturation",
            "backpressure may surface SQLITE_BUSY earlier, but avoids unbounded writer contention",
        );
    }

    if derived
        .publish_contention_per_mille
        .is_some_and(|contention| contention > config.publish_contention_threshold_per_mille)
    {
        return rule(
            "G4",
            SwarmSloAction::Defer {
                park_budget_us: 500,
            },
            "without a short defer, contenders keep colliding in the publish window",
            "parking adds small admission latency, but reduces collision probability",
        );
    }

    if derived.tail_stress_per_mille > config.tail_stress_threshold_per_mille {
        return rule(
            "G5",
            SwarmSloAction::Defer {
                park_budget_us: 1_000,
            },
            "without admission delay, pathological tail pressure can persist",
            "defer slows new work, but lets queues drain before p99 grows further",
        );
    }

    if input.retry_rate_per_mille > config.retry_rate_threshold_per_mille {
        return rule(
            "G6",
            SwarmSloAction::Defer {
                park_budget_us: 200,
            },
            "without retry dampening, compute is wasted on repeated attempts",
            "brief defer may lower peak throughput, but reduces retry churn",
        );
    }

    if input.abort_rate_per_mille > config.abort_rate_threshold_per_mille {
        return rule(
            "G7",
            SwarmSloAction::Defer {
                park_budget_us: 500,
            },
            "without abort dampening, hot-page contention or SSI pressure persists",
            "brief defer may delay independent work, but gives competing transactions time to finish",
        );
    }

    if input.evidence_queue_drops > config.evidence_drop_threshold {
        return rule(
            "G8",
            SwarmSloAction::ShrinkHelperBudget {
                target_lane: SwarmSloHelperLane::Evidence,
            },
            "without reducing evidence detail, the evidence lane keeps dropping samples",
            "lower evidence fidelity is worse for diagnostics, but preserves foreground work",
        );
    }

    if input.invalidation_fallback_count > config.invalidation_fallback_threshold {
        return rule(
            "G9",
            SwarmSloAction::ShrinkHelperBudget {
                target_lane: SwarmSloHelperLane::Invalidation,
            },
            "without invalidation budget shrink, inline fallbacks can keep increasing",
            "coarser invalidation may reduce freshness, but prevents fallback storms",
        );
    }

    if input.build_or_test_saturation_per_mille >= config.build_test_saturation_threshold_per_mille
        || input.agent_wedge_risk
    {
        return rule(
            "S1",
            SwarmSloAction::ShrinkHelperBudget {
                target_lane: SwarmSloHelperLane::AgentProof,
            },
            "without proof-lane shrink, agent builds and tests can starve useful database work",
            "slowing proof lanes delays feedback, but avoids wedging the shared host",
        );
    }

    if input.latency_p99_ns > config.safe_mode_p99_threshold_ns {
        return rule(
            "G10",
            SwarmSloAction::ForceSafeMode,
            "without safe mode, user-visible p99 is already above the hard threshold",
            "safe mode removes scheduling uncertainty, but reduces optional offload capacity",
        );
    }

    if input.gc_inline_active
        && input
            .publish_window_p99_ns
            .is_some_and(|p99| p99 > config.safe_mode_publish_window_threshold_ns)
    {
        return rule(
            "G11",
            SwarmSloAction::ForceSafeMode,
            "without safe mode, inline GC can dominate the publish window",
            "safe mode can reduce helper concurrency, but keeps critical work inline and bounded",
        );
    }

    rule(
        "G12",
        SwarmSloAction::Admit,
        "pressure rules would have delayed or throttled work",
        "admitting preserves throughput when sampled pressure is healthy",
    )
}

fn apply_safe_mode_hysteresis(
    raw_match: RuleMatch,
    state: &mut SwarmSloGovernorState,
    input: &SwarmSloGovernorInput,
    config: &SwarmSloGovernorConfig,
) -> RuleMatch {
    if raw_match.action == SwarmSloAction::ForceSafeMode {
        state.safe_mode_active = true;
        state.healthy_safe_mode_exit_samples = 0;
        return raw_match;
    }

    if !state.safe_mode_active {
        state.healthy_safe_mode_exit_samples = 0;
        return raw_match;
    }

    if safe_mode_exit_sample_is_healthy(raw_match.action, input, config) {
        state.healthy_safe_mode_exit_samples =
            state.healthy_safe_mode_exit_samples.saturating_add(1);

        if state.healthy_safe_mode_exit_samples >= config.recovery_healthy_decisions {
            state.safe_mode_active = false;
            state.healthy_safe_mode_exit_samples = 0;
            return raw_match;
        }
    } else {
        state.healthy_safe_mode_exit_samples = 0;
    }

    rule(
        "G_SAFE_HOLD",
        SwarmSloAction::ForceSafeMode,
        "raw pressure no longer requires immediate safe mode, but recovery holdoff has not elapsed",
        "holding safe mode protects against flapping at the cost of temporary helper suppression",
    )
}

fn safe_mode_exit_sample_is_healthy(
    raw_action: SwarmSloAction,
    input: &SwarmSloGovernorInput,
    config: &SwarmSloGovernorConfig,
) -> bool {
    raw_action == SwarmSloAction::Admit
        && input.latency_p99_ns < config.safe_mode_p99_threshold_ns / 2
        && !input.gc_inline_active
        && derive_signals(input).tail_stress_per_mille < config.tail_stress_threshold_per_mille / 2
}

const fn rule(
    guardrail_id: &'static str,
    action: SwarmSloAction,
    counterfactual: &'static str,
    regret: &'static str,
) -> RuleMatch {
    RuleMatch {
        guardrail_id,
        action,
        counterfactual,
        regret,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy_input() -> SwarmSloGovernorInput {
        SwarmSloGovernorInput::healthy_replay(
            "run-slo-policy-test",
            "trace-slo-policy-test",
            "scenario-slo-policy-test",
        )
    }

    #[test]
    fn healthy_replay_admits_and_records_degraded_replay_signals() {
        let decision = evaluate_swarm_slo_once(&healthy_input());

        assert_eq!(decision.guardrail_id, "G12");
        assert_eq!(decision.action, SwarmSloAction::Admit);
        assert_eq!(decision.decision_id, 0);
        assert!(decision.concurrent_mode_default_observed);
        assert!(
            decision
                .degraded_signals
                .contains(&SwarmSloDegradedSignal::MissingP999)
        );
        assert!(
            decision
                .degraded_signals
                .contains(&SwarmSloDegradedSignal::ReplayOnlyInput)
        );
    }

    #[test]
    fn emergency_gc_has_priority_over_writer_backpressure() {
        let mut input = healthy_input();
        input.max_chain_depth = 257;
        input.active_writers = 128;

        let decision = evaluate_swarm_slo_once(&input);

        assert_eq!(decision.guardrail_id, "G1");
        assert_eq!(decision.action, SwarmSloAction::TriggerEmergencyGc);
    }

    #[test]
    fn writer_saturation_applies_backpressure() {
        let mut input = healthy_input();
        input.available_cores = 64;
        input.active_writers = 64;

        let decision = evaluate_swarm_slo_once(&input);

        assert_eq!(decision.guardrail_id, "G3");
        assert_eq!(decision.action, SwarmSloAction::ApplyBackpressure);
        assert_eq!(decision.derived.writer_saturation_per_mille, 1_000);
    }

    #[test]
    fn publish_contention_defers_admission() {
        let mut input = healthy_input();
        input.available_cores = 64;
        input.publish_window_occupancy = Some(40);

        let decision = evaluate_swarm_slo_once(&input);

        assert_eq!(decision.guardrail_id, "G4");
        assert_eq!(
            decision.action,
            SwarmSloAction::Defer {
                park_budget_us: 500
            }
        );
    }

    #[test]
    fn retry_and_abort_pressure_are_distinct_defer_rules() {
        let mut retry_input = healthy_input();
        retry_input.retry_rate_per_mille = 301;
        let retry_decision = evaluate_swarm_slo_once(&retry_input);
        assert_eq!(retry_decision.guardrail_id, "G6");
        assert_eq!(
            retry_decision.action,
            SwarmSloAction::Defer {
                park_budget_us: 200
            }
        );

        let mut abort_input = healthy_input();
        abort_input.abort_rate_per_mille = 201;
        let abort_decision = evaluate_swarm_slo_once(&abort_input);
        assert_eq!(abort_decision.guardrail_id, "G7");
        assert_eq!(
            abort_decision.action,
            SwarmSloAction::Defer {
                park_budget_us: 500
            }
        );
    }

    #[test]
    fn missing_publish_window_is_degraded_without_fabricating_g11() {
        let mut input = healthy_input();
        input.gc_inline_active = true;
        input.publish_window_occupancy = None;
        input.publish_window_p99_ns = None;

        let decision = evaluate_swarm_slo_once(&input);

        assert_eq!(decision.guardrail_id, "G12");
        assert_eq!(decision.action, SwarmSloAction::Admit);
        assert!(
            decision
                .degraded_signals
                .contains(&SwarmSloDegradedSignal::MissingPublishWindow)
        );
    }

    #[test]
    fn proof_lane_saturation_shrinks_agent_helper_budget() {
        let mut input = healthy_input();
        input.build_or_test_saturation_per_mille = 900;

        let decision = evaluate_swarm_slo_once(&input);

        assert_eq!(decision.guardrail_id, "S1");
        assert_eq!(
            decision.action,
            SwarmSloAction::ShrinkHelperBudget {
                target_lane: SwarmSloHelperLane::AgentProof
            }
        );
    }

    #[test]
    fn safe_mode_exit_requires_sustained_healthy_samples() {
        let config = SwarmSloGovernorConfig {
            recovery_healthy_decisions: 3,
            ..SwarmSloGovernorConfig::default()
        };
        let mut state = SwarmSloGovernorState::default();
        let mut unsafe_input = healthy_input();
        unsafe_input.latency_p50_ns = 20_000_000;
        unsafe_input.latency_p99_ns = 150_000_000;

        let enter = evaluate_swarm_slo(&unsafe_input, &config, &mut state);
        assert_eq!(enter.guardrail_id, "G10");
        assert!(state.safe_mode_active());

        let healthy = healthy_input();
        let hold1 = evaluate_swarm_slo(&healthy, &config, &mut state);
        let hold2 = evaluate_swarm_slo(&healthy, &config, &mut state);
        let exit = evaluate_swarm_slo(&healthy, &config, &mut state);

        assert_eq!(hold1.guardrail_id, "G_SAFE_HOLD");
        assert_eq!(hold2.guardrail_id, "G_SAFE_HOLD");
        assert_eq!(exit.guardrail_id, "G12");
        assert!(!state.safe_mode_active());
        assert_eq!(exit.decision_id, 3);
    }

    #[test]
    fn concurrent_mode_default_guard_fails_closed_without_mutating_connection() {
        let conn = fsqlite_core::connection::Connection::open(":memory:").unwrap();
        assert!(conn.is_concurrent_mode_default());

        let mut input = healthy_input();
        input.concurrent_mode_default_observed = conn.is_concurrent_mode_default();
        input.latency_p50_ns = 20_000_000;
        input.latency_p99_ns = 150_000_000;
        let safe_mode_decision = evaluate_swarm_slo_once(&input);
        assert_eq!(safe_mode_decision.action, SwarmSloAction::ForceSafeMode);
        assert!(
            conn.is_concurrent_mode_default(),
            "shadow policy must not disable concurrent-writer default"
        );

        input.concurrent_mode_default_observed = false;
        input.latency_p99_ns = 4_000_000;
        let fail_closed_decision = evaluate_swarm_slo_once(&input);
        assert_eq!(
            fail_closed_decision.guardrail_id,
            "G0_CONCURRENT_DEFAULT_OFF"
        );
        assert_eq!(
            fail_closed_decision.action,
            SwarmSloAction::ApplyBackpressure
        );
    }

    #[test]
    fn decisions_are_json_serializable_for_evidence_artifacts() {
        let decision = evaluate_swarm_slo_once(&healthy_input());
        let encoded = serde_json::to_string(&decision).unwrap();

        assert!(encoded.contains(SWARM_SLO_POLICY_ID));
        assert!(encoded.contains("swarm-slo-shadow.v1"));
    }
}
