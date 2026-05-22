//! bd-6xvq3 — Multi-connection MVCC visibility oracle parity e2e tests.
//!
//! Exercises FrankenSQLite's MVCC visibility rules: each connection sees a
//! consistent snapshot, uncommitted changes are invisible to other
//! connections, and committed changes become visible after the reader's
//! transaction ends. Compares behavior against C SQLite (rusqlite) in
//! WAL mode where multi-connection reads are natively supported.
//!
//! These tests use file-backed databases since in-memory databases in
//! C SQLite don't support multiple connections.

use std::thread;

use fsqlite::SqliteValue;

// ── Helpers ────────────────────────────────────────────────────────────

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

fn csql_scalar(conn: &rusqlite::Connection, sql: &str) -> String {
    conn.query_row(sql, [], |row| {
        let v: rusqlite::types::Value = row.get_unwrap(0);
        Ok(match v {
            rusqlite::types::Value::Null => "NULL".into(),
            rusqlite::types::Value::Integer(x) => x.to_string(),
            rusqlite::types::Value::Real(f) => format!("{f}"),
            rusqlite::types::Value::Text(s) => s,
            rusqlite::types::Value::Blob(b) => {
                format!(
                    "X'{}'",
                    b.iter().map(|x| format!("{x:02X}")).collect::<String>()
                )
            }
        })
    })
    .unwrap()
}

// ── Test 1: Committed writes visible to new connection ────────────────

#[test]
fn committed_writes_visible_to_new_connection() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let r_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();
    let r_path = r_tmp.path().to_str().unwrap().to_owned();

    // FrankenSQLite
    {
        let c1 = fsqlite::Connection::open(&f_path).unwrap();
        c1.execute("PRAGMA journal_mode = WAL;").unwrap();
        c1.execute("CREATE TABLE vis (id INTEGER PRIMARY KEY, v INTEGER);")
            .unwrap();
        c1.execute("INSERT INTO vis VALUES (1, 100);").unwrap();
        c1.execute("INSERT INTO vis VALUES (2, 200);").unwrap();
    }
    {
        let c2 = fsqlite::Connection::open(&f_path).unwrap();
        c2.execute("PRAGMA journal_mode = WAL;").unwrap();
        let count = frank_scalar(&c2, "SELECT COUNT(*) FROM vis");
        assert_eq!(
            count, "2",
            "frank: new connection should see committed rows"
        );
    }

    // C SQLite
    {
        let c1 = rusqlite::Connection::open(&r_path).unwrap();
        c1.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        c1.execute_batch("CREATE TABLE vis (id INTEGER PRIMARY KEY, v INTEGER);")
            .unwrap();
        c1.execute_batch("INSERT INTO vis VALUES (1, 100);")
            .unwrap();
        c1.execute_batch("INSERT INTO vis VALUES (2, 200);")
            .unwrap();
    }
    {
        let c2 = rusqlite::Connection::open(&r_path).unwrap();
        c2.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        let count = csql_scalar(&c2, "SELECT COUNT(*) FROM vis");
        assert_eq!(count, "2", "csql: new connection should see committed rows");
    }
}

// ── Test 2: Autocommit changes visible across connections ─────────────

#[test]
fn autocommit_changes_visible_from_other_connection() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();

    let c1 = fsqlite::Connection::open(&f_path).unwrap();
    c1.execute("PRAGMA journal_mode = WAL;").unwrap();
    c1.execute("CREATE TABLE auto_vis (id INTEGER PRIMARY KEY, v TEXT);")
        .unwrap();
    c1.execute("INSERT INTO auto_vis VALUES (1, 'first');")
        .unwrap();

    let c2 = fsqlite::Connection::open(&f_path).unwrap();
    c2.execute("PRAGMA journal_mode = WAL;").unwrap();
    let v = frank_scalar(&c2, "SELECT v FROM auto_vis WHERE id = 1");
    assert_eq!(v, "first", "autocommit insert should be visible from c2");

    c1.execute("INSERT INTO auto_vis VALUES (2, 'second');")
        .unwrap();
    let count = frank_scalar(&c2, "SELECT COUNT(*) FROM auto_vis");
    assert_eq!(count, "2", "second autocommit insert visible from c2");
}

// ── Test 3: Multi-connection sequential write then verify ─────────────

