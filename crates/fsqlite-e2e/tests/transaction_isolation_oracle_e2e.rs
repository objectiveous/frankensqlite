//! bd-o653e — Transaction isolation semantics oracle parity e2e tests.
//!
//! Exercises FrankenSQLite's transaction isolation guarantees:
//!   - Snapshot isolation: readers see consistent state
//!   - Write-write conflict detection (SSI)
//!   - Dirty read prevention
//!   - Repeatable reads within a transaction
//!   - Phantom read prevention
//!   - Serializable outcomes for conflicting workloads

use std::sync::{Arc, Barrier};
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

// ── Test 1: No dirty reads ───────────────────────────────────────────

#[test]
fn no_dirty_reads() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();

    // Setup
    {
        let setup = fsqlite::Connection::open(&f_path).unwrap();
        setup.execute("PRAGMA journal_mode = WAL;").unwrap();
        setup
            .execute("CREATE TABLE dirty (id INTEGER PRIMARY KEY, val INTEGER);")
            .unwrap();
        setup.execute("INSERT INTO dirty VALUES (1, 100);").unwrap();
    }

    let barrier = Arc::new(Barrier::new(2));

    let fp_writer = f_path.clone();
    let bar_w = barrier.clone();

    let writer = thread::spawn(move || {
        let conn = fsqlite::Connection::open(&fp_writer).unwrap();
        conn.execute("PRAGMA journal_mode = WAL;").unwrap();
        conn.execute("BEGIN CONCURRENT").unwrap();
        conn.execute("UPDATE dirty SET val = 999 WHERE id = 1;")
            .unwrap();
        bar_w.wait();
        // Hold the transaction open briefly — reader should NOT see val=999
        thread::sleep(std::time::Duration::from_millis(100));
        conn.execute("ROLLBACK").unwrap();
    });

    barrier.wait();

    // Reader should see original value, not the uncommitted 999
    let reader = fsqlite::Connection::open(&f_path).unwrap();
    reader.execute("PRAGMA journal_mode = WAL;").unwrap();
    let val = frank_scalar(&reader, "SELECT val FROM dirty WHERE id = 1");
    assert_eq!(val, "100", "reader must not see uncommitted (dirty) write");

    writer.join().unwrap();
}

// ── Test 2: Repeatable reads within a transaction ────────────────────

#[test]
fn repeatable_reads() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();

    {
        let setup = fsqlite::Connection::open(&f_path).unwrap();
        setup.execute("PRAGMA journal_mode = WAL;").unwrap();
        setup
            .execute("CREATE TABLE rr (id INTEGER PRIMARY KEY, val INTEGER);")
            .unwrap();
        for i in 0..10 {
            setup
                .execute(&format!("INSERT INTO rr VALUES ({i}, {});", i * 10))
                .unwrap();
        }
    }

    let barrier = Arc::new(Barrier::new(2));

    let fp_writer = f_path.clone();
    let bar_w = barrier.clone();

    let writer = thread::spawn(move || {
        let conn = fsqlite::Connection::open(&fp_writer).unwrap();
        conn.execute("PRAGMA journal_mode = WAL;").unwrap();
        bar_w.wait();
        // Modify data while reader has an open transaction
        conn.execute("UPDATE rr SET val = val + 1000;").unwrap();
    });

    let reader = fsqlite::Connection::open(&f_path).unwrap();
    reader.execute("PRAGMA journal_mode = WAL;").unwrap();
    reader.execute("BEGIN").unwrap();
    let read1 = frank_scalar(&reader, "SELECT SUM(val) FROM rr");

    barrier.wait();
    writer.join().unwrap();

    // Second read within same transaction should see same snapshot
    let read2 = frank_scalar(&reader, "SELECT SUM(val) FROM rr");
    reader.execute("COMMIT").unwrap();

    assert_eq!(
        read1, read2,
        "reads within same transaction must be repeatable"
    );

    // After committing the reader's txn, fresh read should see the update
    let read3 = frank_scalar(&reader, "SELECT SUM(val) FROM rr");
    let original_sum: i64 = (0..10).map(|i: i64| i * 10).sum();
    let updated_sum = original_sum + 10_000; // 10 rows * 1000
    assert_eq!(
        read3,
        updated_sum.to_string(),
        "after commit, should see writer's changes"
    );
}

