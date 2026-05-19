//! Stock C SQLite verification helper for concurrency test artifacts.
//!
//! Opens a `.fsqlite` database file via rusqlite (read-only) and runs a battery
//! of PRAGMAs to produce a structured [`VerifyReport`].  Tiered fallback:
//! if rusqlite cannot open the file, all check results are `Skipped` and
//! a raw diagnostic is captured.
//!
//! Bead: bd-bpnnx

use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Result of a single PRAGMA or diagnostic check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "detail")]
pub enum CheckResult {
    Pass,
    Fail(String),
    Skipped(String),
}

impl CheckResult {
    #[must_use]
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }

    #[must_use]
    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail(_))
    }

    #[must_use]
    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped(_))
    }
}

impl std::fmt::Display for CheckResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "Pass"),
            Self::Fail(detail) => write!(f, "Fail: {detail}"),
            Self::Skipped(reason) => write!(f, "Skipped: {reason}"),
        }
    }
}

/// Timing information for each phase of verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyTimings {
    pub open_ms: f64,
    pub quick_check_ms: f64,
    pub integrity_check_ms: f64,
    pub wal_checkpoint_ms: f64,
    pub metadata_ms: f64,
    pub total_ms: f64,
}

/// Structured report from [`verify_with_c_sqlite`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    pub ok: bool,
    pub quick_check: CheckResult,
    pub integrity_check: CheckResult,
    pub wal_checkpoint: CheckResult,
    pub schema_version_u32: u32,
    pub user_version_u32: u32,
    pub page_count: u64,
    pub page_size: u32,
    pub wal_mode: bool,
    pub free_pages: u64,
    pub table_count: u32,
    pub c_sqlite_diagnostics: Option<String>,
    pub timings: VerifyTimings,
}

impl VerifyReport {
    fn skipped(reason: &str) -> Self {
        let skip = CheckResult::Skipped(reason.to_owned());
        Self {
            ok: false,
            quick_check: skip.clone(),
            integrity_check: skip.clone(),
            wal_checkpoint: skip.clone(),
            schema_version_u32: 0,
            user_version_u32: 0,
            page_count: 0,
            page_size: 0,
            wal_mode: false,
            free_pages: 0,
            table_count: 0,
            c_sqlite_diagnostics: Some(reason.to_owned()),
            timings: VerifyTimings {
                open_ms: 0.0,
                quick_check_ms: 0.0,
                integrity_check_ms: 0.0,
                wal_checkpoint_ms: 0.0,
                metadata_ms: 0.0,
                total_ms: 0.0,
            },
        }
    }
}

impl std::fmt::Display for VerifyReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.ok {
            write!(
                f,
                "VERIFY PASSED: {} pages ({}B each), {} tables, {:.1}ms",
                self.page_count, self.page_size, self.table_count, self.timings.total_ms
            )
        } else {
            write!(
                f,
                "VERIFY FAILED: quick_check={}, integrity_check={}, wal_checkpoint={}",
                self.quick_check, self.integrity_check, self.wal_checkpoint
            )
        }
    }
}

/// Error returned when verification infrastructure itself fails.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("database file does not exist: {0}")]
    FileNotFound(String),
    #[error("rusqlite open failed: {0}")]
    OpenFailed(String),
}

