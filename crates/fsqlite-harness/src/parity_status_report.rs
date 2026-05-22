//! User-facing parity status report and evidence freshness dashboard (bd-2yqp6.7.5).
//!
//! The report stitches together the canonical parity taxonomy, oracle preflight
//! readiness diagnostics, and the current differential frontier into one JSON
//! artifact plus a deterministic Markdown summary.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::oracle_preflight_doctor::OraclePreflightReport;
use crate::parity_taxonomy::{
    Feature, FeatureUniverse, ParityScore, ParityStatus, build_canonical_universe,
};

/// Owning bead identifier.
pub const PARITY_STATUS_REPORT_BEAD_ID: &str = "bd-2yqp6.7.5";
/// Report schema version.
pub const PARITY_STATUS_REPORT_SCHEMA_VERSION: &str = "fsqlite.parity_status_report.v1";
/// Default evidence freshness budget: 24 hours.
pub const DEFAULT_EVIDENCE_FRESHNESS_BUDGET_MS: u128 = 24 * 60 * 60 * 1_000;

/// Status of one evidence source feeding the user report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceFreshnessSource {
    /// Stable source identifier.
    pub source_id: String,
    /// Whether the source was present.
    pub present: bool,
    /// Unix millisecond timestamp observed from the source, when available.
    pub observed_unix_ms: Option<u128>,
    /// Age relative to report generation, when computable.
    pub age_ms: Option<u128>,
    /// Whether the source age is within budget.
    pub fresh: bool,
    /// Optional artifact path.
    pub artifact_path: Option<String>,
    /// Optional artifact SHA-256.
    pub artifact_sha256: Option<String>,
    /// Deterministic replay command for regenerating the source.
    pub replay_command: Option<String>,
}

/// Aggregate evidence freshness status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceFreshnessStatus {
    /// Budget used for all evidence freshness checks.
    pub freshness_budget_ms: u128,
    /// Whether every required source is present and fresh.
    pub overall_fresh: bool,
    /// Per-source freshness state.
    pub sources: Vec<EvidenceFreshnessSource>,
}

/// Per-feature support state shown to users.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureStatusEntry {
    /// Feature id from the canonical taxonomy.
    pub feature_id: String,
    /// Feature title.
    pub title: String,
    /// Feature category.
    pub category: String,
    /// Current support state.
    pub support_state: String,
    /// Human-readable residual gap statement.
    pub residual_gap: String,
    /// Feature-specific evidence bead ids.
    pub evidence_beads: Vec<String>,
    /// Deterministic replay pointers for this feature.
    pub replay_pointers: Vec<String>,
}

/// Oracle preflight readiness summary embedded in the status report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OraclePreflightStatus {
    /// Whether a doctor report was provided.
    pub present: bool,
    /// Doctor outcome (`green`, `yellow`, or `red`) when present.
    pub outcome: Option<String>,
    /// Whether the doctor run is certifying.
    pub certifying: Option<bool>,
    /// First preflight failure summary, when present.
    pub first_failure_summary: Option<String>,
    /// First preflight remediation command, when present.
    pub first_failure_fix_command: Option<String>,
    /// Fixture manifest path used by the doctor.
    pub fixture_manifest_path: Option<String>,
    /// Fixture manifest SHA-256, when available.
    pub fixture_manifest_sha256: Option<String>,
    /// Doctor replay command.
    pub replay_command: Option<String>,
}

/// Remediation playbook for the current differential frontier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontierRemediationPlaybook {
    /// Human summary.
    pub summary: String,
    /// Owner/domain hint.
    pub owner_hint: String,
    /// Next commands.
    pub next_commands: Vec<String>,
}

/// First-failure diagnostic for the current differential frontier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontierFirstFailure {
    /// Root-cause domain (`parser`, `planner`, `vdbe`, `storage`, `harness`, `fixture`).
    pub root_cause_domain: String,
    /// JSON pointer into the differential manifest.
    pub diagnostic_json_pointer: String,
    /// Deterministic one-command replay.
    pub replay_command: String,
    /// Optional minimized reproduction JSON pointer.
    pub minimal_reproduction_json_pointer: Option<String>,
    /// Artifact entries needed to inspect the failure.
    pub artifact_entries: Vec<String>,
    /// Remediation playbook.
    pub remediation_playbook: FrontierRemediationPlaybook,
}

