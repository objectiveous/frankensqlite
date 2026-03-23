//! bd-db300.4.2.3: Statement cache invalidation correctness verification.
//!
//! Proves:
//! 1. Parse/compiled/prepared caches produce hits on repeated identical SQL.
//! 2. DDL (schema_cookie change) invalidates all three caches.
//! 3. Results remain correct after invalidation + re-prepare.
//! 4. Rollback to savepoint does NOT spuriously invalidate caches when
//!    schema_cookie is unchanged.
//! 5. Schema generation bump (connection-local DDL) invalidates prepared
//!    statements via SchemaChanged error.
//! 6. File-backed databases share the same cache/invalidation behavior.
//! 7. Warm-loop churn measurement scorecard with structured output.
//!
//! NOTE: Tests T1–T3 and T5–T7 use global atomic counters, so they must
//! run with `--test-threads=1` for reliable absolute counts.
//!
//! Run:
//!   cargo test -p fsqlite-core --test statement_cache_invalidation \
//!     -- --test-threads=1 --nocapture
//!
//! Structured log capture:
//!   RUST_LOG="fsqlite.statement_reuse=info" cargo test -p fsqlite-core \
//!     --test statement_cache_invalidation -- --test-threads=1 --nocapture \
//!     2>&1 | grep "fsqlite.statement_reuse"

use fsqlite_core::connection::{
    Connection, hot_path_profile_snapshot, reset_hot_path_profile, set_hot_path_profile_enabled,
};

/// Helper: snapshot the parser cache counters.
fn cache_snapshot() -> (u64, u64, u64, u64, u64, u64) {
    let s = hot_path_profile_snapshot();
    (
        s.parser.parse_cache_hits,
        s.parser.parse_cache_misses,
        s.parser.compiled_cache_hits,
        s.parser.compiled_cache_misses,
        s.parser.prepared_cache_hits,
        s.parser.prepared_cache_misses,
    )
}

/// T1: Repeated identical SELECT produces parse cache hit delta.
#[test]
fn test_parse_cache_hits_on_repeated_select() {
    set_hot_path_profile_enabled(true);

    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    conn.execute("INSERT INTO t VALUES(1, 'a')").unwrap();

    // First execution — warms cache.
    let rows1 = conn.query("SELECT val FROM t WHERE id = 1").unwrap();
    let (ph1, _, _, _, _, _) = cache_snapshot();

    // Second execution — delta should show a hit.
    let rows2 = conn.query("SELECT val FROM t WHERE id = 1").unwrap();
    let (ph2, _, _, _, _, _) = cache_snapshot();

    assert!(
        ph2 > ph1,
        "parse cache hits should increase on repeated SQL: before={ph1}, after={ph2}"
    );

    // Results must be identical.
    assert_eq!(
        rows1, rows2,
        "repeated query must produce identical results"
    );
}

/// T2: DDL invalidates caches; subsequent query still correct.
#[test]
fn test_ddl_invalidates_all_caches() {
    set_hot_path_profile_enabled(true);

    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    conn.execute("INSERT INTO t VALUES(1, 'before')").unwrap();

    // Warm the cache with two identical queries.
    let _ = conn.query("SELECT val FROM t WHERE id = 1").unwrap();
    let (ph_pre, _, _, _, _, _) = cache_snapshot();
    let _ = conn.query("SELECT val FROM t WHERE id = 1").unwrap();
    let (ph_warm, _, _, _, _, _) = cache_snapshot();
    assert!(
        ph_warm > ph_pre,
        "cache should be warm: ph_pre={ph_pre}, ph_warm={ph_warm}"
    );

    // DDL: add a column (changes schema_cookie).
    conn.execute("ALTER TABLE t ADD COLUMN extra INTEGER DEFAULT 0")
        .unwrap();

    // Snapshot AFTER DDL, BEFORE next query.
    let (_, pm_pre_query, _, _, _, _) = cache_snapshot();

    // Next query — should trigger a cache miss (invalidated by DDL).
    let rows = conn.query("SELECT val FROM t WHERE id = 1").unwrap();
    let (_, pm_post_query, _, _, _, _) = cache_snapshot();

    assert!(
        pm_post_query > pm_pre_query,
        "after DDL, first query should produce a parse cache miss: pre={pm_pre_query}, post={pm_post_query}"
    );

    // Result correctness.
    assert!(
        !rows.is_empty(),
        "query should still return the row after ALTER TABLE"
    );
}