/// Open a `.fsqlite` database via stock C SQLite (rusqlite) and run
/// PRAGMA quick_check, integrity_check, metadata reads, and wal_checkpoint.
///
/// Returns a structured [`VerifyReport`] with pass/fail/skipped for each check.
/// If the file cannot be opened, returns a report with all checks `Skipped`.
///
/// # Errors
///
/// Returns [`VerifyError::FileNotFound`] if the path does not exist.
pub fn verify_with_c_sqlite(path: &Path) -> Result<VerifyReport, VerifyError> {
    if !path.exists() {
        return Err(VerifyError::FileNotFound(path.display().to_string()));
    }

    let total_start = Instant::now();

    let open_start = Instant::now();
    let conn = match rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(c) => c,
        Err(e) => {
            let reason = format!("rusqlite open failed: {e}");
            let mut report = VerifyReport::skipped(&reason);
            report.timings.open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
            report.timings.total_ms = total_start.elapsed().as_secs_f64() * 1000.0;
            return Ok(report);
        }
    };
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;

    let qc_start = Instant::now();
    let quick_check = run_pragma_check(&conn, "quick_check");
    let quick_check_ms = qc_start.elapsed().as_secs_f64() * 1000.0;

    let ic_start = Instant::now();
    let integrity_check = run_pragma_check(&conn, "integrity_check");
    let integrity_check_ms = ic_start.elapsed().as_secs_f64() * 1000.0;

    let wc_start = Instant::now();
    let wal_checkpoint = run_wal_checkpoint(&conn);
    let wal_checkpoint_ms = wc_start.elapsed().as_secs_f64() * 1000.0;

    let meta_start = Instant::now();
    let schema_version_u32 = pragma_u32(&conn, "schema_version");
    let user_version_u32 = pragma_u32(&conn, "user_version");
    let page_count = pragma_u64(&conn, "page_count");
    let page_size = pragma_u32(&conn, "page_size");
    let free_pages = pragma_u64(&conn, "freelist_count");
    let wal_mode =
        pragma_string(&conn, "journal_mode").is_some_and(|m| m.eq_ignore_ascii_case("wal"));
    let table_count = count_user_tables(&conn);
    let metadata_ms = meta_start.elapsed().as_secs_f64() * 1000.0;

    let total_ms = total_start.elapsed().as_secs_f64() * 1000.0;

    let ok = quick_check.is_pass() && integrity_check.is_pass();

    Ok(VerifyReport {
        ok,
        quick_check,
        integrity_check,
        wal_checkpoint,
        schema_version_u32,
        user_version_u32,
        page_count,
        page_size,
        wal_mode,
        free_pages,
        table_count,
        c_sqlite_diagnostics: None,
        timings: VerifyTimings {
            open_ms,
            quick_check_ms,
            integrity_check_ms,
            wal_checkpoint_ms,
            metadata_ms,
            total_ms,
        },
    })
}

fn run_pragma_check(conn: &rusqlite::Connection, pragma: &str) -> CheckResult {
    let sql = format!("PRAGMA {pragma}");
    match conn.prepare(&sql) {
        Ok(mut stmt) => match stmt.query_map([], |row| row.get::<_, String>(0)) {
            Ok(rows) => {
                let mut results: Vec<String> = Vec::new();
                for row in rows {
                    match row {
                        Ok(val) => results.push(val),
                        Err(e) => return CheckResult::Fail(format!("row read error: {e}")),
                    }
                }
                let detail = results.join("; ");
                if results.is_empty() || detail == "ok" {
                    CheckResult::Pass
                } else {
                    CheckResult::Fail(detail)
                }
            }
            Err(e) => CheckResult::Fail(format!("{pragma} query failed: {e}")),
        },
        Err(e) => CheckResult::Fail(format!("{pragma} prepare failed: {e}")),
    }
}

fn run_wal_checkpoint(conn: &rusqlite::Connection) -> CheckResult {
    match conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE)") {
        Ok(()) => CheckResult::Pass,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not in WAL mode") || msg.contains("no such") {
                CheckResult::Skipped(format!("not in WAL mode: {e}"))
            } else {
                CheckResult::Fail(format!("wal_checkpoint failed: {e}"))
            }
        }
    }
}

fn pragma_u32(conn: &rusqlite::Connection, pragma: &str) -> u32 {
    let sql = format!("PRAGMA {pragma}");
    conn.query_row(&sql, [], |row| row.get::<_, u32>(0))
        .unwrap_or(0)
}

fn pragma_u64(conn: &rusqlite::Connection, pragma: &str) -> u64 {
    let sql = format!("PRAGMA {pragma}");
    conn.query_row(&sql, [], |row| row.get::<_, u64>(0))
        .unwrap_or(0)
}