/// Differential frontier input extracted from a manifest artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DifferentialStatusInput {
    /// Run identifier.
    pub run_id: String,
    /// Trace identifier.
    pub trace_id: String,
    /// Scenario identifier.
    pub scenario_id: String,
    /// Root seed.
    pub root_seed: u64,
    /// Manifest generation timestamp.
    pub generated_unix_ms: u128,
    /// Total differential cases.
    pub total_cases: u64,
    /// Passed differential cases.
    pub passed_cases: u64,
    /// Divergent differential cases.
    pub divergent_cases: u64,
    /// Data hash from the run report.
    pub data_hash: String,
    /// Manifest artifact path.
    pub manifest_path: Option<String>,
    /// Manifest artifact SHA-256.
    pub manifest_sha256: Option<String>,
    /// Deterministic replay command for the run.
    pub replay_command: String,
    /// Sampled passing replay count.
    pub sampled_passing_replay_count: usize,
    /// Current first failure, when the frontier is failing.
    pub first_failure: Option<FrontierFirstFailure>,
}

/// User-facing current frontier summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrentFrontierStatus {
    /// Whether a differential manifest was provided.
    pub present: bool,
    /// Run identifier.
    pub run_id: Option<String>,
    /// Trace identifier.
    pub trace_id: Option<String>,
    /// Scenario identifier.
    pub scenario_id: Option<String>,
    /// Root seed.
    pub root_seed: Option<u64>,
    /// Total differential cases.
    pub total_cases: u64,
    /// Passed differential cases.
    pub passed_cases: u64,
    /// Divergent differential cases.
    pub divergent_cases: u64,
    /// Data hash from the run report.
    pub data_hash: Option<String>,
    /// Deterministic replay command.
    pub replay_command: Option<String>,
    /// Sampled passing replay count.
    pub sampled_passing_replay_count: usize,
    /// Current first failure, when present.
    pub first_failure: Option<FrontierFirstFailure>,
}

/// Divergence ledger row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DivergenceLedgerEntry {
    /// Stable ledger id.
    pub ledger_id: String,
    /// Severity (`info` or `blocking`).
    pub severity: String,
    /// User-facing summary.
    pub summary: String,
    /// Artifact pointer.
    pub artifact_pointer: String,
    /// Replay command.
    pub replay_command: String,
}

/// Machine-readable user parity status report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityStatusReport {
    /// Report schema version.
    pub schema_version: String,
    /// Owning bead identifier.
    pub bead_id: String,
    /// Generation timestamp.
    pub generated_unix_ms: u128,
    /// Target SQLite version.
    pub target_sqlite_version: String,
    /// Weighted parity score from the canonical taxonomy.
    pub score: ParityScore,
    /// Per-feature support state.
    pub features: Vec<FeatureStatusEntry>,
    /// Evidence freshness status.
    pub evidence_freshness: EvidenceFreshnessStatus,
    /// Oracle preflight readiness.
    pub oracle_preflight: OraclePreflightStatus,
    /// Current failing or passing frontier.
    pub current_frontier: CurrentFrontierStatus,
    /// Explicit divergence ledger entries.
    pub divergence_ledger: Vec<DivergenceLedgerEntry>,
    /// Validation violations for report publication.
    pub validation_violations: Vec<ParityStatusViolation>,
    /// Whether the report is complete enough to publish as certifying evidence.
    pub report_complete: bool,
    /// Compact human summary.
    pub summary: String,
}

/// Report validation violation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParityStatusViolation {
    /// Violation code.
    pub code: String,
    /// Human detail.
    pub detail: String,
}

/// Build configuration for the parity status report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParityStatusReportConfig {
    /// Generation timestamp.
    pub generated_unix_ms: u128,
    /// Evidence freshness budget.
    pub freshness_budget_ms: u128,
}

impl Default for ParityStatusReportConfig {
    fn default() -> Self {
        Self {
            generated_unix_ms: 0,
            freshness_budget_ms: DEFAULT_EVIDENCE_FRESHNESS_BUDGET_MS,
        }
    }
}

impl DifferentialStatusInput {
    /// Extract a compact differential frontier from a manifest JSON value.
    pub fn from_manifest_value(
        value: &Value,
        manifest_path: Option<String>,
        manifest_sha256: Option<String>,
    ) -> Result<Self, String> {
        let run_id = required_string(value, "/run_id")?;
        let trace_id = required_string(value, "/trace_id")?;
        let scenario_id = required_string(value, "/scenario_id")?;
        let root_seed = required_u64(value, "/root_seed")?;
        let generated_unix_ms = u128::from(required_u64(value, "/generated_unix_ms")?);
        let total_cases = required_u64(value, "/run_report/total_cases")?;
        let passed_cases = required_u64(value, "/run_report/passed")?;
        let divergent_cases = required_u64(value, "/run_report/diverged")?;
        let data_hash = required_string(value, "/run_report/data_hash")?;
        let replay_command = required_string(value, "/replay/command")?;
        let sampled_passing_replay_count = value
            .pointer("/sampled_passing_replays")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let first_failure = value
            .pointer("/first_failure")
            .filter(|failure| failure.is_object())
            .map(parse_first_failure)
            .transpose()?;

        Ok(Self {
            run_id,
            trace_id,
            scenario_id,
            root_seed,
            generated_unix_ms,
            total_cases,
            passed_cases,
            divergent_cases,
            data_hash,
            manifest_path,
            manifest_sha256,
            replay_command,
            sampled_passing_replay_count,
            first_failure,
        })
    }
}