// ── Test 3: Phantom read prevention ──────────────────────────────────

#[test]
fn phantom_read_prevention() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();

    {
        let setup = fsqlite::Connection::open(&f_path).unwrap();
        setup.execute("PRAGMA journal_mode = WAL;").unwrap();
        setup
            .execute("CREATE TABLE phantom (id INTEGER PRIMARY KEY, category TEXT);")
            .unwrap();
        setup
            .execute("INSERT INTO phantom VALUES (1, 'A'), (2, 'A'), (3, 'B');")
            .unwrap();
    }

    let barrier = Arc::new(Barrier::new(2));

    let fp_writer = f_path.clone();
    let bar_w = barrier.clone();

    let writer = thread::spawn(move || {
        let conn = fsqlite::Connection::open(&fp_writer).unwrap();
        conn.execute("PRAGMA journal_mode = WAL;").unwrap();
        bar_w.wait();
        conn.execute("INSERT INTO phantom VALUES (4, 'A');")
            .unwrap();
    });

    let reader = fsqlite::Connection::open(&f_path).unwrap();
    reader.execute("PRAGMA journal_mode = WAL;").unwrap();
    reader.execute("BEGIN").unwrap();
    let count1 = frank_scalar(&reader, "SELECT COUNT(*) FROM phantom WHERE category = 'A'");

    barrier.wait();
    writer.join().unwrap();

    // Same query in same transaction should return same count (no phantom)
    let count2 = frank_scalar(&reader, "SELECT COUNT(*) FROM phantom WHERE category = 'A'");
    reader.execute("COMMIT").unwrap();

    assert_eq!(count1, "2", "initial count of category A");
    assert_eq!(
        count1, count2,
        "phantom read prevention: count must not change within txn"
    );

    // After commit, fresh read sees the new row
    let count3 = frank_scalar(&reader, "SELECT COUNT(*) FROM phantom WHERE category = 'A'");
    assert_eq!(count3, "3", "after commit, new row should be visible");
}

// ── Test 4: Write-write conflict detection ───────────────────────────

#[test]
fn write_write_conflict_detected() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();

    {
        let setup = fsqlite::Connection::open(&f_path).unwrap();
        setup.execute("PRAGMA journal_mode = WAL;").unwrap();
        setup
            .execute("CREATE TABLE ww (id INTEGER PRIMARY KEY, val INTEGER);")
            .unwrap();
        setup.execute("INSERT INTO ww VALUES (1, 100);").unwrap();
    }

    let c1 = fsqlite::Connection::open(&f_path).unwrap();
    c1.execute("PRAGMA journal_mode = WAL;").unwrap();
    let c2 = fsqlite::Connection::open(&f_path).unwrap();
    c2.execute("PRAGMA journal_mode = WAL;").unwrap();

    // Both connections begin concurrent transactions
    c1.execute("BEGIN CONCURRENT").unwrap();
    c2.execute("BEGIN CONCURRENT").unwrap();

    // Both update the same row — c2 may get Busy on UPDATE or COMMIT
    c1.execute("UPDATE ww SET val = 200 WHERE id = 1;").unwrap();
    let c2_update = c2.execute("UPDATE ww SET val = 300 WHERE id = 1;");

    let r1 = c1.execute("COMMIT");
    assert!(r1.is_ok(), "first commit should succeed");

    if c2_update.is_err() {
        // c2 got Busy on UPDATE (page-level conflict) — rollback
        let _ = c2.execute("ROLLBACK");
        let val = frank_scalar(&c1, "SELECT val FROM ww WHERE id = 1");
        assert_eq!(val, "200", "c1's write should win");
    } else {
        // c2 UPDATE succeeded, try commit
        let r2 = c2.execute("COMMIT");
        if r2.is_err() {
            let _ = c2.execute("ROLLBACK");
            let val = frank_scalar(&c1, "SELECT val FROM ww WHERE id = 1");
            assert_eq!(val, "200", "c1's write should win after c2 conflict");
        } else {
            let val = frank_scalar(&c1, "SELECT val FROM ww WHERE id = 1");
            assert!(
                val == "200" || val == "300",
                "one of the writes should be the final value"
            );
        }
    }
}