fn pragma_string(conn: &rusqlite::Connection, pragma: &str) -> Option<String> {
    let sql = format!("PRAGMA {pragma}");
    conn.query_row(&sql, [], |row| row.get::<_, String>(0)).ok()
}

fn count_user_tables(conn: &rusqlite::Connection) -> u32 {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get::<_, u32>(0),
    )
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_db() -> NamedTempFile {
        let f = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(f.path()).unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE t1 (id INTEGER PRIMARY KEY, val TEXT);
             INSERT INTO t1 VALUES (1, 'alpha');
             INSERT INTO t1 VALUES (2, 'beta');
             INSERT INTO t1 VALUES (3, 'gamma');
             CREATE TABLE t2 (id INTEGER PRIMARY KEY, num REAL);
             INSERT INTO t2 VALUES (10, 3.14);",
        )
        .unwrap();
        f
    }

    #[test]
    fn clean_db_all_pass() {
        let db = create_test_db();
        let report = verify_with_c_sqlite(db.path()).unwrap();
        assert!(report.ok, "clean DB should pass: {report}");
        assert!(report.quick_check.is_pass());
        assert!(report.integrity_check.is_pass());
        assert_eq!(report.table_count, 2);
        assert!(report.page_count > 0);
        assert!(report.page_size > 0);
        assert!(report.wal_mode);
        assert!(report.timings.total_ms >= 0.0);
    }

    #[test]
    fn nonexistent_file_returns_error() {
        let result = verify_with_c_sqlite(Path::new("/tmp/nonexistent_fsqlite_test_db_bpnnx.db"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerifyError::FileNotFound(_)));
    }

    #[test]
    fn corrupted_db_produces_fail() {
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), b"this is not a valid sqlite database at all").unwrap();
        let report = verify_with_c_sqlite(f.path()).unwrap();
        assert!(!report.ok);
        let has_fail = report.quick_check.is_fail()
            || report.integrity_check.is_fail()
            || report.quick_check.is_skipped();
        assert!(has_fail, "corrupted DB should fail or skip: {report}");
    }

    #[test]
    fn empty_db_passes() {
        let f = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(f.path()).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        drop(conn);
        let report = verify_with_c_sqlite(f.path()).unwrap();
        assert!(report.ok, "empty WAL DB should pass: {report}");
        assert_eq!(report.table_count, 0);
        assert!(report.wal_mode);
    }

    #[test]
    fn non_wal_db_skips_checkpoint() {
        let f = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(f.path()).unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode=DELETE;
             CREATE TABLE t (id INTEGER PRIMARY KEY);
             INSERT INTO t VALUES (1);",
        )
        .unwrap();
        drop(conn);
        let report = verify_with_c_sqlite(f.path()).unwrap();
        assert!(report.ok);
        assert!(!report.wal_mode);
    }

    #[test]
    fn report_serializes_to_json() {
        let db = create_test_db();
        let report = verify_with_c_sqlite(db.path()).unwrap();
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("\"ok\": true"));
        assert!(json.contains("\"quick_check\""));
        assert!(json.contains("\"integrity_check\""));
        let roundtrip: VerifyReport = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.ok, report.ok);
        assert_eq!(roundtrip.page_count, report.page_count);
        assert_eq!(roundtrip.table_count, report.table_count);
    }

    #[test]
    fn metadata_reads_correct_values() {
        let db = create_test_db();
        let report = verify_with_c_sqlite(db.path()).unwrap();
        assert!(report.page_size == 4096 || report.page_size == 1024);
        assert!(report.schema_version_u32 > 0);
        assert_eq!(report.table_count, 2);
    }

    #[test]
    fn display_trait_on_pass() {
        let db = create_test_db();
        let report = verify_with_c_sqlite(db.path()).unwrap();
        let s = format!("{report}");
        assert!(s.contains("VERIFY PASSED"));
    }

    #[test]
    fn display_trait_on_fail() {
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), b"not a database").unwrap();
        let report = verify_with_c_sqlite(f.path()).unwrap();
        let s = format!("{report}");
        assert!(s.contains("VERIFY FAILED") || s.contains("Skipped"));
    }
}
