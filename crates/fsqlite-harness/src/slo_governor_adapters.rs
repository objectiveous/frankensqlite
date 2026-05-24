//! Replay and live-harness signal adapters for the Swarm SLO governor.
//!
//! The adapters in this module normalize existing harness artifacts into the
//! single [`SwarmSloGovernorInput`] schema. They are intentionally private to
//! the harness crate surface and do not expose a competing production metrics
//! endpoint; production export belongs to the `bd-zywqc.11` metrics work.

#![allow(clippy::struct_excessive_bools)]

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::agent_swarm_trace::{
    AgentSwarmEvidenceManifest, AgentSwarmReplayBackend, AgentSwarmReplayReport,
    AgentSwarmReplaySchedule, AgentSwarmResourceProfileId, AgentSwarmResourceScorecard,
    AgentSwarmStatementReplay,
};
use crate::slo_governor::{
    SwarmSloControlMode, SwarmSloDegradedSignal, SwarmSloGcTier, SwarmSloGovernorInput,
    SwarmSloSampleSource,
};

/// Owning bead for the replay/live signal adapter slice.
pub const SWARM_SLO_ADAPTER_BEAD_ID: &str = "bd-swarm-slo-resource-governor-qb256.4";

/// Configuration applied while adapting deterministic replay scorecards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmSloReplayAdapterConfig {
    /// Control mode attached to the resulting governor input.
    pub control_mode: SwarmSloControlMode,
    /// Sample timestamp in milliseconds since caller-defined epoch.
    pub sample_ts_ms: u64,
    /// Sample age in milliseconds.
    pub sample_age_ms: u64,
    /// Replay adapters leave this at zero to mark live metrics unavailable.
    pub sample_window_ms: u64,
    /// Artifact path copied from the evidence manifest when present.
    pub artifact_path: Option<String>,
    /// Artifact hash copied from the evidence manifest when present.
    pub artifact_hash: Option<String>,
    /// Configured helper threads, when known by the caller.
    pub configured_helper_threads: u32,
    /// Active helper threads, when known by the caller.
    pub active_helper_threads: u32,
    /// Evidence workers, when known by the caller.
    pub evidence_worker_count: u32,
}

impl Default for SwarmSloReplayAdapterConfig {
    fn default() -> Self {
        Self {
            control_mode: SwarmSloControlMode::Shadow,
            sample_ts_ms: 0,
            sample_age_ms: 0,
            sample_window_ms: 0,
            artifact_path: None,
            artifact_hash: None,
            configured_helper_threads: 0,
            active_helper_threads: 0,
            evidence_worker_count: 0,
        }
    }
}

impl SwarmSloReplayAdapterConfig {
    /// Copy replay artifact identity from an existing evidence manifest.
    pub fn with_evidence_manifest(mut self, manifest: &AgentSwarmEvidenceManifest) -> Self {
        self.artifact_path = Some(manifest.trace_artifact_path.clone());
        self.artifact_hash = Some(manifest.artifact_hash.clone());
        self
    }
}

/// Failure while translating a replay artifact into governor input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwarmSloAdapterError {
    /// Replay report did not contain the requested backend.
    ReplayBackendMissing {
        /// Requested backend.
        backend: AgentSwarmReplayBackend,
    },
    /// Resource scorecard did not contain the requested backend.
    ScorecardBackendMissing {
        /// Requested backend.
        backend: AgentSwarmReplayBackend,
    },
    /// Report and scorecard identity fields disagree.
    MismatchedReplayIdentity {
        /// Identity field name.
        field: &'static str,
        /// Value from the replay report.
        report_value: String,
        /// Value from the resource scorecard.
        scorecard_value: String,
    },
}

impl fmt::Display for SwarmSloAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReplayBackendMissing { backend } => {
                write!(f, "replay report is missing backend {backend:?}")
            }
            Self::ScorecardBackendMissing { backend } => {
                write!(f, "resource scorecard is missing backend {backend:?}")
            }
            Self::MismatchedReplayIdentity {
                field,
                report_value,
                scorecard_value,
            } => write!(
                f,
                "replay report {field}={report_value:?} does not match scorecard {field}={scorecard_value:?}",
            ),
        }
    }
}