/// Build a report from canonical sources.
#[must_use]
pub fn generate_canonical_parity_status_report(
    preflight: Option<&OraclePreflightReport>,
    differential: Option<DifferentialStatusInput>,
    config: ParityStatusReportConfig,
) -> ParityStatusReport {
    let universe = build_canonical_universe();
    build_parity_status_report(&universe, preflight, differential, config)
}

/// Build a report from provided sources.
#[must_use]
pub fn build_parity_status_report(
    universe: &FeatureUniverse,
    preflight: Option<&OraclePreflightReport>,
    differential: Option<DifferentialStatusInput>,
    config: ParityStatusReportConfig,
) -> ParityStatusReport {
    let score = universe.compute_score();
    let features = universe
        .sorted_features()
        .into_iter()
        .map(feature_status_entry)
        .collect::<Vec<_>>();
    let oracle_preflight = preflight_status(preflight);
    let current_frontier = current_frontier_status(differential.as_ref());
    let evidence_freshness = evidence_freshness_status(preflight, differential.as_ref(), config);
    let divergence_ledger = divergence_ledger(differential.as_ref());
    let mut report = ParityStatusReport {
        schema_version: PARITY_STATUS_REPORT_SCHEMA_VERSION.to_owned(),
        bead_id: PARITY_STATUS_REPORT_BEAD_ID.to_owned(),
        generated_unix_ms: config.generated_unix_ms,
        target_sqlite_version: universe.target_sqlite_version.clone(),
        score,
        features,
        evidence_freshness,
        oracle_preflight,
        current_frontier,
        divergence_ledger,
        validation_violations: Vec::new(),
        report_complete: false,
        summary: String::new(),
    };

    report.validation_violations = validate_parity_status_report(&report);
    report.report_complete = report.validation_violations.is_empty();
    report.summary = build_summary(&report);
    report
}

/// Validate report completeness for publication.
#[must_use]
pub fn validate_parity_status_report(report: &ParityStatusReport) -> Vec<ParityStatusViolation> {
    let mut violations = Vec::new();
    if report.features.is_empty() {
        violations.push(violation(
            "missing_feature_status",
            "feature status list is empty",
        ));
    }
    if !report.oracle_preflight.present {
        violations.push(violation(
            "missing_oracle_preflight",
            "oracle preflight diagnostics are required",
        ));
    }
    if !report.current_frontier.present {
        violations.push(violation(
            "missing_current_frontier",
            "differential frontier manifest is required",
        ));
    }
    if !report.evidence_freshness.overall_fresh {
        violations.push(violation(
            "stale_or_missing_evidence",
            "one or more required evidence sources are stale or missing",
        ));
    }
    if report
        .current_frontier
        .first_failure
        .as_ref()
        .is_some_and(|failure| failure.remediation_playbook.next_commands.is_empty())
    {
        violations.push(violation(
            "missing_first_failure_remediation",
            "first failure is missing remediation commands",
        ));
    }
    violations
}