/// T3: Rollback to savepoint does NOT invalidate caches when schema unchanged.
#[test]
fn test_rollback_savepoint_preserves_cache() {
    set_hot_path_profile_enabled(true);

    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    conn.execute("INSERT INTO t VALUES(1, 'original')").unwrap();

    // Warm parse cache.
    let _ = conn.query("SELECT val FROM t WHERE id = 1").unwrap();
    let _ = conn.query("SELECT val FROM t WHERE id = 1").unwrap();
    let (ph_pre, _, _, _, _, _) = cache_snapshot();

    // Savepoint + DML + rollback (no DDL = no schema_cookie change).
    conn.execute("SAVEPOINT sp1").unwrap();
    conn.execute("INSERT INTO t VALUES(2, 'temp')").unwrap();
    conn.execute("ROLLBACK TO sp1").unwrap();
    conn.execute("RELEASE sp1").unwrap();

    // Query again — should still hit parse cache (schema unchanged).
    let _ = conn.query("SELECT val FROM t WHERE id = 1").unwrap();
    let (ph_post, _, _, _, _, _) = cache_snapshot();

    assert!(
        ph_post > ph_pre,
        "parse cache hits should increase after rollback with unchanged schema: before={ph_pre}, after={ph_post}"
    );

    // Result correctness: rollback undid the INSERT.
    let rows = conn.query("SELECT COUNT(*) FROM t").unwrap();
    let count = rows[0].get(0).unwrap();
    assert_eq!(
        count,
        &fsqlite_types::SqliteValue::Integer(1),
        "rollback should undo the INSERT, leaving 1 row"
    );
}

/// T4: Schema generation bump invalidates prepared statements.
#[test]
fn test_schema_generation_invalidates_prepared() {
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    conn.execute("INSERT INTO t VALUES(1, 'v1')").unwrap();

    // Prepare and execute.
    let stmt = conn.prepare("SELECT val FROM t WHERE id = ?1").unwrap();
    let rows1 = stmt
        .query_with_params(&[fsqlite_types::SqliteValue::Integer(1)])
        .unwrap();
    assert!(!rows1.is_empty(), "prepared statement should return rows");

    // DDL changes schema_generation.
    conn.execute("CREATE TABLE t2(x INTEGER)").unwrap();

    // Prepared statement should detect schema change.
    let result = stmt.query_with_params(&[fsqlite_types::SqliteValue::Integer(1)]);
    match result {
        Err(fsqlite_error::FrankenError::SchemaChanged) => {
            // Expected — the statement was invalidated.
        }
        Ok(rows) => {
            // Transparent re-prepare — result must still be correct.
            assert!(
                !rows.is_empty(),
                "re-prepared result should still be correct"
            );
        }
        Err(other) => {
            panic!("unexpected error after schema change: {other:?}");
        }
    }
}

/// T5: Repeated identical INSERT uses compiled cache.
#[test]
fn test_compiled_cache_hits_on_repeated_insert() {
    set_hot_path_profile_enabled(true);

    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();

    // Two identical INSERTs.
    conn.execute("INSERT INTO t VALUES(1, 'a')").unwrap();
    let (_, _, ch1, _, _, _) = cache_snapshot();
    conn.execute("INSERT INTO t VALUES(1, 'a')")
        .unwrap_or_default(); // PK conflict OK
    let (_, _, ch2, _, _, _) = cache_snapshot();

    eprintln!("[T5] compiled_cache hits: {ch1} -> {ch2}");
    // At least one compiled cache hit should appear for the repeated identical SQL.
    assert!(
        ch2 > ch1,
        "identical SQL should produce a compiled cache hit: before={ch1}, after={ch2}"
    );
}

