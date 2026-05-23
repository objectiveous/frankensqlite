//! Agent-swarm SQL workload trace schema and privacy scrubber.
//!
//! The replay lab records the shape of concurrent agent activity without
//! storing user data. This module owns the canonical JSON schema for those
//! traces plus the deterministic scrubber used before writing fixtures or
//! replay bundles.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Bead that introduced the agent-swarm trace contract.
#[allow(dead_code)]
const BEAD_ID: &str = "bd-agent-swarm-replay-lab-1cc5y.1";

/// Schema version for serialized agent-swarm traces.
pub const SWARM_TRACE_SCHEMA_VERSION: &str = "1.0.0";

/// Version of the privacy scrubber contract.
pub const SWARM_TRACE_SCRUBBER_VERSION: &str = "1.0.0";

/// Version of the fixed-seed synthetic trace generator.
pub const SYNTHETIC_TRACE_GENERATOR_VERSION: &str = "1.0.0";

/// Marker used in structured logs when no failure diagnostic exists yet.
pub const FIRST_FAILURE_DIAGNOSTIC_ABSENT: &str = "none";

/// Complete sanitized trace for an agent-swarm workload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSwarmTrace {
    /// Version of this serialized schema.
    pub schema_version: String,
    /// Stable trace identifier used across artifacts.
    pub trace_id: String,
    /// Replay or capture run identifier.
    pub run_id: String,
    /// Scenario identifier shared with logs and future replay harnesses.
    pub scenario_id: String,
    /// Version of the scrubber that produced the sanitized statements.
    pub scrubber_version: String,
    /// Sanitized provenance for the trace as a whole.
    pub metadata: TraceMetadata,
    /// Sanitized statements in deterministic replay order.
    pub statements: Vec<TraceStatement>,
    /// Aggregate redaction counts for quick audit checks.
    pub redaction_summary: RedactionSummary,
}

impl AgentSwarmTrace {
    /// Build a trace and compute aggregate redaction metadata.
    pub fn new(
        trace_id: impl Into<String>,
        run_id: impl Into<String>,
        scenario_id: impl Into<String>,
        metadata: TraceMetadata,
        statements: Vec<TraceStatement>,
    ) -> Result<Self, TraceScrubError> {
        let trace_id = required_owned("trace_id", trace_id.into())?;
        let run_id = required_owned("run_id", run_id.into())?;
        let scenario_id = required_owned("scenario_id", scenario_id.into())?;
        if statements.is_empty() {
            return Err(TraceScrubError::EmptyStatements);
        }
        let redaction_summary = RedactionSummary::from_statements(&statements);
        Ok(Self {
            schema_version: SWARM_TRACE_SCHEMA_VERSION.to_owned(),
            trace_id,
            run_id,
            scenario_id,
            scrubber_version: SWARM_TRACE_SCRUBBER_VERSION.to_owned(),
            metadata,
            statements,
            redaction_summary,
        })
    }

    /// Number of statements in the trace.
    pub fn statement_count(&self) -> usize {
        self.statements.len()
    }

    /// Number of distinct transaction identifiers preserved in the trace.
    pub fn transaction_count(&self) -> usize {
        self.statements
            .iter()
            .filter_map(|statement| statement.transaction_id.as_deref())
            .collect::<BTreeSet<_>>()
            .len()
    }
}

/// Sanitized provenance for an agent-swarm trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceMetadata {
    /// High-level source category, for example `agent-mail` or `cass`.
    pub source_kind: String,
    /// Sanitized source identifier suitable for artifact references.
    pub source_id: String,
    /// Digest of the raw source before scrubbing, when available.
    pub source_digest: Option<String>,
    /// Logical capture clock or deterministic source ordering label.
    pub logical_clock: Option<String>,
    /// Extra non-sensitive provenance tags.
    pub tags: BTreeMap<String, String>,
}

impl TraceMetadata {
    /// Create minimal trace metadata.
    pub fn new(
        source_kind: impl Into<String>,
        source_id: impl Into<String>,
    ) -> Result<Self, TraceScrubError> {
        Ok(Self {
            source_kind: required_owned("source_kind", source_kind.into())?,
            source_id: required_owned("source_id", source_id.into())?,
            source_digest: None,
            logical_clock: None,
            tags: BTreeMap::new(),
        })
    }
}

/// Sanitized provenance for a single statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatementSource {
    /// Source category, for example `agent-mail-message` or `bead-comment`.
    pub source_kind: String,
    /// Sanitized reference within the source category.
    pub source_ref: String,
    /// Optional digest of the raw source record.
    pub source_digest: Option<String>,
}

impl StatementSource {
    /// Create statement-level provenance.
    pub fn new(
        source_kind: impl Into<String>,
        source_ref: impl Into<String>,
    ) -> Result<Self, TraceScrubError> {
        Ok(Self {
            source_kind: required_owned("source_kind", source_kind.into())?,
            source_ref: required_owned("source_ref", source_ref.into())?,
            source_digest: None,
        })
    }
}

/// Raw statement data before SQL scrubbing.
#[derive(Debug, Clone)]
pub struct RawTraceStatement<'a> {
    /// Deterministic replay order within the trace.
    pub logical_order: u64,
    /// Optional logical timestamp from the capture source.
    pub logical_timestamp: Option<&'a str>,
    /// Logical actor that issued the statement.
    pub actor_id: &'a str,
    /// Logical connection used by the actor.
    pub connection_id: &'a str,
    /// Transaction identifier, when the statement belongs to a transaction.
    pub transaction_id: Option<&'a str>,
    /// Transaction boundary marker for this statement.
    pub transaction_boundary: TransactionBoundary,
    /// Group of statements that may overlap concurrently.
    pub concurrency_group: &'a str,
    /// Workload phase, for example `setup`, `hot-write`, or `verify`.
    pub workload_phase: &'a str,
    /// Raw SQL statement text.
    pub sql: &'a str,
    /// Expected high-level outcome class.
    pub expected_result_class: ExpectedResultClass,
    /// Expected row-count bucket, when known.
    pub row_count_class: RowCountClass,
    /// Error class, when the expected outcome is an error-like result.
    pub error_class: Option<&'a str>,
    /// Sanitized source metadata.
    pub source: StatementSource,
}