/// Render a deterministic Markdown user report.
#[must_use]
pub fn render_parity_status_markdown(report: &ParityStatusReport) -> String {
    let mut output = String::new();
    output.push_str("# FrankenSQLite Parity Status\n\n");
    output.push_str(&format!("bead_id: `{}`\n", report.bead_id));
    output.push_str(&format!("schema_version: `{}`\n", report.schema_version));
    output.push_str(&format!(
        "generated_unix_ms: `{}`\n",
        report.generated_unix_ms
    ));
    output.push_str(&format!(
        "target_sqlite_version: `{}`\n",
        report.target_sqlite_version
    ));
    output.push_str(&format!(
        "report_complete: `{}`\n\n",
        report.report_complete
    ));

    output.push_str("## Score\n\n");
    output.push_str(&format!(
        "- global_score: `{:.6}`\n- passing: `{}`\n- partial: `{}`\n- missing: `{}`\n- excluded: `{}`\n\n",
        report.score.global_score,
        report.score.status_counts.passing,
        report.score.status_counts.partial,
        report.score.status_counts.missing,
        report.score.status_counts.excluded,
    ));

    output.push_str("## Evidence Freshness\n\n");
    for source in &report.evidence_freshness.sources {
        output.push_str(&format!(
            "- `{}`: present=`{}` fresh=`{}` age_ms=`{}` artifact=`{}`\n",
            source.source_id,
            source.present,
            source.fresh,
            source
                .age_ms
                .map_or_else(|| "unknown".to_owned(), |age| age.to_string()),
            source.artifact_path.as_deref().unwrap_or("none"),
        ));
    }
    output.push('\n');

    output.push_str("## Oracle Preflight\n\n");
    output.push_str(&format!(
        "- present: `{}`\n- outcome: `{}`\n- certifying: `{}`\n- fixture_manifest: `{}`\n- replay: `{}`\n\n",
        report.oracle_preflight.present,
        report.oracle_preflight.outcome.as_deref().unwrap_or("missing"),
        report
            .oracle_preflight
            .certifying
            .map_or_else(|| "missing".to_owned(), |value| value.to_string()),
        report
            .oracle_preflight
            .fixture_manifest_path
            .as_deref()
            .unwrap_or("missing"),
        report
            .oracle_preflight
            .replay_command
            .as_deref()
            .unwrap_or("missing"),
    ));
    if let Some(summary) = &report.oracle_preflight.first_failure_summary {
        output.push_str(&format!("- first_failure: `{summary}`\n"));
    }
    if let Some(command) = &report.oracle_preflight.first_failure_fix_command {
        output.push_str(&format!("- first_failure_fix: `{command}`\n"));
    }
    output.push('\n');

    output.push_str("## Current Frontier\n\n");
    output.push_str(&format!(
        "- present: `{}`\n- run_id: `{}`\n- scenario_id: `{}`\n- seed: `{}`\n- total_cases: `{}`\n- divergent_cases: `{}`\n- sampled_passing_replay_count: `{}`\n- replay: `{}`\n\n",
        report.current_frontier.present,
        report.current_frontier.run_id.as_deref().unwrap_or("missing"),
        report
            .current_frontier
            .scenario_id
            .as_deref()
            .unwrap_or("missing"),
        report
            .current_frontier
            .root_seed
            .map_or_else(|| "missing".to_owned(), |seed| seed.to_string()),
        report.current_frontier.total_cases,
        report.current_frontier.divergent_cases,
        report.current_frontier.sampled_passing_replay_count,
        report
            .current_frontier
            .replay_command
            .as_deref()
            .unwrap_or("missing"),
    ));
    if let Some(first_failure) = &report.current_frontier.first_failure {
        output.push_str("### First Failure\n\n");
        output.push_str(&format!(
            "- root_cause_domain: `{}`\n- diagnostic_json_pointer: `{}`\n- replay: `{}`\n- remediation_owner: `{}`\n- remediation_summary: {}\n",
            first_failure.root_cause_domain,
            first_failure.diagnostic_json_pointer,
            first_failure.replay_command,
            first_failure.remediation_playbook.owner_hint,
            first_failure.remediation_playbook.summary,
        ));
        output.push_str("- remediation_commands:\n");
        for command in &first_failure.remediation_playbook.next_commands {
            output.push_str(&format!("  - `{command}`\n"));
        }
        output.push('\n');
    }

    output.push_str("## Divergence Ledger\n\n");
    for entry in &report.divergence_ledger {
        output.push_str(&format!(
            "- `{}` severity=`{}` artifact=`{}` replay=`{}`: {}\n",
            entry.ledger_id,
            entry.severity,
            entry.artifact_pointer,
            entry.replay_command,
            entry.summary,
        ));
    }
    output.push('\n');

    output.push_str("## Residual Gaps\n\n");
    let mut gap_count = 0_usize;
    for feature in report
        .features
        .iter()
        .filter(|feature| feature.support_state != "passing" && feature.support_state != "excluded")
        .take(20)
    {
        gap_count += 1;
        output.push_str(&format!(
            "- `{}` {} [{}]: {}\n",
            feature.feature_id, feature.title, feature.support_state, feature.residual_gap,
        ));
    }
    if gap_count == 0 {
        output.push_str("- none\n");
    }
    output.push('\n');

    output.push_str("## Validation\n\n");
    if report.validation_violations.is_empty() {
        output.push_str("- no validation violations\n");
    } else {
        for violation in &report.validation_violations {
            output.push_str(&format!("- `{}`: {}\n", violation.code, violation.detail));
        }
    }
    output
}