impl std::error::Error for SwarmSloAdapterError {}

/// Adapt a replay report plus resource scorecard into one governor input row.
pub fn adapt_replay_scorecard_to_swarm_slo_input(
    report: &AgentSwarmReplayReport,
    scorecard: &AgentSwarmResourceScorecard,
    backend: AgentSwarmReplayBackend,
    config: &SwarmSloReplayAdapterConfig,
) -> Result<SwarmSloGovernorInput, SwarmSloAdapterError> {
    validate_replay_identity(report, scorecard)?;

    let replay_backend = report
        .backends
        .iter()
        .find(|candidate| candidate.identity.backend == backend)
        .ok_or(SwarmSloAdapterError::ReplayBackendMissing { backend })?;
    let scorecard_backend = scorecard
        .backends
        .iter()
        .find(|candidate| candidate.backend == backend)
        .ok_or(SwarmSloAdapterError::ScorecardBackendMissing { backend })?;

    let actor_count = capped_u32(scorecard_backend.per_actor_statement_count.len());
    let connection_count = unique_count_u32(
        replay_backend
            .statements
            .iter()
            .map(|statement| statement.connection_id.as_str()),
    );
    let writer_count = writer_actor_count(&replay_backend.statements);
    let reader_count = actor_count.saturating_sub(writer_count);
    let concurrency_level = actor_count.max(connection_count).max(unique_count_u32(
        replay_backend
            .statements
            .iter()
            .map(|statement| statement.concurrency_group.as_str()),
    ));

    Ok(SwarmSloGovernorInput {
        run_id: report.run_id.clone(),
        trace_id: report.trace_id.clone(),
        scenario_id: report.scenario_id.clone(),
        backend: backend.as_str().to_owned(),
        profile_id: resource_profile_id_label(scorecard.profile.profile_id).to_owned(),
        sample_source: SwarmSloSampleSource::Replay,
        control_mode: config.control_mode,
        artifact_path: config.artifact_path.clone(),
        artifact_hash: config.artifact_hash.clone(),
        first_failure_diag: scorecard.first_failure_diag.clone(),
        sample_ts_ms: config.sample_ts_ms,
        sample_age_ms: config.sample_age_ms,
        sample_window_ms: config.sample_window_ms,
        actor_count,
        connection_count,
        writer_count,
        reader_count,
        statement_count: usize_to_u64(replay_backend.summary.statements_total),
        transaction_count: usize_to_u64(report.transaction_count),
        concurrency_level,
        schedule_fingerprint: format!(
            "seed={};schedule={}",
            report.seed,
            replay_schedule_label(report.schedule),
        ),
        available_cores: scorecard_backend.core_count,
        configured_helper_threads: config.configured_helper_threads,
        active_helper_threads: config.active_helper_threads,
        memory_limit_bytes: scorecard_backend.memory_limit_bytes,
        memory_high_water_bytes: scorecard_backend.memory_high_water_bytes,
        page_cache_bytes: scorecard_backend.page_cache_footprint_bytes,
        cpu_utilization_per_mille: scorecard_backend.cpu_utilization_per_mille,
        active_writers: writer_count,
        publish_window_occupancy: None,
        publish_window_p99_ns: None,
        retry_count: replay_backend.summary.retry_count,
        retry_rate_per_mille: capped_u16(scorecard_backend.retry_rate_per_mille),
        abort_count: usize_to_u64(replay_backend.summary.abort_count),
        abort_rate_per_mille: capped_u16(scorecard_backend.abort_rate_per_mille),
        evidence_queue_depth: 0,
        evidence_queue_drops: 0,
        evidence_worker_count: config.evidence_worker_count,
        wakeup_queue_depth: 0,
        wakeup_to_run_p95_ns: None,
        wakeup_to_run_p99_ns: None,
        max_chain_depth: 0,
        gc_tier: SwarmSloGcTier::Normal,
        gc_inline_active: false,
        wal_frames_pending_checkpoint: 0,
        checkpoint_active: false,
        invalidation_queue_depth: 0,
        invalidation_fallback_count: 0,
        build_or_test_saturation_per_mille: 0,
        agent_wedge_risk: false,
        latency_p50_ns: scorecard_backend.latency_p50_ns,
        latency_p95_ns: scorecard_backend.latency_p95_ns,
        latency_p99_ns: scorecard_backend.latency_p99_ns,
        latency_p999_ns: None,
        privacy_redacted: true,
        concurrent_mode_default_observed: scorecard_backend.concurrent_writer_default,
        degraded_signals: replay_degraded_signals(),
    })
}