/// Sanitized trace statement ready for serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceStatement {
    /// Deterministic replay order within the trace.
    pub logical_order: u64,
    /// Optional logical timestamp from the capture source.
    pub logical_timestamp: Option<String>,
    /// Logical actor that issued the statement.
    pub actor_id: String,
    /// Logical connection used by the actor.
    pub connection_id: String,
    /// Transaction identifier, when present.
    pub transaction_id: Option<String>,
    /// Transaction boundary marker.
    pub transaction_boundary: TransactionBoundary,
    /// Group of statements that may overlap concurrently.
    pub concurrency_group: String,
    /// Workload phase, for example `setup`, `hot-write`, or `verify`.
    pub workload_phase: String,
    /// SQL after privacy-preserving literal redaction.
    pub scrubbed_sql: String,
    /// SHA-256 hash of `scrubbed_sql`, encoded as lowercase hex.
    pub statement_shape: String,
    /// Number of bind parameters in the original SQL shape.
    pub parameter_count: usize,
    /// Redactions performed while producing `scrubbed_sql`.
    pub redactions: RedactionCounts,
    /// Expected high-level outcome class.
    pub expected_result_class: ExpectedResultClass,
    /// Expected row-count bucket, when known.
    pub row_count_class: RowCountClass,
    /// Error class, when known.
    pub error_class: Option<String>,
    /// Sanitized statement source metadata.
    pub source: StatementSource,
}

impl TraceStatement {
    /// Scrub a raw statement and validate required topology fields.
    pub fn from_raw(raw: RawTraceStatement<'_>) -> Result<Self, TraceScrubError> {
        if raw.sql.trim().is_empty() {
            return Err(TraceScrubError::EmptySql);
        }
        if raw.transaction_boundary != TransactionBoundary::None && raw.transaction_id.is_none() {
            return Err(TraceScrubError::BoundaryWithoutTransactionId);
        }
        let scrubbed = scrub_sql_statement(raw.sql);
        Ok(Self {
            logical_order: raw.logical_order,
            logical_timestamp: raw.logical_timestamp.map(ToOwned::to_owned),
            actor_id: required_ref("actor_id", raw.actor_id)?.to_owned(),
            connection_id: required_ref("connection_id", raw.connection_id)?.to_owned(),
            transaction_id: raw.transaction_id.map(ToOwned::to_owned),
            transaction_boundary: raw.transaction_boundary,
            concurrency_group: required_ref("concurrency_group", raw.concurrency_group)?.to_owned(),
            workload_phase: required_ref("workload_phase", raw.workload_phase)?.to_owned(),
            scrubbed_sql: scrubbed.sql,
            statement_shape: scrubbed.statement_shape,
            parameter_count: scrubbed.parameter_count,
            redactions: scrubbed.redactions,
            expected_result_class: raw.expected_result_class,
            row_count_class: raw.row_count_class,
            error_class: raw.error_class.map(ToOwned::to_owned),
            source: raw.source,
        })
    }
}

/// Transaction boundary marker preserved for replay topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionBoundary {
    /// The statement is not itself a transaction boundary.
    None,
    /// Transaction begin marker.
    Begin,
    /// Transaction commit marker.
    Commit,
    /// Transaction rollback marker.
    Rollback,
}

/// High-level expected statement outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedResultClass {
    /// Statement is expected to succeed.
    Success,
    /// Statement is expected to hit a busy/locked condition.
    Busy,
    /// Statement is expected to hit a page-level conflict.
    Conflict,
    /// Statement is expected to fail with an error.
    Error,
    /// Expected result is not known yet.
    Unknown,
}

/// Coarse row-count bucket for replay comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowCountClass {
    /// Statement returns or affects no rows.
    Zero,
    /// Statement returns or affects one row.
    One,
    /// Statement returns or affects a small bounded count.
    Few,
    /// Statement returns or affects many rows.
    Many,
    /// Row-count behavior is not known yet.
    Unknown,
}

/// Result of scrubbing one SQL statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrubbedSql {
    /// Sanitized SQL text.
    pub sql: String,
    /// SHA-256 hash of the sanitized SQL shape.
    pub statement_shape: String,
    /// Number of bind parameters preserved in SQL shape.
    pub parameter_count: usize,
    /// Redaction counts by literal class.
    pub redactions: RedactionCounts,
}

/// Redaction counts for a single statement.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionCounts {
    /// Single-quoted string literals replaced with `?s`.
    pub string_literals: usize,
    /// Numeric literals replaced with `?n`.
    pub numeric_literals: usize,
    /// SQLite blob literals replaced with `?b`.
    pub blob_literals: usize,
    /// SQL comments replaced with comment markers.
    pub comments: usize,
}

impl RedactionCounts {
    /// Total number of redacted spans.
    pub const fn total(self) -> usize {
        self.string_literals + self.numeric_literals + self.blob_literals + self.comments
    }
}

/// Aggregate redaction summary for a trace.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionSummary {
    /// Number of statements included in the trace.
    pub statement_count: usize,
    /// Number of distinct transactions represented in the trace.
    pub transaction_count: usize,
    /// Total redacted spans across all statements.
    pub redaction_count: usize,
    /// Single-quoted string literal redactions.
    pub string_literals: usize,
    /// Numeric literal redactions.
    pub numeric_literals: usize,
    /// SQLite blob literal redactions.
    pub blob_literals: usize,
    /// SQL comment redactions.
    pub comments: usize,
}

impl RedactionSummary {
    /// Summarize statements after scrubbing.
    pub fn from_statements(statements: &[TraceStatement]) -> Self {
        let mut summary = Self {
            statement_count: statements.len(),
            transaction_count: statements
                .iter()
                .filter_map(|statement| statement.transaction_id.as_deref())
                .collect::<BTreeSet<_>>()
                .len(),
            ..Self::default()
        };
        for statement in statements {
            summary.string_literals += statement.redactions.string_literals;
            summary.numeric_literals += statement.redactions.numeric_literals;
            summary.blob_literals += statement.redactions.blob_literals;
            summary.comments += statement.redactions.comments;
        }
        summary.redaction_count = summary.string_literals
            + summary.numeric_literals
            + summary.blob_literals
            + summary.comments;
        summary
    }
}

/// Workload family used by the deterministic synthetic trace generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticTraceScenario {
    /// Agents append session events and tool-call logs.
    SessionEventAppend,
    /// Agents repeatedly claim, update, and complete task-queue rows.
    TaskQueueClaimLoop,
    /// Artifact index writes are mixed with read-heavy metadata lookups.
    ArtifactIndexMixedLookup,
    /// Many agents update shared counters and status rows.
    HotCounterStatusRows,
    /// Long readers overlap with bursty writers.
    LongReaderBurstWriter,
}