fn parse_first_failure(value: &Value) -> Result<FrontierFirstFailure, String> {
    let root_cause_domain = required_string(value, "/root_cause_domain")?;
    let diagnostic_json_pointer = required_string(value, "/diagnostic_json_pointer")?;
    let replay_command = required_string(value, "/replay_command")?;
    let minimal_reproduction_json_pointer =
        optional_string(value, "/minimal_reproduction_json_pointer");
    let artifact_entries = string_array(value, "/artifact_entries")?;
    let remediation = value
        .pointer("/remediation_playbook")
        .ok_or_else(|| "missing required field /first_failure/remediation_playbook".to_owned())?;
    let remediation_playbook = FrontierRemediationPlaybook {
        summary: required_string(remediation, "/summary")?,
        owner_hint: required_string(remediation, "/owner_hint")?,
        next_commands: string_array(remediation, "/next_commands")?,
    };

    Ok(FrontierFirstFailure {
        root_cause_domain,
        diagnostic_json_pointer,
        replay_command,
        minimal_reproduction_json_pointer,
        artifact_entries,
        remediation_playbook,
    })
}

fn required_string(value: &Value, pointer: &str) -> Result<String, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("missing required string field {pointer}"))
}

fn optional_string(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn required_u64(value: &Value, pointer: &str) -> Result<u64, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing required u64 field {pointer}"))
}

fn string_array(value: &Value, pointer: &str) -> Result<Vec<String>, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing required string array field {pointer}"))?
        .iter()
        .map(|item| {
            item.as_str()
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("invalid string array item in {pointer}"))
        })
        .collect()
}

fn feature_status_entry(feature: &Feature) -> FeatureStatusEntry {
    let support_state = feature.status.to_string();
    FeatureStatusEntry {
        feature_id: feature.id.to_string(),
        title: feature.title.clone(),
        category: feature.category.display_name().to_owned(),
        support_state: support_state.clone(),
        residual_gap: residual_gap(feature.status),
        evidence_beads: feature.observability.bead_ids.clone(),
        replay_pointers: replay_pointers(feature),
    }
}

fn residual_gap(status: ParityStatus) -> String {
    match status {
        ParityStatus::Passing => "No residual gap declared in the canonical taxonomy.".to_owned(),
        ParityStatus::Partial => {
            "Partially supported; inspect linked replay pointers and current frontier diagnostics."
                .to_owned()
        }
        ParityStatus::Missing => {
            "Not yet supported; this feature remains in the user-visible divergence ledger."
                .to_owned()
        }
        ParityStatus::Excluded => "Excluded from parity scoring by documented policy.".to_owned(),
    }
}

fn replay_pointers(feature: &Feature) -> Vec<String> {
    let mut pointers = Vec::new();
    for fixture_id in &feature.observability.fixture_ids {
        pointers.push(format!(
            "cargo run -p fsqlite-harness --bin differential_manifest_runner -- --fixture-id {fixture_id}"
        ));
    }
    for module in &feature.observability.test_modules {
        pointers.push(format!("cargo test -p fsqlite-harness {module}"));
    }
    for bead_id in &feature.observability.bead_ids {
        pointers.push(format!("br show {bead_id} --json"));
    }
    if pointers.is_empty() {
        pointers.push(format!(
            "cargo run -p fsqlite-harness --bin parity_status_report_runner -- --feature-id {}",
            feature.id
        ));
    }
    pointers
}

fn preflight_status(preflight: Option<&OraclePreflightReport>) -> OraclePreflightStatus {
    preflight.map_or(
        OraclePreflightStatus {
            present: false,
            outcome: None,
            certifying: None,
            first_failure_summary: None,
            first_failure_fix_command: None,
            fixture_manifest_path: None,
            fixture_manifest_sha256: None,
            replay_command: None,
        },
        |report| OraclePreflightStatus {
            present: true,
            outcome: Some(report.outcome.to_string()),
            certifying: Some(report.certifying),
            first_failure_summary: report
                .first_failure
                .as_ref()
                .map(|failure| failure.summary.clone()),
            first_failure_fix_command: report
                .first_failure
                .as_ref()
                .map(|failure| failure.fix_command.clone()),
            fixture_manifest_path: Some(report.checks.fixture_manifest_path.clone()),
            fixture_manifest_sha256: report.checks.fixture_manifest_sha256.clone(),
            replay_command: Some(report.replay_command.clone()),
        },
    )
}