/// Measured live-harness sample before normalization into governor input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmSloLiveHarnessSample {
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
    /// Requested control mode.
    pub control_mode: SwarmSloControlMode,
    /// First live diagnostic, or `none` when absent.
    pub first_failure_diag: String,
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
    /// Schedule or workload fingerprint.
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
    /// Degraded signals already known by the live adapter.
    pub degraded_signals: Vec<SwarmSloDegradedSignal>,
}

/// Adapt a measured live-harness sample into the shared governor input schema.
pub fn adapt_live_harness_sample_to_swarm_slo_input(
    sample: SwarmSloLiveHarnessSample,
) -> SwarmSloGovernorInput {
    SwarmSloGovernorInput {
        run_id: sample.run_id,
        trace_id: sample.trace_id,
        scenario_id: sample.scenario_id,
        backend: sample.backend,
        profile_id: sample.profile_id,
        sample_source: SwarmSloSampleSource::LiveHarness,
        control_mode: sample.control_mode,
        artifact_path: None,
        artifact_hash: None,
        first_failure_diag: sample.first_failure_diag,
        sample_ts_ms: sample.sample_ts_ms,
        sample_age_ms: sample.sample_age_ms,
        sample_window_ms: sample.sample_window_ms,
        actor_count: sample.actor_count,
        connection_count: sample.connection_count,
        writer_count: sample.writer_count,
        reader_count: sample.reader_count,
        statement_count: sample.statement_count,
        transaction_count: sample.transaction_count,
        concurrency_level: sample.concurrency_level,
        schedule_fingerprint: sample.schedule_fingerprint,
        available_cores: sample.available_cores,
        configured_helper_threads: sample.configured_helper_threads,
        active_helper_threads: sample.active_helper_threads,
        memory_limit_bytes: sample.memory_limit_bytes,
        memory_high_water_bytes: sample.memory_high_water_bytes,
        page_cache_bytes: sample.page_cache_bytes,
        cpu_utilization_per_mille: sample.cpu_utilization_per_mille,
        active_writers: sample.active_writers,
        publish_window_occupancy: sample.publish_window_occupancy,
        publish_window_p99_ns: sample.publish_window_p99_ns,
        retry_count: sample.retry_count,
        retry_rate_per_mille: sample.retry_rate_per_mille,
        abort_count: sample.abort_count,
        abort_rate_per_mille: sample.abort_rate_per_mille,
        evidence_queue_depth: sample.evidence_queue_depth,
        evidence_queue_drops: sample.evidence_queue_drops,
        evidence_worker_count: sample.evidence_worker_count,
        wakeup_queue_depth: sample.wakeup_queue_depth,
        wakeup_to_run_p95_ns: sample.wakeup_to_run_p95_ns,
        wakeup_to_run_p99_ns: sample.wakeup_to_run_p99_ns,
        max_chain_depth: sample.max_chain_depth,
        gc_tier: sample.gc_tier,
        gc_inline_active: sample.gc_inline_active,
        wal_frames_pending_checkpoint: sample.wal_frames_pending_checkpoint,
        checkpoint_active: sample.checkpoint_active,
        invalidation_queue_depth: sample.invalidation_queue_depth,
        invalidation_fallback_count: sample.invalidation_fallback_count,
        build_or_test_saturation_per_mille: sample.build_or_test_saturation_per_mille,
        agent_wedge_risk: sample.agent_wedge_risk,
        latency_p50_ns: sample.latency_p50_ns,
        latency_p95_ns: sample.latency_p95_ns,
        latency_p99_ns: sample.latency_p99_ns,
        latency_p999_ns: sample.latency_p999_ns,
        privacy_redacted: sample.privacy_redacted,
        concurrent_mode_default_observed: sample.concurrent_mode_default_observed,
        degraded_signals: sample.degraded_signals,
    }
}