impl SyntheticTraceScenario {
    /// Stable scenario identifier used in trace and log metadata.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionEventAppend => "session_event_append",
            Self::TaskQueueClaimLoop => "task_queue_claim_loop",
            Self::ArtifactIndexMixedLookup => "artifact_index_mixed_lookup",
            Self::HotCounterStatusRows => "hot_counter_status_rows",
            Self::LongReaderBurstWriter => "long_reader_burst_writer",
        }
    }

    fn expected_contention_shape(self, hot_key_ratio: u8) -> String {
        let skew = match hot_key_ratio {
            0..=24 => "low_skew",
            25..=74 => "mixed_skew",
            _ => "high_skew",
        };
        let base = match self {
            Self::SessionEventAppend => "append_heavy_session_streams",
            Self::TaskQueueClaimLoop => "claim_update_status_rows",
            Self::ArtifactIndexMixedLookup => "metadata_read_write_mix",
            Self::HotCounterStatusRows => "shared_counter_hotspots",
            Self::LongReaderBurstWriter => "long_reader_writer_overlap",
        };
        format!("{base}:{skew}")
    }

    fn target_invariants(self) -> &'static str {
        match self {
            Self::SessionEventAppend => {
                "schema_conformance,deterministic_seed,transaction_boundaries,append_order_shape"
            }
            Self::TaskQueueClaimLoop => {
                "schema_conformance,deterministic_seed,transaction_boundaries,no_double_claim_shape"
            }
            Self::ArtifactIndexMixedLookup => {
                "schema_conformance,deterministic_seed,transaction_boundaries,read_write_shape"
            }
            Self::HotCounterStatusRows => {
                "schema_conformance,deterministic_seed,transaction_boundaries,hotspot_conflict_shape"
            }
            Self::LongReaderBurstWriter => {
                "schema_conformance,deterministic_seed,transaction_boundaries,mvcc_visibility_shape"
            }
        }
    }

    fn phase(self, is_read: bool) -> &'static str {
        match (self, is_read) {
            (Self::SessionEventAppend, true) => "session-read",
            (Self::SessionEventAppend, false) => "session-append",
            (Self::TaskQueueClaimLoop, true) => "queue-peek",
            (Self::TaskQueueClaimLoop, false) => "queue-claim",
            (Self::ArtifactIndexMixedLookup, true) => "artifact-lookup",
            (Self::ArtifactIndexMixedLookup, false) => "artifact-index-write",
            (Self::HotCounterStatusRows, true) => "counter-read",
            (Self::HotCounterStatusRows, false) => "counter-update",
            (Self::LongReaderBurstWriter, true) => "long-read",
            (Self::LongReaderBurstWriter, false) => "bursty-write",
        }
    }
}

/// All synthetic scenario families currently emitted by the replay lab.
pub const SYNTHETIC_TRACE_SCENARIOS: [SyntheticTraceScenario; 5] = [
    SyntheticTraceScenario::SessionEventAppend,
    SyntheticTraceScenario::TaskQueueClaimLoop,
    SyntheticTraceScenario::ArtifactIndexMixedLookup,
    SyntheticTraceScenario::HotCounterStatusRows,
    SyntheticTraceScenario::LongReaderBurstWriter,
];

/// Configuration for deterministic synthetic agent-swarm trace generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntheticTraceConfig {
    /// Workload family to synthesize.
    pub scenario: SyntheticTraceScenario,
    /// Fixed seed for deterministic generation.
    pub seed: u64,
    /// Number of logical agents represented in the trace.
    pub agent_count: usize,
    /// Number of transactions to emit.
    pub transaction_count: usize,
    /// Relative read weight in the transaction mix.
    pub read_weight: u16,
    /// Relative write weight in the transaction mix.
    pub write_weight: u16,
    /// Percentage of transactions routed to hot keys, from 0 to 100.
    pub hot_key_ratio: u8,
}

impl SyntheticTraceConfig {
    /// Create a synthetic trace configuration.
    pub const fn new(
        scenario: SyntheticTraceScenario,
        seed: u64,
        agent_count: usize,
        transaction_count: usize,
    ) -> Self {
        Self {
            scenario,
            seed,
            agent_count,
            transaction_count,
            read_weight: 40,
            write_weight: 60,
            hot_key_ratio: 70,
        }
    }

    /// Override the read/write transaction mix.
    pub const fn with_read_write_mix(mut self, read_weight: u16, write_weight: u16) -> Self {
        self.read_weight = read_weight;
        self.write_weight = write_weight;
        self
    }

    /// Override the hot-key skew percentage.
    pub const fn with_hot_key_ratio(mut self, hot_key_ratio: u8) -> Self {
        self.hot_key_ratio = hot_key_ratio;
        self
    }

    /// Stable read/write mix label used in metadata and logs.
    pub fn read_write_mix(&self) -> String {
        format!("read:{}:write:{}", self.read_weight, self.write_weight)
    }

    fn validate(&self) -> Result<(), SyntheticTraceError> {
        if self.agent_count == 0 {
            return Err(SyntheticTraceError::AgentCountZero);
        }
        if self.transaction_count == 0 {
            return Err(SyntheticTraceError::TransactionCountZero);
        }
        if self.read_write_total() == 0 {
            return Err(SyntheticTraceError::EmptyReadWriteMix);
        }
        if self.hot_key_ratio > 100 {
            return Err(SyntheticTraceError::HotKeyRatioOutOfRange(
                self.hot_key_ratio,
            ));
        }
        Ok(())
    }

    fn read_write_total(&self) -> u64 {
        u64::from(self.read_weight) + u64::from(self.write_weight)
    }

    fn choose_read(&self, rng: &mut SyntheticTraceRng) -> bool {
        rng.next_bounded(self.read_write_total()) < u64::from(self.read_weight)
    }
}

/// Error raised while generating a synthetic trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntheticTraceError {
    /// `agent_count` must be greater than zero.
    AgentCountZero,
    /// `transaction_count` must be greater than zero.
    TransactionCountZero,
    /// At least one read or write weight must be non-zero.
    EmptyReadWriteMix,
    /// Hot-key skew is a percentage and must be at most 100.
    HotKeyRatioOutOfRange(u8),
    /// Generated raw trace data failed schema validation.
    Trace(TraceScrubError),
}

impl fmt::Display for SyntheticTraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AgentCountZero => {
                f.write_str("synthetic trace agent_count must be greater than zero")
            }
            Self::TransactionCountZero => {
                f.write_str("synthetic trace transaction_count must be greater than zero")
            }
            Self::EmptyReadWriteMix => {
                f.write_str("synthetic trace read/write mix must include at least one operation")
            }
            Self::HotKeyRatioOutOfRange(value) => {
                write!(
                    f,
                    "synthetic trace hot_key_ratio must be <= 100, got {value}"
                )
            }
            Self::Trace(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SyntheticTraceError {}

impl From<TraceScrubError> for SyntheticTraceError {
    fn from(value: TraceScrubError) -> Self {
        Self::Trace(value)
    }
}

/// Error raised while validating trace fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceScrubError {
    /// A required field was empty after trimming.
    EmptyField(&'static str),
    /// A SQL statement was empty after trimming.
    EmptySql,
    /// A boundary marker was provided without a transaction id.
    BoundaryWithoutTransactionId,
    /// A trace was created without statements.
    EmptyStatements,
}

impl fmt::Display for TraceScrubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "required trace field `{field}` is empty"),
            Self::EmptySql => f.write_str("trace statement SQL is empty"),
            Self::BoundaryWithoutTransactionId => {
                f.write_str("transaction boundary requires a transaction id")
            }
            Self::EmptyStatements => f.write_str("agent-swarm trace must contain statements"),
        }
    }
}