fn current_frontier_status(
    differential: Option<&DifferentialStatusInput>,
) -> CurrentFrontierStatus {
    differential.map_or(
        CurrentFrontierStatus {
            present: false,
            run_id: None,
            trace_id: None,
            scenario_id: None,
            root_seed: None,
            total_cases: 0,
            passed_cases: 0,
            divergent_cases: 0,
            data_hash: None,
            replay_command: None,
            sampled_passing_replay_count: 0,
            first_failure: None,
        },
        |input| CurrentFrontierStatus {
            present: true,
            run_id: Some(input.run_id.clone()),
            trace_id: Some(input.trace_id.clone()),
            scenario_id: Some(input.scenario_id.clone()),
            root_seed: Some(input.root_seed),
            total_cases: input.total_cases,
            passed_cases: input.passed_cases,
            divergent_cases: input.divergent_cases,
            data_hash: Some(input.data_hash.clone()),
            replay_command: Some(input.replay_command.clone()),
            sampled_passing_replay_count: input.sampled_passing_replay_count,
            first_failure: input.first_failure.clone(),
        },
    )
}

fn evidence_freshness_status(
    preflight: Option<&OraclePreflightReport>,
    differential: Option<&DifferentialStatusInput>,
    config: ParityStatusReportConfig,
) -> EvidenceFreshnessStatus {
    let sources = vec![
        EvidenceFreshnessSource {
            source_id: "parity_taxonomy".to_owned(),
            present: true,
            observed_unix_ms: Some(config.generated_unix_ms),
            age_ms: Some(0),
            fresh: true,
            artifact_path: Some("parity_taxonomy.toml".to_owned()),
            artifact_sha256: None,
            replay_command: Some(
                "cargo run -p fsqlite-harness --bin parity_status_report_runner".to_owned(),
            ),
        },
        freshness_source_from_preflight(preflight, config),
        freshness_source_from_differential(differential, config),
    ];
    let overall_fresh = sources.iter().all(|source| source.present && source.fresh);
    EvidenceFreshnessStatus {
        freshness_budget_ms: config.freshness_budget_ms,
        overall_fresh,
        sources,
    }
}

fn freshness_source_from_preflight(
    preflight: Option<&OraclePreflightReport>,
    config: ParityStatusReportConfig,
) -> EvidenceFreshnessSource {
    preflight.map_or_else(
        || EvidenceFreshnessSource {
            source_id: "oracle_preflight_doctor".to_owned(),
            present: false,
            observed_unix_ms: None,
            age_ms: None,
            fresh: false,
            artifact_path: None,
            artifact_sha256: None,
            replay_command: None,
        },
        |report| {
            let age_ms = config
                .generated_unix_ms
                .saturating_sub(report.generated_unix_ms);
            EvidenceFreshnessSource {
                source_id: "oracle_preflight_doctor".to_owned(),
                present: true,
                observed_unix_ms: Some(report.generated_unix_ms),
                age_ms: Some(age_ms),
                fresh: age_ms <= config.freshness_budget_ms,
                artifact_path: None,
                artifact_sha256: report.checks.fixture_manifest_sha256.clone(),
                replay_command: Some(report.replay_command.clone()),
            }
        },
    )
}

fn freshness_source_from_differential(
    differential: Option<&DifferentialStatusInput>,
    config: ParityStatusReportConfig,
) -> EvidenceFreshnessSource {
    differential.map_or_else(
        || EvidenceFreshnessSource {
            source_id: "differential_manifest".to_owned(),
            present: false,
            observed_unix_ms: None,
            age_ms: None,
            fresh: false,
            artifact_path: None,
            artifact_sha256: None,
            replay_command: None,
        },
        |input| {
            let age_ms = config
                .generated_unix_ms
                .saturating_sub(input.generated_unix_ms);
            EvidenceFreshnessSource {
                source_id: "differential_manifest".to_owned(),
                present: true,
                observed_unix_ms: Some(input.generated_unix_ms),
                age_ms: Some(age_ms),
                fresh: age_ms <= config.freshness_budget_ms,
                artifact_path: input.manifest_path.clone(),
                artifact_sha256: input.manifest_sha256.clone(),
                replay_command: Some(input.replay_command.clone()),
            }
        },
    )
}

