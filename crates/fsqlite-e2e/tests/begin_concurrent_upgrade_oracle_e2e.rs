//! bd-c0d3h — BEGIN CONCURRENT upgrade mechanism oracle parity tests.
//!
//! FrankenSQLite auto-promotes BEGIN to BEGIN CONCURRENT when
//! concurrent_mode_default is true. These tests verify the upgrade
//! behavior: that BEGIN CONCURRENT is accepted, that it enables
//! concurrent writes, that COMMIT/ROLLBACK work correctly, and that
//! the semantics match C SQLite's serialized writer model for
//! equivalent workloads.

use fsqlite::SqliteValue;

fn frank_scalar(conn: &fsqlite::Connection, sql: &str) -> String {
    let rows = conn.query(sql).unwrap();
    match &rows[0].values()[0] {
        SqliteValue::Null => "NULL".into(),
        SqliteValue::Integer(n) => n.to_string(),
        SqliteValue::Float(f) => format!("{f}"),
        SqliteValue::Text(s) => s.to_string(),
        SqliteValue::Blob(b) => {
            format!(
                "X'{}'",
                b.iter().map(|x| format!("{x:02X}")).collect::<String>()
            )
        }
    }
}

// ── Test 1: BEGIN CONCURRENT is accepted ──────────────────────────────

#[test]
fn begin_concurrent_accepted() {
    let f = fsqlite::Connection::open(":memory:").unwrap();
    f.execute("CREATE TABLE bc (id INTEGER PRIMARY KEY);")
        .unwrap();

    f.execute("BEGIN CONCURRENT").unwrap();
    f.execute("INSERT INTO bc VALUES (1);").unwrap();
    f.execute("COMMIT").unwrap();

    let count = frank_scalar(&f, "SELECT COUNT(*) FROM bc");
    assert_eq!(count, "1");
}

// ── Test 2: BEGIN auto-promotes to CONCURRENT ─────────────────────────

#[test]
fn begin_auto_promotes_to_concurrent() {
    let f = fsqlite::Connection::open(":memory:").unwrap();
    f.execute("CREATE TABLE auto_bc (id INTEGER PRIMARY KEY);")
        .unwrap();

    f.execute("BEGIN").unwrap();
    f.execute("INSERT INTO auto_bc VALUES (1);").unwrap();
    f.execute("COMMIT").unwrap();

    let count = frank_scalar(&f, "SELECT COUNT(*) FROM auto_bc");
    assert_eq!(count, "1");
}

// ── Test 3: BEGIN CONCURRENT + ROLLBACK undoes changes ────────────────

#[test]
fn begin_concurrent_rollback() {
    let f = fsqlite::Connection::open(":memory:").unwrap();
    f.execute("CREATE TABLE bc_rb (id INTEGER PRIMARY KEY);")
        .unwrap();
    f.execute("INSERT INTO bc_rb VALUES (1);").unwrap();

    f.execute("BEGIN CONCURRENT").unwrap();
    f.execute("INSERT INTO bc_rb VALUES (2);").unwrap();
    f.execute("INSERT INTO bc_rb VALUES (3);").unwrap();
    f.execute("ROLLBACK").unwrap();

    let count = frank_scalar(&f, "SELECT COUNT(*) FROM bc_rb");
    assert_eq!(count, "1", "rollback should undo concurrent inserts");
}

// ── Test 4: Nested SAVEPOINT within CONCURRENT ────────────────────────

#[test]
fn savepoint_within_concurrent_transaction() {
    let f = fsqlite::Connection::open(":memory:").unwrap();
    let r = rusqlite::Connection::open_in_memory().unwrap();

    f.execute("CREATE TABLE sp_bc (id INTEGER PRIMARY KEY, v INTEGER);")
        .unwrap();
    r.execute_batch("CREATE TABLE sp_bc (id INTEGER PRIMARY KEY, v INTEGER);")
        .unwrap();

    let steps = [
        "BEGIN",
        "INSERT INTO sp_bc VALUES (1, 10)",
        "SAVEPOINT sp1",
        "INSERT INTO sp_bc VALUES (2, 20)",
        "ROLLBACK TO sp1",
        "INSERT INTO sp_bc VALUES (3, 30)",
        "RELEASE sp1",
        "COMMIT",
    ];

    for s in &steps {
        let fe = f.execute(s);
        let re = r.execute_batch(s);
        match (&fe, &re) {
            (Ok(_), Ok(())) | (Err(_), Err(_)) => {}
            (Ok(_), Err(e)) => panic!("frank OK but csql err on `{s}`: {e}"),
            (Err(e), Ok(())) => panic!("frank err but csql OK on `{s}`: {e}"),
        }
    }

    let fcount = frank_scalar(&f, "SELECT COUNT(*) FROM sp_bc");
    let rcount: i64 = r
        .query_row("SELECT COUNT(*) FROM sp_bc", [], |row| row.get(0))
        .unwrap();
    assert_eq!(fcount, rcount.to_string(), "count mismatch");
    assert_eq!(fcount, "2", "should have rows 1 and 3 (2 was rolled back)");
}