impl std::error::Error for TraceScrubError {}

/// Scrub sensitive SQL literals while preserving statement topology.
pub fn scrub_sql_statement(sql: &str) -> ScrubbedSql {
    let chars = sql.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(sql.len());
    let mut redactions = RedactionCounts::default();
    let mut parameter_count = 0;
    let mut i = 0;

    while i < chars.len() {
        if starts_blob_literal(&chars, i) {
            out.push_str("?b");
            redactions.blob_literals += 1;
            i = skip_single_quoted(&chars, i + 2);
            continue;
        }

        let ch = chars[i];
        if ch == '\'' {
            out.push_str("?s");
            redactions.string_literals += 1;
            i = skip_single_quoted(&chars, i + 1);
            continue;
        }

        if ch == '-' && chars.get(i + 1) == Some(&'-') {
            out.push_str("--?c");
            redactions.comments += 1;
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        if ch == '/' && chars.get(i + 1) == Some(&'*') {
            out.push_str("/*?c*/");
            redactions.comments += 1;
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < chars.len() {
                i += 2;
            }
            continue;
        }

        if starts_numeric_literal(&chars, i) {
            out.push_str("?n");
            redactions.numeric_literals += 1;
            i = consume_numeric_literal(&chars, i);
            continue;
        }

        if let Some(parameter_end) = consume_parameter(&chars, i) {
            parameter_count += 1;
            for parameter_char in &chars[i..parameter_end] {
                out.push(*parameter_char);
            }
            i = parameter_end;
            continue;
        }

        out.push(ch);
        i += 1;
    }

    let statement_shape = sha256_hex(out.as_bytes());
    ScrubbedSql {
        sql: out,
        statement_shape,
        parameter_count,
        redactions,
    }
}

/// Generate a deterministic synthetic trace from a fixed seed and workload mix.
pub fn generate_synthetic_swarm_trace(
    config: SyntheticTraceConfig,
) -> Result<AgentSwarmTrace, SyntheticTraceError> {
    config.validate()?;

    let mut rng = SyntheticTraceRng::new(config.seed);
    let mut statements = Vec::with_capacity(config.transaction_count.saturating_mul(3));
    let mut logical_order = 0_u64;

    for tx_index in 0..config.transaction_count {
        let context = synthetic_transaction_context(&config, &mut rng, tx_index);
        statements.push(synthetic_statement(
            logical_order,
            &context,
            SyntheticStatementSpec::begin(),
        )?);
        logical_order += 1;

        let is_read = config.choose_read(&mut rng);
        let sequence = rng.next_u64();
        statements.push(synthetic_statement(
            logical_order,
            &context,
            synthetic_operation_spec(&config, &context, sequence, is_read),
        )?);
        logical_order += 1;

        statements.push(synthetic_statement(
            logical_order,
            &context,
            SyntheticStatementSpec::commit(),
        )?);
        logical_order += 1;
    }

    let trace = AgentSwarmTrace::new(
        synthetic_trace_id(&config),
        synthetic_run_id(&config),
        synthetic_scenario_id(config.scenario),
        synthetic_trace_metadata(&config)?,
        statements,
    )?;
    log_synthetic_swarm_trace_generation(&config, &trace, None);
    Ok(trace)
}

/// Emit the structured summary fields required by replay-lab operators.
pub fn log_swarm_trace_scrub_summary(trace: &AgentSwarmTrace, first_failure_diag: Option<&str>) {
    tracing::info!(
        target: "fsqlite.agent_swarm_trace",
        trace_id = %trace.trace_id,
        run_id = %trace.run_id,
        scenario_id = %trace.scenario_id,
        scrubber_version = %trace.scrubber_version,
        redaction_count = trace.redaction_summary.redaction_count,
        statement_count = trace.statement_count(),
        transaction_count = trace.transaction_count(),
        first_failure_diag = first_failure_diag.unwrap_or(FIRST_FAILURE_DIAGNOSTIC_ABSENT),
        "agent swarm trace scrub summary",
    );
}

/// Emit the structured fields required for synthetic trace generation logs.
pub fn log_synthetic_swarm_trace_generation(
    config: &SyntheticTraceConfig,
    trace: &AgentSwarmTrace,
    first_failure_diag: Option<&str>,
) {
    tracing::info!(
        target: "fsqlite.agent_swarm_trace.synthetic",
        trace_id = %trace.trace_id,
        run_id = %trace.run_id,
        scenario_id = %trace.scenario_id,
        seed = config.seed,
        agent_count = config.agent_count,
        transaction_count = config.transaction_count,
        hot_key_ratio = config.hot_key_ratio,
        read_write_mix = %config.read_write_mix(),
        generator_version = SYNTHETIC_TRACE_GENERATOR_VERSION,
        first_failure_diag = first_failure_diag.unwrap_or(FIRST_FAILURE_DIAGNOSTIC_ABSENT),
        "synthetic agent swarm trace generated",
    );
}

struct SyntheticTraceRng {
    state: u64,
}

impl SyntheticTraceRng {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn next_bounded(&mut self, bound: u64) -> u64 {
        debug_assert!(bound > 0);
        self.next_u64() % bound
    }

    fn choose_hot_key(&mut self, hot_key_ratio: u8) -> bool {
        self.next_bounded(100) < u64::from(hot_key_ratio)
    }
}

struct SyntheticTxnContext {
    actor_id: String,
    connection_id: String,
    transaction_id: String,
    concurrency_group: String,
    workload_key: String,
    source_prefix: String,
}

struct SyntheticStatementSpec {
    boundary: TransactionBoundary,
    workload_phase: &'static str,
    sql: String,
    expected_result_class: ExpectedResultClass,
    row_count_class: RowCountClass,
}

impl SyntheticStatementSpec {
    fn begin() -> Self {
        Self {
            boundary: TransactionBoundary::Begin,
            workload_phase: "transaction-begin",
            sql: "BEGIN CONCURRENT".to_owned(),
            expected_result_class: ExpectedResultClass::Success,
            row_count_class: RowCountClass::Zero,
        }
    }

    fn commit() -> Self {
        Self {
            boundary: TransactionBoundary::Commit,
            workload_phase: "transaction-commit",
            sql: "COMMIT".to_owned(),
            expected_result_class: ExpectedResultClass::Success,
            row_count_class: RowCountClass::Zero,
        }
    }
}

fn synthetic_transaction_context(
    config: &SyntheticTraceConfig,
    rng: &mut SyntheticTraceRng,
    tx_index: usize,
) -> SyntheticTxnContext {
    let agent_index = rng.next_bounded(config.agent_count as u64);
    let actor_id = format!("agent-{agent_index:04}");
    let connection_slot = rng.next_bounded(4);
    let connection_id = format!("{actor_id}-conn-{connection_slot:02}");
    let transaction_id = format!("txn-{}-{tx_index:08}", config.scenario.as_str());
    let workload_key = synthetic_workload_key(config, rng);
    let key_temperature = if workload_key.starts_with("hot-") {
        "hot"
    } else {
        "cold"
    };
    let concurrency_group = format!(
        "{}-{key_temperature}-{}",
        config.scenario.as_str(),
        workload_key
    );
    let source_prefix = format!("{}:{tx_index:08}", config.scenario.as_str());

    SyntheticTxnContext {
        actor_id,
        connection_id,
        transaction_id,
        concurrency_group,
        workload_key,
        source_prefix,
    }
}

fn synthetic_workload_key(config: &SyntheticTraceConfig, rng: &mut SyntheticTraceRng) -> String {
    if rng.choose_hot_key(config.hot_key_ratio) {
        let hot_set = (config.agent_count.max(1) as u64).min(16);
        format!("hot-{:02}", rng.next_bounded(hot_set))
    } else {
        format!("cold-{:08x}", rng.next_u64() & 0xFFFF_FFFF)
    }
}

fn synthetic_statement(
    logical_order: u64,
    context: &SyntheticTxnContext,
    spec: SyntheticStatementSpec,
) -> Result<TraceStatement, SyntheticTraceError> {
    let logical_timestamp = format!("tick-{logical_order:012}");
    let source_ref = format!("{}:stmt-{logical_order:012}", context.source_prefix);
    Ok(TraceStatement::from_raw(RawTraceStatement {
        logical_order,
        logical_timestamp: Some(&logical_timestamp),
        actor_id: &context.actor_id,
        connection_id: &context.connection_id,
        transaction_id: Some(&context.transaction_id),
        transaction_boundary: spec.boundary,
        concurrency_group: &context.concurrency_group,
        workload_phase: spec.workload_phase,
        sql: &spec.sql,
        expected_result_class: spec.expected_result_class,
        row_count_class: spec.row_count_class,
        error_class: None,
        source: StatementSource::new("synthetic-generator", source_ref)?,
    })?)
}

fn synthetic_operation_spec(
    config: &SyntheticTraceConfig,
    context: &SyntheticTxnContext,
    sequence: u64,
    is_read: bool,
) -> SyntheticStatementSpec {
    let key = &context.workload_key;
    let actor = &context.actor_id;
    let sequence_bucket = sequence % 1_000_000;
    let sql = match (config.scenario, is_read) {
        (SyntheticTraceScenario::SessionEventAppend, true) => format!(
            "SELECT COUNT(*) FROM session_events WHERE session_id = '{key}' AND agent_id = '{actor}';"
        ),
        (SyntheticTraceScenario::SessionEventAppend, false) => format!(
            "INSERT INTO session_events(session_id, agent_id, event_kind, payload, seq) VALUES ('{key}', '{actor}', 'tool_call', 'payload-{sequence_bucket}', {sequence_bucket});"
        ),
        (SyntheticTraceScenario::TaskQueueClaimLoop, true) => format!(
            "SELECT id, status FROM task_queue WHERE shard = '{key}' AND status = 'ready' ORDER BY priority DESC LIMIT 1;"
        ),
        (SyntheticTraceScenario::TaskQueueClaimLoop, false) => format!(
            "UPDATE task_queue SET status = 'claimed', owner = '{actor}', claim_seq = {sequence_bucket} WHERE shard = '{key}' AND status = 'ready';"
        ),
        (SyntheticTraceScenario::ArtifactIndexMixedLookup, true) => format!(
            "SELECT artifact_id, content_hash FROM artifact_index WHERE workspace = '{key}' AND path_hash = 'path-{sequence_bucket}';"
        ),
        (SyntheticTraceScenario::ArtifactIndexMixedLookup, false) => format!(
            "INSERT INTO artifact_index(workspace, artifact_id, path_hash, content_hash, updated_by) VALUES ('{key}', 'artifact-{sequence_bucket}', 'path-{sequence_bucket}', 'hash-{sequence_bucket}', '{actor}');"
        ),
        (SyntheticTraceScenario::HotCounterStatusRows, true) => {
            format!("SELECT value, updated_by FROM swarm_counters WHERE counter_key = '{key}';")
        }
        (SyntheticTraceScenario::HotCounterStatusRows, false) => format!(
            "UPDATE swarm_counters SET value = value + 1, updated_by = '{actor}', update_seq = {sequence_bucket} WHERE counter_key = '{key}';"
        ),
        (SyntheticTraceScenario::LongReaderBurstWriter, true) => format!(
            "SELECT event_id, payload FROM session_events WHERE session_id = '{key}' AND logical_clock BETWEEN {sequence_bucket} AND {} ORDER BY logical_clock;",
            sequence_bucket + 250
        ),
        (SyntheticTraceScenario::LongReaderBurstWriter, false) => format!(
            "INSERT INTO session_status(session_id, agent_id, status, logical_clock) VALUES ('{key}', '{actor}', 'checkpoint', {sequence_bucket});"
        ),
    };

    SyntheticStatementSpec {
        boundary: TransactionBoundary::None,
        workload_phase: config.scenario.phase(is_read),
        sql,
        expected_result_class: ExpectedResultClass::Success,
        row_count_class: if is_read {
            RowCountClass::Few
        } else {
            RowCountClass::One
        },
    }
}

fn synthetic_trace_metadata(
    config: &SyntheticTraceConfig,
) -> Result<TraceMetadata, SyntheticTraceError> {
    let mut metadata = TraceMetadata::new(
        "synthetic-agent-swarm",
        format!("{}:{:016x}", config.scenario.as_str(), config.seed),
    )?;
    metadata.logical_clock = Some(format!("splitmix64-seed-{:016x}", config.seed));
    metadata.tags.insert(
        "generator_version".to_owned(),
        SYNTHETIC_TRACE_GENERATOR_VERSION.to_owned(),
    );
    metadata.tags.insert(
        "scenario_family".to_owned(),
        config.scenario.as_str().to_owned(),
    );
    metadata
        .tags
        .insert("seed".to_owned(), config.seed.to_string());
    metadata
        .tags
        .insert("agent_count".to_owned(), config.agent_count.to_string());
    metadata.tags.insert(
        "transaction_count".to_owned(),
        config.transaction_count.to_string(),
    );
    metadata
        .tags
        .insert("hot_key_ratio".to_owned(), config.hot_key_ratio.to_string());
    metadata
        .tags
        .insert("read_write_mix".to_owned(), config.read_write_mix());
    metadata.tags.insert(
        "expected_contention_shape".to_owned(),
        config
            .scenario
            .expected_contention_shape(config.hot_key_ratio),
    );
    metadata.tags.insert(
        "target_invariants".to_owned(),
        config.scenario.target_invariants().to_owned(),
    );
    Ok(metadata)
}

fn synthetic_trace_id(config: &SyntheticTraceConfig) -> String {
    format!(
        "trace-synthetic-{}-{:016x}",
        config.scenario.as_str(),
        config.seed
    )
}

fn synthetic_run_id(config: &SyntheticTraceConfig) -> String {
    format!("run-synthetic-{:016x}", config.seed)
}

fn synthetic_scenario_id(scenario: SyntheticTraceScenario) -> String {
    format!("scenario-synthetic-{}", scenario.as_str())
}

fn required_owned(field: &'static str, value: String) -> Result<String, TraceScrubError> {
    if value.trim().is_empty() {
        Err(TraceScrubError::EmptyField(field))
    } else {
        Ok(value)
    }
}

fn required_ref<'a>(field: &'static str, value: &'a str) -> Result<&'a str, TraceScrubError> {
    if value.trim().is_empty() {
        Err(TraceScrubError::EmptyField(field))
    } else {
        Ok(value)
    }
}

fn starts_blob_literal(chars: &[char], i: usize) -> bool {
    let Some(ch) = chars.get(i) else {
        return false;
    };
    (*ch == 'x' || *ch == 'X') && chars.get(i + 1) == Some(&'\'') && is_left_boundary(chars, i)
}

fn skip_single_quoted(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() {
        if chars[i] == '\'' {
            if chars.get(i + 1) == Some(&'\'') {
                i += 2;
            } else {
                return i + 1;
            }
        } else {
            i += 1;
        }
    }
    i
}

fn starts_numeric_literal(chars: &[char], i: usize) -> bool {
    let Some(ch) = chars.get(i) else {
        return false;
    };
    if !is_left_boundary(chars, i) {
        return false;
    }
    ch.is_ascii_digit()
        || (*ch == '.' && chars.get(i + 1).is_some_and(|next| next.is_ascii_digit()))
}

fn consume_numeric_literal(chars: &[char], i: usize) -> usize {
    if chars.get(i) == Some(&'0') && matches!(chars.get(i + 1), Some('x' | 'X')) {
        let mut j = i + 2;
        while chars
            .get(j)
            .is_some_and(|ch| ch.is_ascii_hexdigit() || *ch == '_')
        {
            j += 1;
        }
        return j;
    }

    let mut j = i;
    while chars
        .get(j)
        .is_some_and(|ch| ch.is_ascii_digit() || *ch == '_')
    {
        j += 1;
    }

    if chars.get(j) == Some(&'.') && chars.get(j + 1).is_some_and(|ch| ch.is_ascii_digit()) {
        j += 1;
        while chars
            .get(j)
            .is_some_and(|ch| ch.is_ascii_digit() || *ch == '_')
        {
            j += 1;
        }
    }

    if matches!(chars.get(j), Some('e' | 'E')) {
        let mut exp = j + 1;
        if matches!(chars.get(exp), Some('+' | '-')) {
            exp += 1;
        }
        if chars.get(exp).is_some_and(|ch| ch.is_ascii_digit()) {
            j = exp + 1;
            while chars
                .get(j)
                .is_some_and(|ch| ch.is_ascii_digit() || *ch == '_')
            {
                j += 1;
            }
        }
    }

    j
}

fn consume_parameter(chars: &[char], i: usize) -> Option<usize> {
    match chars.get(i)? {
        '?' => {
            let mut j = i + 1;
            while chars.get(j).is_some_and(|ch| ch.is_ascii_digit()) {
                j += 1;
            }
            Some(j)
        }
        ':' | '@' | '$' => {
            let mut j = i + 1;
            if !chars
                .get(j)
                .is_some_and(|ch| ch.is_ascii_alphabetic() || *ch == '_')
            {
                return None;
            }
            j += 1;
            while chars
                .get(j)
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            {
                j += 1;
            }
            Some(j)
        }
        _ => None,
    }
}

fn is_left_boundary(chars: &[char], i: usize) -> bool {
    i == 0 || !is_identifier_continue(chars[i - 1])
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        write!(out, "{byte:02x}").expect("writing to a string cannot fail");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const GOLDEN: &str = include_str!("../conformance/agent_swarm_trace_sanitized_golden.json");

    #[test]
    fn scrubber_redacts_literal_classes_and_preserves_parameters() {
        let scrubbed = scrub_sql_statement(
            "INSERT INTO t(a,b,c,d,e) VALUES ('secret', 42, X'DEADBEEF', :name, ?1);",
        );

        assert_eq!(
            scrubbed.sql,
            "INSERT INTO t(a,b,c,d,e) VALUES (?s, ?n, ?b, :name, ?1);"
        );
        assert_eq!(scrubbed.parameter_count, 2);
        assert_eq!(scrubbed.redactions.string_literals, 1);
        assert_eq!(scrubbed.redactions.numeric_literals, 1);
        assert_eq!(scrubbed.redactions.blob_literals, 1);
    }

    #[test]
    fn scrubber_handles_multiline_sql_and_comments() {
        let scrubbed = scrub_sql_statement(
            "UPDATE inbox\n/* token: abc */\nSET body = 'line1''line2'\nWHERE id = 9001 -- owner=alice\n",
        );

        assert_eq!(
            scrubbed.sql,
            "UPDATE inbox\n/*?c*/\nSET body = ?s\nWHERE id = ?n --?c\n"
        );
        assert_eq!(scrubbed.redactions.comments, 2);
        assert_eq!(scrubbed.redactions.string_literals, 1);
        assert_eq!(scrubbed.redactions.numeric_literals, 1);
    }

    #[test]
    fn trace_statement_preserves_transaction_boundary_topology() {
        let statement = TraceStatement::from_raw(raw_statement(
            0,
            "agent-a",
            Some("txn-a"),
            TransactionBoundary::Begin,
            "BEGIN CONCURRENT",
        ))
        .expect("valid begin boundary");

        assert_eq!(statement.transaction_id.as_deref(), Some("txn-a"));
        assert_eq!(statement.transaction_boundary, TransactionBoundary::Begin);
        assert_eq!(statement.scrubbed_sql, "BEGIN CONCURRENT");
    }

    #[test]
    fn invalid_input_rejects_empty_fields_and_unattached_boundary() {
        let empty_actor = TraceStatement::from_raw(raw_statement(
            0,
            " ",
            Some("txn-a"),
            TransactionBoundary::Begin,
            "BEGIN",
        ));
        assert_eq!(empty_actor, Err(TraceScrubError::EmptyField("actor_id")));

        let missing_transaction = TraceStatement::from_raw(raw_statement(
            0,
            "agent-a",
            None,
            TransactionBoundary::Commit,
            "COMMIT",
        ));
        assert_eq!(
            missing_transaction,
            Err(TraceScrubError::BoundaryWithoutTransactionId)
        );

        let empty_sql = TraceStatement::from_raw(raw_statement(
            0,
            "agent-a",
            None,
            TransactionBoundary::None,
            "  ",
        ));
        assert_eq!(empty_sql, Err(TraceScrubError::EmptySql));
    }

    #[test]
    fn deterministic_statement_shape_hashes_scrubbed_sql() {
        let first = scrub_sql_statement("SELECT * FROM t WHERE id = 1 AND owner = 'alice'");
        let second = scrub_sql_statement("SELECT * FROM t WHERE id = 2 AND owner = 'bob'");
        let different_shape = scrub_sql_statement("SELECT * FROM t WHERE owner = 'bob'");

        assert_eq!(first.sql, second.sql);
        assert_eq!(first.statement_shape, second.statement_shape);
        assert_ne!(first.statement_shape, different_shape.statement_shape);
        assert_eq!(first.statement_shape.len(), 64);
    }

    #[test]
    fn trace_summary_counts_statements_transactions_and_redactions() {
        let trace = sample_trace();

        assert_eq!(trace.statement_count(), 4);
        assert_eq!(trace.transaction_count(), 1);
        assert_eq!(trace.redaction_summary.statement_count, 4);
        assert_eq!(trace.redaction_summary.transaction_count, 1);
        assert_eq!(trace.redaction_summary.redaction_count, 8);
    }

    #[test]
    fn golden_sanitized_fixture_matches_schema_and_hashes() {
        let trace: AgentSwarmTrace = serde_json::from_str(GOLDEN).expect("golden fixture parses");

        assert_eq!(trace.schema_version, SWARM_TRACE_SCHEMA_VERSION);
        assert_eq!(trace.scrubber_version, SWARM_TRACE_SCRUBBER_VERSION);
        assert_eq!(
            trace.redaction_summary.statement_count,
            trace.statement_count()
        );
        assert_eq!(
            trace.redaction_summary.transaction_count,
            trace.transaction_count()
        );
        for statement in &trace.statements {
            assert_eq!(
                statement.statement_shape,
                scrub_sql_statement(&statement.scrubbed_sql).statement_shape
            );
        }
    }

    #[test]
    fn synthetic_trace_generator_is_seed_deterministic() {
        let config = SyntheticTraceConfig::new(
            SyntheticTraceScenario::TaskQueueClaimLoop,
            0xA11C_E5ED,
            8,
            12,
        )
        .with_read_write_mix(30, 70)
        .with_hot_key_ratio(85);

        let first = generate_synthetic_swarm_trace(config.clone()).expect("first trace");
        let second = generate_synthetic_swarm_trace(config.clone()).expect("second trace");
        assert_eq!(first, second);

        let changed_seed = SyntheticTraceConfig {
            seed: 0xA11C_E5EE,
            ..config
        };
        let third = generate_synthetic_swarm_trace(changed_seed).expect("third trace");
        assert_ne!(first.statements, third.statements);
    }

    #[test]
    fn synthetic_trace_generator_varies_by_scenario_family() {
        let mut operation_shapes = BTreeSet::new();

        for scenario in SYNTHETIC_TRACE_SCENARIOS {
            let trace = generate_synthetic_swarm_trace(SyntheticTraceConfig::new(
                scenario,
                0x51A7_E5E1,
                4,
                4,
            ))
            .expect("scenario trace");

            assert_eq!(
                trace
                    .metadata
                    .tags
                    .get("scenario_family")
                    .map(String::as_str),
                Some(scenario.as_str())
            );
            operation_shapes.insert(trace.statements[1].statement_shape.clone());
        }

        assert_eq!(operation_shapes.len(), SYNTHETIC_TRACE_SCENARIOS.len());
    }

    #[test]
    fn synthetic_trace_generator_records_contention_metadata_and_invariants() {
        let config = SyntheticTraceConfig::new(
            SyntheticTraceScenario::LongReaderBurstWriter,
            0xFEED_FACE,
            6,
            5,
        )
        .with_read_write_mix(80, 20)
        .with_hot_key_ratio(90);

        let trace = generate_synthetic_swarm_trace(config.clone()).expect("synthetic trace");
        let encoded = serde_json::to_string(&trace).expect("trace serializes");
        let decoded: AgentSwarmTrace = serde_json::from_str(&encoded).expect("trace deserializes");

        assert_eq!(trace, decoded);
        assert_eq!(trace.statement_count(), config.transaction_count * 3);
        assert_eq!(trace.transaction_count(), config.transaction_count);
        assert_eq!(
            trace
                .metadata
                .tags
                .get("generator_version")
                .map(String::as_str),
            Some(SYNTHETIC_TRACE_GENERATOR_VERSION)
        );
        assert_eq!(
            trace.metadata.tags.get("seed").map(String::as_str),
            Some("4277009102")
        );
        assert_eq!(
            trace.metadata.tags.get("agent_count").map(String::as_str),
            Some("6")
        );
        assert_eq!(
            trace
                .metadata
                .tags
                .get("transaction_count")
                .map(String::as_str),
            Some("5")
        );
        assert_eq!(
            trace.metadata.tags.get("hot_key_ratio").map(String::as_str),
            Some("90")
        );
        assert_eq!(
            trace
                .metadata
                .tags
                .get("read_write_mix")
                .map(String::as_str),
            Some("read:80:write:20")
        );
        assert!(
            trace
                .metadata
                .tags
                .get("expected_contention_shape")
                .is_some_and(|shape| shape.contains("high_skew"))
        );
        assert!(
            trace
                .metadata
                .tags
                .get("target_invariants")
                .is_some_and(|invariants| invariants.contains("mvcc_visibility_shape"))
        );
        assert!(
            trace
                .statements
                .iter()
                .all(|statement| statement.logical_timestamp.is_some())
        );
        assert!(
            trace
                .statements
                .iter()
                .step_by(3)
                .all(|statement| statement.transaction_boundary == TransactionBoundary::Begin)
        );
    }

    #[test]
    fn synthetic_trace_generator_rejects_invalid_config() {
        let no_agents =
            SyntheticTraceConfig::new(SyntheticTraceScenario::SessionEventAppend, 1, 0, 1);
        assert_eq!(
            generate_synthetic_swarm_trace(no_agents),
            Err(SyntheticTraceError::AgentCountZero)
        );

        let no_transactions =
            SyntheticTraceConfig::new(SyntheticTraceScenario::SessionEventAppend, 1, 1, 0);
        assert_eq!(
            generate_synthetic_swarm_trace(no_transactions),
            Err(SyntheticTraceError::TransactionCountZero)
        );

        let no_mix = SyntheticTraceConfig::new(SyntheticTraceScenario::SessionEventAppend, 1, 1, 1)
            .with_read_write_mix(0, 0);
        assert_eq!(
            generate_synthetic_swarm_trace(no_mix),
            Err(SyntheticTraceError::EmptyReadWriteMix)
        );

        let invalid_hot_key_ratio =
            SyntheticTraceConfig::new(SyntheticTraceScenario::SessionEventAppend, 1, 1, 1)
                .with_hot_key_ratio(101);
        assert_eq!(
            generate_synthetic_swarm_trace(invalid_hot_key_ratio),
            Err(SyntheticTraceError::HotKeyRatioOutOfRange(101))
        );
    }

    proptest! {
        #[test]
        fn synthetic_trace_generation_preserves_bounded_config(
            seed in any::<u64>(),
            scenario_index in 0usize..SYNTHETIC_TRACE_SCENARIOS.len(),
            agent_count in 1usize..12,
            transaction_count in 1usize..20,
            read_weight in 0u16..100,
            write_weight in 0u16..100,
            hot_key_ratio in 0u8..=100,
        ) {
            prop_assume!(u64::from(read_weight) + u64::from(write_weight) > 0);

            let scenario = SYNTHETIC_TRACE_SCENARIOS[scenario_index];
            let config = SyntheticTraceConfig::new(
                scenario,
                seed,
                agent_count,
                transaction_count,
            )
            .with_read_write_mix(read_weight, write_weight)
            .with_hot_key_ratio(hot_key_ratio);
            let trace = generate_synthetic_swarm_trace(config.clone()).expect("synthetic trace");

            prop_assert_eq!(trace.statement_count(), transaction_count * 3);
            prop_assert_eq!(trace.transaction_count(), transaction_count);
            let expected_seed = seed.to_string();
            let expected_agent_count = agent_count.to_string();
            let expected_transaction_count = transaction_count.to_string();
            let expected_hot_key_ratio = hot_key_ratio.to_string();
            let expected_read_write_mix = config.read_write_mix();
            prop_assert_eq!(
                trace.metadata.tags.get("scenario_family").map(String::as_str),
                Some(scenario.as_str())
            );
            prop_assert_eq!(
                trace.metadata.tags.get("seed").map(String::as_str),
                Some(expected_seed.as_str())
            );
            prop_assert_eq!(
                trace.metadata.tags.get("agent_count").map(String::as_str),
                Some(expected_agent_count.as_str())
            );
            prop_assert_eq!(
                trace
                    .metadata
                    .tags
                    .get("transaction_count")
                    .map(String::as_str),
                Some(expected_transaction_count.as_str())
            );
            prop_assert_eq!(
                trace.metadata.tags.get("hot_key_ratio").map(String::as_str),
                Some(expected_hot_key_ratio.as_str())
            );
            prop_assert_eq!(
                trace.metadata.tags.get("read_write_mix").map(String::as_str),
                Some(expected_read_write_mix.as_str())
            );
            let schema_safe = trace
                .statements
                .iter()
                .all(TraceStatement::schema_safe_for_synthetic_property);
            prop_assert!(schema_safe);
        }
    }

    fn raw_statement<'a>(
        logical_order: u64,
        actor_id: &'a str,
        transaction_id: Option<&'a str>,
        transaction_boundary: TransactionBoundary,
        sql: &'a str,
    ) -> RawTraceStatement<'a> {
        RawTraceStatement {
            logical_order,
            logical_timestamp: None,
            actor_id,
            connection_id: "conn-a",
            transaction_id,
            transaction_boundary,
            concurrency_group: "hot-pages",
            workload_phase: "hot-write",
            sql,
            expected_result_class: ExpectedResultClass::Success,
            row_count_class: RowCountClass::Unknown,
            error_class: None,
            source: StatementSource::new("unit-test", "statement").expect("valid source"),
        }
    }

    fn sample_trace() -> AgentSwarmTrace {
        let metadata = TraceMetadata::new("unit-test", "sample").expect("valid metadata");
        let source = StatementSource::new("unit-test", "statement").expect("valid source");
        let statements = vec![
            TraceStatement::from_raw(RawTraceStatement {
                source: source.clone(),
                ..raw_statement(
                    0,
                    "agent-a",
                    Some("txn-a"),
                    TransactionBoundary::Begin,
                    "BEGIN CONCURRENT",
                )
            })
            .expect("begin statement"),
            TraceStatement::from_raw(RawTraceStatement {
                logical_order: 1,
                source: source.clone(),
                ..raw_statement(
                    1,
                    "agent-a",
                    Some("txn-a"),
                    TransactionBoundary::None,
                    "INSERT INTO inbox(owner, body, raw, retries) VALUES ('agent-a', 'secret', X'DEADBEEF', 3); -- sensitive",
                )
            })
            .expect("insert statement"),
            TraceStatement::from_raw(RawTraceStatement {
                logical_order: 2,
                source: source.clone(),
                ..raw_statement(
                    2,
                    "agent-b",
                    Some("txn-a"),
                    TransactionBoundary::None,
                    "UPDATE inbox SET body = :body, retries = retries + 1 WHERE owner = 'agent-a' AND id = 98765;",
                )
            })
            .expect("update statement"),
            TraceStatement::from_raw(RawTraceStatement {
                source,
                ..raw_statement(
                    3,
                    "agent-a",
                    Some("txn-a"),
                    TransactionBoundary::Commit,
                    "COMMIT",
                )
            })
            .expect("commit statement"),
        ];
        AgentSwarmTrace::new(
            "trace-sample",
            "run-sample",
            "scenario-sample",
            metadata,
            statements,
        )
        .expect("sample trace")
    }

    trait SyntheticTraceStatementTestExt {
        fn schema_safe_for_synthetic_property(&self) -> bool;
    }

    impl SyntheticTraceStatementTestExt for TraceStatement {
        fn schema_safe_for_synthetic_property(&self) -> bool {
            !self.actor_id.trim().is_empty()
                && !self.connection_id.trim().is_empty()
                && !self.concurrency_group.trim().is_empty()
                && !self.workload_phase.trim().is_empty()
                && !self.scrubbed_sql.trim().is_empty()
                && self.statement_shape.len() == 64
                && self.transaction_id.is_some()
        }
    }
}
