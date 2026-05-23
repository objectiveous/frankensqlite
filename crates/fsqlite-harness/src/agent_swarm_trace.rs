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
}