fn divergence_ledger(differential: Option<&DifferentialStatusInput>) -> Vec<DivergenceLedgerEntry> {
    let Some(input) = differential else {
        return vec![DivergenceLedgerEntry {
            ledger_id: "missing-differential-frontier".to_owned(),
            severity: "blocking".to_owned(),
            summary: "No differential manifest was provided.".to_owned(),
            artifact_pointer: "missing".to_owned(),
            replay_command: "cargo run -p fsqlite-harness --bin differential_manifest_runner"
                .to_owned(),
        }];
    };

    if input.divergent_cases == 0 {
        return vec![DivergenceLedgerEntry {
            ledger_id: format!("{}-clean-frontier", input.run_id),
            severity: "info".to_owned(),
            summary: format!(
                "No divergent cases in current frontier (total_cases={}).",
                input.total_cases
            ),
            artifact_pointer: input
                .manifest_path
                .clone()
                .unwrap_or_else(|| "differential_manifest.json".to_owned()),
            replay_command: input.replay_command.clone(),
        }];
    }

    let first_failure_summary = input.first_failure.as_ref().map_or_else(
        || "Divergence present but first-failure details are missing.".to_owned(),
        |failure| {
            format!(
                "{} divergence at {}",
                failure.root_cause_domain, failure.diagnostic_json_pointer
            )
        },
    );
    vec![DivergenceLedgerEntry {
        ledger_id: format!("{}-first-divergence", input.run_id),
        severity: "blocking".to_owned(),
        summary: first_failure_summary,
        artifact_pointer: input
            .manifest_path
            .clone()
            .unwrap_or_else(|| "differential_manifest.json".to_owned()),
        replay_command: input.replay_command.clone(),
    }]
}

fn build_summary(report: &ParityStatusReport) -> String {
    format!(
        "Parity status report: complete={} score={:.6} features={} preflight={} frontier_present={} divergences={} freshness={}",
        report.report_complete,
        report.score.global_score,
        report.features.len(),
        report
            .oracle_preflight
            .outcome
            .as_deref()
            .unwrap_or("missing"),
        report.current_frontier.present,
        report.current_frontier.divergent_cases,
        report.evidence_freshness.overall_fresh,
    )
}