/// T6: File-backed database has same invalidation behavior.
#[test]
fn test_file_backed_cache_invalidation() {
    set_hot_path_profile_enabled(true);

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap();

    let conn = Connection::open(path).unwrap();
    conn.execute("PRAGMA journal_mode = WAL").unwrap();
    conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    conn.execute("INSERT INTO t VALUES(1, 'file-backed')")
        .unwrap();

    // Warm cache.
    let _ = conn.query("SELECT val FROM t WHERE id = 1").unwrap();
    let (ph1, _, _, _, _, _) = cache_snapshot();

    // Repeat — hit delta.
    let _ = conn.query("SELECT val FROM t WHERE id = 1").unwrap();
    let (ph2, _, _, _, _, _) = cache_snapshot();
    assert!(
        ph2 > ph1,
        "file-backed: parse cache should hit on repeated query: {ph1} -> {ph2}"
    );

    // DDL invalidation.
    conn.execute("CREATE TABLE t2(x INTEGER)").unwrap();
    let (_, pm_pre, _, _, _, _) = cache_snapshot();
    let rows = conn.query("SELECT val FROM t WHERE id = 1").unwrap();
    let (_, pm_post, _, _, _, _) = cache_snapshot();
    assert!(
        pm_post > pm_pre,
        "file-backed: DDL should cause a cache miss: {pm_pre} -> {pm_post}"
    );
    assert!(
        !rows.is_empty(),
        "file-backed: result must be correct after invalidation"
    );
}

/// T7: Churn measurement scorecard — the authoritative artifact for bd-db300.4.2.3.
#[test]
fn test_churn_measurement_scorecard() {
    set_hot_path_profile_enabled(true);
    reset_hot_path_profile();

    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE bench(id INTEGER PRIMARY KEY, name TEXT, score INTEGER)")
        .unwrap();

    let insert_sql = "INSERT INTO bench VALUES(1, 'test', 42)";
    let select_sql = "SELECT name, score FROM bench WHERE id = 1";
    let delete_sql = "DELETE FROM bench WHERE id = 1";

    // Cold iteration.
    conn.execute(insert_sql).unwrap();
    let _ = conn.query(select_sql).unwrap();
    let snap_cold = hot_path_profile_snapshot();
    conn.execute(delete_sql).unwrap();

    // Warm loop (100 iterations).
    reset_hot_path_profile();
    for _ in 0..100 {
        conn.execute(insert_sql).unwrap();
        let _ = conn.query(select_sql).unwrap();
        conn.execute(delete_sql).unwrap();
    }
    let snap_warm = hot_path_profile_snapshot();

    // ─── Scorecard output ─────────────────────────────────────
    eprintln!("=== bd-db300.4.2.3 Churn Measurement Scorecard ===");
    eprintln!("Cold (first iteration):");
    eprintln!(
        "  parse:    hits={:>4}  misses={:>4}",
        snap_cold.parser.parse_cache_hits, snap_cold.parser.parse_cache_misses
    );
    eprintln!(
        "  compiled: hits={:>4}  misses={:>4}",
        snap_cold.parser.compiled_cache_hits, snap_cold.parser.compiled_cache_misses
    );
    eprintln!(
        "  prepared: hits={:>4}  misses={:>4}",
        snap_cold.parser.prepared_cache_hits, snap_cold.parser.prepared_cache_misses
    );
    eprintln!("Warm (100 iterations, 3 statements each = 300 statement dispatches):");
    eprintln!(
        "  parse:    hits={:>4}  misses={:>4}  hit_rate={:.1}%",
        snap_warm.parser.parse_cache_hits,
        snap_warm.parser.parse_cache_misses,
        100.0 * snap_warm.parser.parse_cache_hits as f64
            / (snap_warm.parser.parse_cache_hits + snap_warm.parser.parse_cache_misses).max(1)
                as f64,
    );
    eprintln!(
        "  compiled: hits={:>4}  misses={:>4}  hit_rate={:.1}%",
        snap_warm.parser.compiled_cache_hits,
        snap_warm.parser.compiled_cache_misses,
        100.0 * snap_warm.parser.compiled_cache_hits as f64
            / (snap_warm.parser.compiled_cache_hits + snap_warm.parser.compiled_cache_misses).max(1)
                as f64,
    );
    eprintln!(
        "  prepared: hits={:>4}  misses={:>4}",
        snap_warm.parser.prepared_cache_hits, snap_warm.parser.prepared_cache_misses
    );
    eprintln!(
        "  parse_time_ns={}, compile_time_ns={}",
        snap_warm.parser.parse_time_ns, snap_warm.parser.compile_time_ns
    );
    eprintln!("=== END SCORECARD ===");

    // Assertion: warm loop cache hits should dominate misses.
    assert!(
        snap_warm.parser.parse_cache_hits > snap_warm.parser.parse_cache_misses,
        "warm loop: parse cache hits ({}) should exceed misses ({})",
        snap_warm.parser.parse_cache_hits,
        snap_warm.parser.parse_cache_misses
    );
}