fn validate_replay_identity(
    report: &AgentSwarmReplayReport,
    scorecard: &AgentSwarmResourceScorecard,
) -> Result<(), SwarmSloAdapterError> {
    replay_identity_field("run_id", &report.run_id, &scorecard.run_id)?;
    replay_identity_field("trace_id", &report.trace_id, &scorecard.trace_id)?;
    replay_identity_field("scenario_id", &report.scenario_id, &scorecard.scenario_id)
}

fn replay_identity_field(
    field: &'static str,
    report_value: &str,
    scorecard_value: &str,
) -> Result<(), SwarmSloAdapterError> {
    if report_value == scorecard_value {
        Ok(())
    } else {
        Err(SwarmSloAdapterError::MismatchedReplayIdentity {
            field,
            report_value: report_value.to_owned(),
            scorecard_value: scorecard_value.to_owned(),
        })
    }
}

fn writer_actor_count(statements: &[AgentSwarmStatementReplay]) -> u32 {
    let mut writers = BTreeSet::new();
    for statement in statements {
        if statement_is_write_work(statement) {
            writers.insert(statement.actor_id.as_str());
        }
    }
    capped_u32(writers.len())
}

fn statement_is_write_work(statement: &AgentSwarmStatementReplay) -> bool {
    let keyword = first_sql_keyword(&statement.materialized_sql);
    WRITE_KEYWORDS
        .iter()
        .any(|candidate| keyword.eq_ignore_ascii_case(candidate))
}

fn first_sql_keyword(sql: &str) -> &str {
    sql.trim_start()
        .split(|c: char| !c.is_ascii_alphabetic())
        .next()
        .unwrap_or_default()
}

const WRITE_KEYWORDS: &[&str] = &[
    "INSERT", "UPDATE", "DELETE", "REPLACE", "CREATE", "DROP", "ALTER", "VACUUM", "PRAGMA",
];

fn replay_degraded_signals() -> Vec<SwarmSloDegradedSignal> {
    vec![
        SwarmSloDegradedSignal::MissingP999,
        SwarmSloDegradedSignal::MissingPublishWindow,
        SwarmSloDegradedSignal::MissingLiveMetrics,
        SwarmSloDegradedSignal::ReplayOnlyInput,
        SwarmSloDegradedSignal::PrivacyRedactedInput,
    ]
}

fn unique_count_u32<'a>(values: impl Iterator<Item = &'a str>) -> u32 {
    capped_u32(values.collect::<BTreeSet<_>>().len())
}

fn capped_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn capped_u16(value: u64) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

const fn resource_profile_id_label(profile_id: AgentSwarmResourceProfileId) -> &'static str {
    match profile_id {
        AgentSwarmResourceProfileId::LocalSmoke => "local_smoke",
        AgentSwarmResourceProfileId::Workstation => "workstation",
        AgentSwarmResourceProfileId::HighCapacityServer => "high_capacity_server",
    }
}