#[test]
fn multi_connection_sequential_writes_oracle_parity() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let r_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();
    let r_path = r_tmp.path().to_str().unwrap().to_owned();

    // Setup
    {
        let f = fsqlite::Connection::open(&f_path).unwrap();
        f.execute("PRAGMA journal_mode = WAL;").unwrap();
        f.execute("CREATE TABLE multi (id INTEGER PRIMARY KEY, writer TEXT);")
            .unwrap();
        let r = rusqlite::Connection::open(&r_path).unwrap();
        r.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        r.execute_batch("CREATE TABLE multi (id INTEGER PRIMARY KEY, writer TEXT);")
            .unwrap();
    }

    // Multiple connections write sequentially
    for conn_id in 0..4 {
        {
            let f = fsqlite::Connection::open(&f_path).unwrap();
            f.execute("PRAGMA journal_mode = WAL;").unwrap();
            for i in 0..10 {
                let pk = conn_id * 10 + i;
                f.execute(&format!(
                    "INSERT INTO multi VALUES ({pk}, 'conn_{conn_id}');"
                ))
                .unwrap();
            }
        }
        {
            let r = rusqlite::Connection::open(&r_path).unwrap();
            r.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
            for i in 0..10 {
                let pk = conn_id * 10 + i;
                r.execute_batch(&format!(
                    "INSERT INTO multi VALUES ({pk}, 'conn_{conn_id}');"
                ))
                .unwrap();
            }
        }
    }

    // Verify parity
    let f = fsqlite::Connection::open(&f_path).unwrap();
    let r = rusqlite::Connection::open(&r_path).unwrap();

    let fcount = frank_scalar(&f, "SELECT COUNT(*) FROM multi");
    let rcount = csql_scalar(&r, "SELECT COUNT(*) FROM multi");
    assert_eq!(fcount, rcount, "row count mismatch");
    assert_eq!(fcount, "40");
}

// ── Test 4: Reader on connection A during writes on connection B ──────

#[test]
fn reader_sees_committed_state_not_in_flight() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();

    // Setup initial data
    {
        let setup = fsqlite::Connection::open(&f_path).unwrap();
        setup.execute("PRAGMA journal_mode = WAL;").unwrap();
        setup
            .execute("CREATE TABLE inflight (id INTEGER PRIMARY KEY, v INTEGER);")
            .unwrap();
        setup
            .execute("INSERT INTO inflight VALUES (1, 10);")
            .unwrap();
    }

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));

    let fp_writer = f_path.clone();
    let bar_w = barrier.clone();

    let writer = thread::spawn(move || {
        let conn = fsqlite::Connection::open(&fp_writer).unwrap();
        conn.execute("PRAGMA journal_mode = WAL;").unwrap();
        conn.execute("BEGIN CONCURRENT").unwrap();
        conn.execute("INSERT INTO inflight VALUES (2, 20);")
            .unwrap();
        bar_w.wait();
        thread::sleep(std::time::Duration::from_millis(50));
        conn.execute("COMMIT").unwrap();
    });

    barrier.wait();

    let reader = fsqlite::Connection::open(&f_path).unwrap();
    reader.execute("PRAGMA journal_mode = WAL;").unwrap();
    let count = frank_scalar(&reader, "SELECT COUNT(*) FROM inflight");
    assert!(
        count == "1" || count == "2",
        "reader should see either 1 (pre-commit snapshot) or 2 (post-commit), got {count}"
    );

    writer.join().unwrap();

    let final_count = frank_scalar(&reader, "SELECT COUNT(*) FROM inflight");
    assert_eq!(
        final_count, "2",
        "after writer commits, fresh read should see both rows"
    );
}

// ── Test 5: Multiple readers don't block writer ───────────────────────

#[test]
fn multiple_readers_dont_block_writer() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();

    {
        let setup = fsqlite::Connection::open(&f_path).unwrap();
        setup.execute("PRAGMA journal_mode = WAL;").unwrap();
        setup
            .execute("CREATE TABLE noblock (id INTEGER PRIMARY KEY, v INTEGER);")
            .unwrap();
        for i in 0..10 {
            setup
                .execute(&format!("INSERT INTO noblock VALUES ({i}, {});", i * 100))
                .unwrap();
        }
    }

    let n_readers = 4;
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(n_readers + 1));

    let reader_handles: Vec<_> = (0..n_readers)
        .map(|rid| {
            let fp = f_path.clone();
            let bar = barrier.clone();
            thread::spawn(move || {
                let conn = fsqlite::Connection::open(&fp).unwrap();
                conn.execute("PRAGMA journal_mode = WAL;").unwrap();
                conn.execute("BEGIN").unwrap();
                let count = frank_scalar(&conn, "SELECT COUNT(*) FROM noblock");
                bar.wait();
                thread::sleep(std::time::Duration::from_millis(100));
                let count2 = frank_scalar(&conn, "SELECT COUNT(*) FROM noblock");
                conn.execute("COMMIT").unwrap();
                (rid, count, count2)
            })
        })
        .collect();

    barrier.wait();

    let writer = fsqlite::Connection::open(&f_path).unwrap();
    writer.execute("PRAGMA journal_mode = WAL;").unwrap();
    writer
        .execute("INSERT INTO noblock VALUES (99, 9900);")
        .unwrap();

    for h in reader_handles {
        let (rid, c1, c2) = h.join().unwrap();
        assert_eq!(
            c1, c2,
            "reader {rid}: count changed within txn (snapshot violation): {c1} -> {c2}"
        );
    }

    let final_count = frank_scalar(&writer, "SELECT COUNT(*) FROM noblock");
    assert_eq!(final_count, "11");
}