fn violation(code: &str, detail: &str) -> ParityStatusViolation {
    ParityStatusViolation {
        code: code.to_owned(),
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle_preflight_doctor::{
        DoctorOutcome, FirstFailureDiagnosis, OraclePreflightChecks, RemediationClass,
    };

    fn config() -> ParityStatusReportConfig {
        ParityStatusReportConfig {
            generated_unix_ms: 1_700_000_000_000,
            freshness_budget_ms: 1_000,
        }
    }

    fn preflight(generated_unix_ms: u128) -> OraclePreflightReport {
        OraclePreflightReport {
            schema_version: "1.0.0".to_owned(),
            bead_id: "bd-2yqp6.2.5".to_owned(),
            run_id: "doctor-run".to_owned(),
            trace_id: "trace-doctor".to_owned(),
            scenario_id: "doctor-scenario".to_owned(),
            seed: 42,
            generated_unix_ms,
            outcome: DoctorOutcome::Green,
            certifying: true,
            timing_ms: 12,
            first_failure: None,
            findings: Vec::new(),
            checks: OraclePreflightChecks {
                expected_subject_identity: "frankensqlite".to_owned(),
                expected_reference_identity: "csqlite-oracle".to_owned(),
                expected_sqlite_version_prefix: "3.52.0".to_owned(),
                fixtures_dir: "fixtures".to_owned(),
                fixture_manifest_path: "docs/contracts/corpus_manifest.toml".to_owned(),
                oracle_binary_path: Some("sqlite3".to_owned()),
                oracle_version: Some("3.52.0-test".to_owned()),
                fixture_json_files_seen: 1,
                fixture_entries_ingested: 1,
                fixture_sql_statements_ingested: 2,
                skipped_fixture_files: 0,
                fixture_manifest_mtime_unix_ms: Some(generated_unix_ms),
                fixture_manifest_sha256: Some("a".repeat(64)),
                latest_fixture_mtime_unix_ms: Some(generated_unix_ms),
            },
            replay_command: "cargo run -p fsqlite-harness --bin oracle_preflight_doctor_runner"
                .to_owned(),
        }
    }

    fn failing_preflight(generated_unix_ms: u128) -> OraclePreflightReport {
        let mut report = preflight(generated_unix_ms);
        report.outcome = DoctorOutcome::Red;
        report.certifying = false;
        report.first_failure = Some(FirstFailureDiagnosis {
            remediation_class: RemediationClass::MissingBinary,
            summary: "sqlite3 oracle missing".to_owned(),
            fix_command: "sudo apt-get install -y sqlite3".to_owned(),
        });
        report
    }

    fn differential(generated_unix_ms: u128) -> DifferentialStatusInput {
        DifferentialStatusInput {
            run_id: "diff-run".to_owned(),
            trace_id: "trace-diff".to_owned(),
            scenario_id: "diff-scenario".to_owned(),
            root_seed: 4242,
            generated_unix_ms,
            total_cases: 3,
            passed_cases: 2,
            divergent_cases: 1,
            data_hash: "data-hash".to_owned(),
            manifest_path: Some("artifacts/differential_manifest.json".to_owned()),
            manifest_sha256: Some("b".repeat(64)),
            replay_command: "cargo run -p fsqlite-harness --bin differential_manifest_runner"
                .to_owned(),
            sampled_passing_replay_count: 1,
            first_failure: Some(FrontierFirstFailure {
                root_cause_domain: "planner".to_owned(),
                diagnostic_json_pointer: "/run_report/divergent_cases/0".to_owned(),
                replay_command: "cargo run -p fsqlite-harness --bin differential_manifest_runner"
                    .to_owned(),
                minimal_reproduction_json_pointer: None,
                artifact_entries: vec!["differential_manifest_json".to_owned()],
                remediation_playbook: FrontierRemediationPlaybook {
                    summary: "inspect planner".to_owned(),
                    owner_hint: "planner owners".to_owned(),
                    next_commands: vec!["cargo test -p fsqlite-planner".to_owned()],
                },
            }),
        }
    }

    #[test]
    fn complete_report_includes_feature_status_and_frontier() {
        let preflight = preflight(config().generated_unix_ms);
        let report = generate_canonical_parity_status_report(
            Some(&preflight),
            Some(differential(config().generated_unix_ms)),
            config(),
        );

        assert!(report.report_complete);
        assert_eq!(report.bead_id, PARITY_STATUS_REPORT_BEAD_ID);
        assert!(!report.features.is_empty());
        assert!(report.evidence_freshness.overall_fresh);
        assert_eq!(report.oracle_preflight.outcome.as_deref(), Some("green"));
        assert_eq!(report.current_frontier.divergent_cases, 1);
        assert!(!report.divergence_ledger.is_empty());
    }

    #[test]
    fn missing_preflight_and_frontier_are_publication_violations() {
        let report = generate_canonical_parity_status_report(None, None, config());

        assert!(!report.report_complete);
        assert!(
            report
                .validation_violations
                .iter()
                .any(|violation| violation.code == "missing_oracle_preflight")
        );
        assert!(
            report
                .validation_violations
                .iter()
                .any(|violation| violation.code == "missing_current_frontier")
        );
    }

    #[test]
    fn stale_evidence_fails_publication() {
        let preflight = preflight(config().generated_unix_ms - 2_000);
        let report = generate_canonical_parity_status_report(
            Some(&preflight),
            Some(differential(config().generated_unix_ms)),
            config(),
        );

        assert!(!report.report_complete);
        assert!(
            report
                .validation_violations
                .iter()
                .any(|violation| violation.code == "stale_or_missing_evidence")
        );
    }

    #[test]
    fn markdown_surfaces_preflight_and_first_failure_playbook() {
        let preflight = failing_preflight(config().generated_unix_ms);
        let report = generate_canonical_parity_status_report(
            Some(&preflight),
            Some(differential(config().generated_unix_ms)),
            config(),
        );
        let markdown = render_parity_status_markdown(&report);

        assert!(markdown.contains("## Oracle Preflight"));
        assert!(markdown.contains("sqlite3 oracle missing"));
        assert!(markdown.contains("## Current Frontier"));
        assert!(markdown.contains("root_cause_domain: `planner`"));
        assert!(markdown.contains("cargo test -p fsqlite-planner"));
    }

    #[test]
    fn differential_manifest_parser_extracts_first_failure() {
        let payload = serde_json::json!({
            "run_id": "run-1",
            "trace_id": "trace-1",
            "scenario_id": "scenario-1",
            "root_seed": 7,
            "generated_unix_ms": 1700000000000_u64,
            "run_report": {
                "total_cases": 2,
                "passed": 1,
                "diverged": 1,
                "data_hash": "abc"
            },
            "replay": { "command": "cargo run -p fsqlite-harness --bin differential_manifest_runner" },
            "sampled_passing_replays": [
                { "case_id": "case", "replay_command": "cmd" }
            ],
            "first_failure": {
                "root_cause_domain": "storage",
                "diagnostic_json_pointer": "/run_report/divergent_cases/0",
                "replay_command": "cargo run -p fsqlite-harness --bin differential_manifest_runner",
                "minimal_reproduction_json_pointer": null,
                "artifact_entries": ["differential_manifest_json"],
                "remediation_playbook": {
                    "summary": "inspect storage",
                    "owner_hint": "storage owners",
                    "next_commands": ["cargo test -p fsqlite-wal"]
                }
            }
        });

        let parsed = DifferentialStatusInput::from_manifest_value(
            &payload,
            Some("manifest.json".to_owned()),
            Some("c".repeat(64)),
        )
        .expect("parse manifest");

        assert_eq!(parsed.run_id, "run-1");
        assert_eq!(parsed.sampled_passing_replay_count, 1);
        assert_eq!(
            parsed
                .first_failure
                .expect("first failure")
                .root_cause_domain,
            "storage"
        );
    }
}