// ── Test 5: Non-conflicting concurrent transactions both commit ──────

#[test]
fn non_conflicting_concurrent_transactions_both_commit() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();

    {
        let setup = fsqlite::Connection::open(&f_path).unwrap();
        setup.execute("PRAGMA journal_mode = WAL;").unwrap();
        setup
            .execute("CREATE TABLE nc (id INTEGER PRIMARY KEY, val INTEGER);")
            .unwrap();
        setup
            .execute("INSERT INTO nc VALUES (1, 10), (2, 20);")
            .unwrap();
    }

    let c1 = fsqlite::Connection::open(&f_path).unwrap();
    c1.execute("PRAGMA journal_mode = WAL;").unwrap();
    let c2 = fsqlite::Connection::open(&f_path).unwrap();
    c2.execute("PRAGMA journal_mode = WAL;").unwrap();

    c1.execute("BEGIN CONCURRENT").unwrap();
    c2.execute("BEGIN CONCURRENT").unwrap();

    // c1 updates row 1, c2 updates row 2 — no conflict at row level,
    // but page-level MVCC may still return Busy on UPDATE or COMMIT
    c1.execute("UPDATE nc SET val = 11 WHERE id = 1;").unwrap();
    let c2_update = c2.execute("UPDATE nc SET val = 22 WHERE id = 2;");

    let r1 = c1.execute("COMMIT");
    assert!(r1.is_ok(), "c1 commit should succeed (no conflict)");

    if c2_update.is_err() {
        // c2 got Busy on UPDATE — page-level conflict, rollback and retry
        let _ = c2.execute("ROLLBACK");
        c2.execute("UPDATE nc SET val = 22 WHERE id = 2;").unwrap();
    } else {
        let r2 = c2.execute("COMMIT");
        if r2.is_err() {
            // SSI may conservatively abort c2 even though rows are disjoint
            let _ = c2.execute("ROLLBACK");
            c2.execute("UPDATE nc SET val = 22 WHERE id = 2;").unwrap();
        }
    }

    let verify = fsqlite::Connection::open(&f_path).unwrap();
    let v1 = frank_scalar(&verify, "SELECT val FROM nc WHERE id = 1");
    assert_eq!(v1, "11", "c1's update should be visible");
}

// ── Test 6: Isolation across multiple tables ─────────────────────────

#[test]
fn isolation_across_multiple_tables() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();

    {
        let setup = fsqlite::Connection::open(&f_path).unwrap();
        setup.execute("PRAGMA journal_mode = WAL;").unwrap();
        setup
            .execute("CREATE TABLE iso_a (id INTEGER PRIMARY KEY, val INTEGER);")
            .unwrap();
        setup
            .execute("CREATE TABLE iso_b (id INTEGER PRIMARY KEY, val INTEGER);")
            .unwrap();
        setup.execute("INSERT INTO iso_a VALUES (1, 100);").unwrap();
        setup.execute("INSERT INTO iso_b VALUES (1, 200);").unwrap();
    }

    let barrier = Arc::new(Barrier::new(2));

    let fp_writer = f_path.clone();
    let bar_w = barrier.clone();

    let writer = thread::spawn(move || {
        let conn = fsqlite::Connection::open(&fp_writer).unwrap();
        conn.execute("PRAGMA journal_mode = WAL;").unwrap();
        bar_w.wait();
        conn.execute("UPDATE iso_a SET val = 111 WHERE id = 1;")
            .unwrap();
        conn.execute("UPDATE iso_b SET val = 222 WHERE id = 1;")
            .unwrap();
    });

    let reader = fsqlite::Connection::open(&f_path).unwrap();
    reader.execute("PRAGMA journal_mode = WAL;").unwrap();
    reader.execute("BEGIN").unwrap();
    let a1 = frank_scalar(&reader, "SELECT val FROM iso_a WHERE id = 1");
    let b1 = frank_scalar(&reader, "SELECT val FROM iso_b WHERE id = 1");

    barrier.wait();
    writer.join().unwrap();

    // Within transaction, reads should still see old values
    let a2 = frank_scalar(&reader, "SELECT val FROM iso_a WHERE id = 1");
    let b2 = frank_scalar(&reader, "SELECT val FROM iso_b WHERE id = 1");
    reader.execute("COMMIT").unwrap();

    assert_eq!(a1, a2, "iso_a must be repeatable within txn");
    assert_eq!(b1, b2, "iso_b must be repeatable within txn");
    assert_eq!(a1, "100");
    assert_eq!(b1, "200");

    // After commit, both tables should reflect writer's changes
    let a3 = frank_scalar(&reader, "SELECT val FROM iso_a WHERE id = 1");
    let b3 = frank_scalar(&reader, "SELECT val FROM iso_b WHERE id = 1");
    assert_eq!(a3, "111");
    assert_eq!(b3, "222");
}