const fn replay_schedule_label(schedule: AgentSwarmReplaySchedule) -> &'static str {
    match schedule {
        AgentSwarmReplaySchedule::TraceOrder => "trace_order",
        AgentSwarmReplaySchedule::ConcurrencyGroupThenOrder => "concurrency_group_then_order",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_swarm_trace::{
        AgentSwarmEvidenceManifestConfig, AgentSwarmReplayConfig, AgentSwarmResourceProfile,
        AgentSwarmResourceScorecardConfig, FIRST_FAILURE_DIAGNOSTIC_ABSENT,
        build_agent_swarm_evidence_manifest, load_agent_swarm_trace_json, replay_agent_swarm_trace,
        score_agent_swarm_resource_envelope,
    };
    use crate::slo_governor::{SwarmSloAction, evaluate_swarm_slo_once};

    const GOLDEN_TRACE_JSON: &str =
        include_str!("../conformance/agent_swarm_trace_sanitized_golden.json");
    const GOLDEN_TRACE_PATH: &str =
        "crates/fsqlite-harness/conformance/agent_swarm_trace_sanitized_golden.json";

    #[test]
    fn golden_replay_adapts_without_fabricating_live_only_signals()
    -> Result<(), Box<dyn std::error::Error>> {
        let trace = load_agent_swarm_trace_json(GOLDEN_TRACE_JSON)?;
        let replay_config = AgentSwarmReplayConfig::smoke(0x51_0A_DA_7A);
        let report = replay_agent_swarm_trace(&trace, &replay_config)?;
        let scorecard_config =
            AgentSwarmResourceScorecardConfig::new(AgentSwarmResourceProfile::local_smoke());
        let scorecard = score_agent_swarm_resource_envelope(&report, &scorecard_config);
        let manifest_config = AgentSwarmEvidenceManifestConfig::new(
            GOLDEN_TRACE_PATH,
            replay_config.commands.smoke_command.clone(),
        )
        .without_regression_proposals();
        let manifest = build_agent_swarm_evidence_manifest(&report, &manifest_config);
        let adapter_config =
            SwarmSloReplayAdapterConfig::default().with_evidence_manifest(&manifest);

        let input = adapt_replay_scorecard_to_swarm_slo_input(
            &report,
            &scorecard,
            AgentSwarmReplayBackend::FrankenSqliteConcurrent,
            &adapter_config,
        )?;

        assert_eq!(input.sample_source, SwarmSloSampleSource::Replay);
        assert_eq!(input.trace_id, trace.trace_id);
        assert_eq!(input.run_id, trace.run_id);
        assert_eq!(input.scenario_id, trace.scenario_id);
        assert_eq!(input.artifact_path.as_deref(), Some(GOLDEN_TRACE_PATH));
        assert_eq!(input.first_failure_diag, scorecard.first_failure_diag);
        assert!(
            input
                .first_failure_diag
                .contains("topology_error: connection conn-b"),
        );
        assert_eq!(input.latency_p999_ns, None);
        assert_eq!(input.publish_window_occupancy, None);
        assert_eq!(input.publish_window_p99_ns, None);
        assert!(input.concurrent_mode_default_observed);
        assert!(
            input
                .degraded_signals
                .contains(&SwarmSloDegradedSignal::MissingP999),
        );

        let decision = evaluate_swarm_slo_once(&input);
        assert_ne!(decision.guardrail_id, "G11");
        assert!(
            decision
                .degraded_signals
                .contains(&SwarmSloDegradedSignal::MissingPublishWindow),
        );
        assert!(
            decision
                .degraded_signals
                .contains(&SwarmSloDegradedSignal::ReplayOnlyInput),
        );

        Ok(())
    }

    #[test]
    fn live_harness_sample_preserves_measured_publish_and_p999_signals() {
        let sample = healthy_live_sample();
        let input = adapt_live_harness_sample_to_swarm_slo_input(sample);

        assert_eq!(input.sample_source, SwarmSloSampleSource::LiveHarness);
        assert_eq!(input.publish_window_occupancy, Some(2));
        assert_eq!(input.publish_window_p99_ns, Some(700_000));
        assert_eq!(input.latency_p999_ns, Some(4_000_000));
        assert_eq!(input.first_failure_diag, FIRST_FAILURE_DIAGNOSTIC_ABSENT);

        let decision = evaluate_swarm_slo_once(&input);
        assert_eq!(decision.action, SwarmSloAction::Admit);
        assert!(
            !decision
                .degraded_signals
                .contains(&SwarmSloDegradedSignal::MissingP999),
        );
        assert!(
            !decision
                .degraded_signals
                .contains(&SwarmSloDegradedSignal::MissingPublishWindow),
        );
    }

    #[test]
    fn missing_scorecard_backend_is_reported() -> Result<(), Box<dyn std::error::Error>> {
        let trace = load_agent_swarm_trace_json(GOLDEN_TRACE_JSON)?;
        let replay_config = AgentSwarmReplayConfig::smoke(0xB4_CE);
        let report = replay_agent_swarm_trace(&trace, &replay_config)?;
        let scorecard_config =
            AgentSwarmResourceScorecardConfig::new(AgentSwarmResourceProfile::local_smoke());
        let mut scorecard = score_agent_swarm_resource_envelope(&report, &scorecard_config);
        scorecard
            .backends
            .retain(|backend| backend.backend != AgentSwarmReplayBackend::CSqliteOracle);

        let Err(error) = adapt_replay_scorecard_to_swarm_slo_input(
            &report,
            &scorecard,
            AgentSwarmReplayBackend::CSqliteOracle,
            &SwarmSloReplayAdapterConfig::default(),
        ) else {
            return Err(std::io::Error::other("missing backend was accepted").into());
        };

        assert_eq!(
            error,
            SwarmSloAdapterError::ScorecardBackendMissing {
                backend: AgentSwarmReplayBackend::CSqliteOracle,
            },
        );

        Ok(())
    }

    fn healthy_live_sample() -> SwarmSloLiveHarnessSample {
        SwarmSloLiveHarnessSample {
            run_id: "run-live".to_owned(),
            trace_id: "trace-live".to_owned(),
            scenario_id: "scenario-live".to_owned(),
            backend: AgentSwarmReplayBackend::FrankenSqliteConcurrent
                .as_str()
                .to_owned(),
            profile_id: "local_smoke".to_owned(),
            control_mode: SwarmSloControlMode::Shadow,
            first_failure_diag: FIRST_FAILURE_DIAGNOSTIC_ABSENT.to_owned(),
            sample_ts_ms: 1_000,
            sample_age_ms: 100,
            sample_window_ms: 1_000,
            actor_count: 4,
            connection_count: 4,
            writer_count: 2,
            reader_count: 2,
            statement_count: 100,
            transaction_count: 20,
            concurrency_level: 4,
            schedule_fingerprint: "live-smoke".to_owned(),
            available_cores: 16,
            configured_helper_threads: 3,
            active_helper_threads: 2,
            memory_limit_bytes: 32 * 1024 * 1024,
            memory_high_water_bytes: 4 * 1024 * 1024,
            page_cache_bytes: 2 * 1024 * 1024,
            cpu_utilization_per_mille: 150,
            active_writers: 2,
            publish_window_occupancy: Some(2),
            publish_window_p99_ns: Some(700_000),
            retry_count: 0,
            retry_rate_per_mille: 0,
            abort_count: 0,
            abort_rate_per_mille: 0,
            evidence_queue_depth: 1,
            evidence_queue_drops: 0,
            evidence_worker_count: 1,
            wakeup_queue_depth: 0,
            wakeup_to_run_p95_ns: Some(80_000),
            wakeup_to_run_p99_ns: Some(120_000),
            max_chain_depth: 4,
            gc_tier: SwarmSloGcTier::Normal,
            gc_inline_active: false,
            wal_frames_pending_checkpoint: 32,
            checkpoint_active: false,
            invalidation_queue_depth: 0,
            invalidation_fallback_count: 0,
            build_or_test_saturation_per_mille: 0,
            agent_wedge_risk: false,
            latency_p50_ns: 1_000_000,
            latency_p95_ns: 2_000_000,
            latency_p99_ns: 3_000_000,
            latency_p999_ns: Some(4_000_000),
            privacy_redacted: false,
            concurrent_mode_default_observed: true,
            degraded_signals: Vec::new(),
        }
    }
}