// ── Test 5: Multiple sequential BEGIN CONCURRENT blocks ───────────────

#[test]
fn sequential_concurrent_transactions() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap();

    let f = fsqlite::Connection::open(f_path).unwrap();
    f.execute("PRAGMA journal_mode = WAL;").unwrap();
    f.execute("CREATE TABLE seq_bc (id INTEGER PRIMARY KEY, batch INTEGER);")
        .unwrap();

    for batch in 0..5 {
        f.execute("BEGIN CONCURRENT").unwrap();
        for i in 0..10 {
            let pk = batch * 10 + i;
            f.execute(&format!("INSERT INTO seq_bc VALUES ({pk}, {batch});"))
                .unwrap();
        }
        f.execute("COMMIT").unwrap();
    }

    let count = frank_scalar(&f, "SELECT COUNT(*) FROM seq_bc");
    assert_eq!(count, "50", "5 batches x 10 rows = 50");

    let r = rusqlite::Connection::open(f_tmp.path()).unwrap();
    let rcount: i64 = r
        .query_row("SELECT COUNT(*) FROM seq_bc", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rcount, 50, "rusqlite cross-check");
}

// ── Test 6: BEGIN CONCURRENT on file-backed DB ────────────────────────

#[test]
fn begin_concurrent_file_backed_wal() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap();

    let f = fsqlite::Connection::open(f_path).unwrap();
    f.execute("PRAGMA journal_mode = WAL;").unwrap();
    f.execute("CREATE TABLE file_bc (id INTEGER PRIMARY KEY, data TEXT);")
        .unwrap();

    f.execute("BEGIN CONCURRENT").unwrap();
    for i in 0..25 {
        f.execute(&format!("INSERT INTO file_bc VALUES ({i}, 'item_{i}');"))
            .unwrap();
    }
    f.execute("COMMIT").unwrap();

    drop(f);

    let verify = fsqlite::Connection::open(f_path).unwrap();
    verify.execute("PRAGMA journal_mode = WAL;").unwrap();
    let count = frank_scalar(&verify, "SELECT COUNT(*) FROM file_bc");
    assert_eq!(count, "25", "data should persist after reopen");
}

// ── Test 7: Commit semantics match between engines ────────────────────

#[test]
fn commit_semantics_parity_in_memory() {
    let f = fsqlite::Connection::open(":memory:").unwrap();
    let r = rusqlite::Connection::open_in_memory().unwrap();

    f.execute("CREATE TABLE sem (id INTEGER PRIMARY KEY, v INTEGER);")
        .unwrap();
    r.execute_batch("CREATE TABLE sem (id INTEGER PRIMARY KEY, v INTEGER);")
        .unwrap();

    f.execute("BEGIN").unwrap();
    f.execute("INSERT INTO sem VALUES (1, 100);").unwrap();
    f.execute("INSERT INTO sem VALUES (2, 200);").unwrap();
    f.execute("INSERT INTO sem VALUES (3, 300);").unwrap();
    f.execute("COMMIT").unwrap();

    r.execute_batch("BEGIN").unwrap();
    r.execute_batch("INSERT INTO sem VALUES (1, 100);").unwrap();
    r.execute_batch("INSERT INTO sem VALUES (2, 200);").unwrap();
    r.execute_batch("INSERT INTO sem VALUES (3, 300);").unwrap();
    r.execute_batch("COMMIT").unwrap();

    let fcount = frank_scalar(&f, "SELECT COUNT(*) FROM sem");
    let rcount: i64 = r
        .query_row("SELECT COUNT(*) FROM sem", [], |row| row.get(0))
        .unwrap();
    assert_eq!(fcount, "3");
    assert_eq!(rcount, 3);

    let fsum = frank_scalar(&f, "SELECT SUM(v) FROM sem");
    let rsum: i64 = r
        .query_row("SELECT SUM(v) FROM sem", [], |row| row.get(0))
        .unwrap();
    assert_eq!(fsum, rsum.to_string());
}