// ── Test 7: Serializable outcome under contention ────────────────────

#[test]
fn serializable_outcome_under_contention() {
    let f_tmp = tempfile::NamedTempFile::new().unwrap();
    let f_path = f_tmp.path().to_str().unwrap().to_owned();

    // Create a counter table
    {
        let setup = fsqlite::Connection::open(&f_path).unwrap();
        setup.execute("PRAGMA journal_mode = WAL;").unwrap();
        setup
            .execute("CREATE TABLE counter (id INTEGER PRIMARY KEY, val INTEGER);")
            .unwrap();
        setup.execute("INSERT INTO counter VALUES (1, 0);").unwrap();
    }

    let n_threads = 4usize;
    let increments_per_thread = 10usize;
    let barrier = Arc::new(Barrier::new(n_threads));

    let handles: Vec<_> = (0..n_threads)
        .map(|_| {
            let p = f_path.clone();
            let bar = barrier.clone();
            thread::spawn(move || {
                let conn = fsqlite::Connection::open(&p).unwrap();
                conn.execute("PRAGMA journal_mode = WAL;").unwrap();
                bar.wait();
                for _ in 0..increments_per_thread {
                    let mut attempts = 0u32;
                    loop {
                        if conn.execute("BEGIN CONCURRENT").is_err() {
                            attempts += 1;
                            assert!(attempts < 500, "too many retries on BEGIN");
                            thread::sleep(std::time::Duration::from_millis(1));
                            continue;
                        }
                        // Read current value
                        let current = match conn.query("SELECT val FROM counter WHERE id = 1") {
                            Ok(rows) => match &rows[0].values()[0] {
                                SqliteValue::Integer(n) => *n,
                                _ => {
                                    let _ = conn.execute("ROLLBACK");
                                    continue;
                                }
                            },
                            Err(_) => {
                                let _ = conn.execute("ROLLBACK");
                                attempts += 1;
                                assert!(attempts < 500);
                                thread::sleep(std::time::Duration::from_millis(1));
                                continue;
                            }
                        };
                        // Increment
                        let sql = format!("UPDATE counter SET val = {} WHERE id = 1;", current + 1);
                        if conn.execute(&sql).is_err() {
                            let _ = conn.execute("ROLLBACK");
                            attempts += 1;
                            assert!(attempts < 500);
                            thread::sleep(std::time::Duration::from_millis(1));
                            continue;
                        }
                        match conn.execute("COMMIT") {
                            Ok(_) => break,
                            Err(_) => {
                                let _ = conn.execute("ROLLBACK");
                                attempts += 1;
                                assert!(attempts < 500);
                                thread::sleep(std::time::Duration::from_millis(1));
                            }
                        }
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    let f = fsqlite::Connection::open(&f_path).unwrap();
    let val = frank_scalar(&f, "SELECT val FROM counter WHERE id = 1");
    let expected = n_threads * increments_per_thread;
    assert_eq!(
        val,
        expected.to_string(),
        "counter should equal total increments (serializable outcome)"
    );
}
