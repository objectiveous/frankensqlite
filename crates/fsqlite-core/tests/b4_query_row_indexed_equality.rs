//! bd-wwqen.4: Proof tests for SimpleIndexedEqualityLookup query_row fast path.
//!
//! These tests verify BOTH correctness (row values) AND that the direct
//! MemDB fast path fires (counter assertions). A regression that silently
//! falls through to VDBE would fail the counter checks.
//!
//! Run:
//!   cargo test -p fsqlite-core --test b4_query_row_indexed_equality \
//!     -- --test-threads=1 --nocapture

use fsqlite_core::connection::{
    Connection, hot_path_profile_enabled, hot_path_profile_snapshot, reset_hot_path_profile,
    set_hot_path_profile_enabled,
};
use fsqlite_types::SqliteValue;
use std::sync::{Mutex, MutexGuard};

static B4_PROFILE_LOCK: Mutex<()> = Mutex::new(());

struct B4ProfileGuard {
    _lock: MutexGuard<'static, ()>,
    previous_enabled: bool,
}

impl B4ProfileGuard {
    fn new() -> Self {
        let lock = B4_PROFILE_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let previous_enabled = hot_path_profile_enabled();
        set_hot_path_profile_enabled(true);
        reset_hot_path_profile();
        Self {
            _lock: lock,
            previous_enabled,
        }
    }
}

impl Drop for B4ProfileGuard {
    fn drop(&mut self) {
        reset_hot_path_profile();
        set_hot_path_profile_enabled(self.previous_enabled);
    }
}

fn setup_indexed_table(conn: &Connection) {
    conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT, category TEXT)")
        .unwrap();
    conn.execute("CREATE INDEX idx_items_category ON items(category)")
        .unwrap();
    conn.execute("INSERT INTO items VALUES (1, 'apple', 'fruit')")
        .unwrap();
    conn.execute("INSERT INTO items VALUES (2, 'banana', 'fruit')")
        .unwrap();
    conn.execute("INSERT INTO items VALUES (3, 'carrot', 'vegetable')")
        .unwrap();
    conn.execute("INSERT INTO items VALUES (4, 'date', 'fruit')")
        .unwrap();
    conn.execute("INSERT INTO items VALUES (5, 'eggplant', 'vegetable')")
        .unwrap();
}

/// B4.1: Basic indexed equality lookup returns correct rows AND fires fast path.
#[test]
fn test_query_row_indexed_equality_basic() {
    let _guard = B4ProfileGuard::new();
    let conn = Connection::open(":memory:").unwrap();
    setup_indexed_table(&conn);

    // Warm the prepared statement cache.
    let _ = conn
        .query_with_params(
            "SELECT id, name FROM items WHERE category = ?1",
            &[SqliteValue::Text("fruit".into())],
        )
        .unwrap();
    reset_hot_path_profile();

    let before = hot_path_profile_snapshot();
    let rows = conn
        .query_with_params(
            "SELECT id, name FROM items WHERE category = ?1",
            &[SqliteValue::Text("fruit".into())],
        )
        .unwrap();
    let after = hot_path_profile_snapshot();

    assert_eq!(rows.len(), 3, "should find 3 fruit rows");
    let ids: Vec<i64> = rows
        .iter()
        .filter_map(|r| r.values.first().and_then(|v| v.as_integer()))
        .collect();
    assert!(ids.contains(&1), "apple should be found");
    assert!(ids.contains(&2), "banana should be found");
    assert!(ids.contains(&4), "date should be found");

    let hits_delta = after
        .direct_indexed_equality_query_hits
        .saturating_sub(before.direct_indexed_equality_query_hits);
    eprintln!("[B4.1] direct_indexed_equality_query_hits delta: {hits_delta}");
    assert!(
        hits_delta >= 1,
        "indexed equality fast path should fire: hits_delta={hits_delta}"
    );
}

/// B4.2: No-match returns empty AND still fires fast path.
#[test]
fn test_query_row_indexed_equality_no_match() {
    let _guard = B4ProfileGuard::new();
    let conn = Connection::open(":memory:").unwrap();
    setup_indexed_table(&conn);

    let _ = conn
        .query_with_params(
            "SELECT id, name FROM items WHERE category = ?1",
            &[SqliteValue::Text("grain".into())],
        )
        .unwrap();
    reset_hot_path_profile();

    let before = hot_path_profile_snapshot();
    let rows = conn
        .query_with_params(
            "SELECT id, name FROM items WHERE category = ?1",
            &[SqliteValue::Text("grain".into())],
        )
        .unwrap();
    let after = hot_path_profile_snapshot();

    assert_eq!(rows.len(), 0, "no grains should exist");

    let hits_delta = after
        .direct_indexed_equality_query_hits
        .saturating_sub(before.direct_indexed_equality_query_hits);
    eprintln!("[B4.2] no-match: direct_indexed_equality_query_hits delta: {hits_delta}");
    assert!(
        hits_delta >= 1,
        "fast path should fire even for no-match: hits_delta={hits_delta}"
    );
}

/// B4.3: NULL parameter returns empty AND fires fast path (NULL → NoRows).
#[test]
fn test_query_row_indexed_equality_null_param() {
    let _guard = B4ProfileGuard::new();
    let conn = Connection::open(":memory:").unwrap();
    setup_indexed_table(&conn);

    let _ = conn
        .query_with_params(
            "SELECT id, name FROM items WHERE category = ?1",
            &[SqliteValue::Null],
        )
        .unwrap();
    reset_hot_path_profile();

    let before = hot_path_profile_snapshot();
    let rows = conn
        .query_with_params(
            "SELECT id, name FROM items WHERE category = ?1",
            &[SqliteValue::Null],
        )
        .unwrap();
    let after = hot_path_profile_snapshot();

    assert_eq!(rows.len(), 0, "NULL equality should match nothing");

    let hits_delta = after
        .direct_indexed_equality_query_hits
        .saturating_sub(before.direct_indexed_equality_query_hits);
    eprintln!("[B4.3] null-param: direct_indexed_equality_query_hits delta: {hits_delta}");
    assert!(
        hits_delta >= 1,
        "fast path should fire for NULL param (early NoRows): hits_delta={hits_delta}"
    );
}

/// B4.4: Read-after-write returns the new row.
#[test]
fn test_query_row_indexed_equality_read_after_write() {
    let conn = Connection::open(":memory:").unwrap();
    setup_indexed_table(&conn);

    conn.execute("INSERT INTO items VALUES (6, 'fig', 'fruit')")
        .unwrap();

    let rows = conn
        .query_with_params(
            "SELECT id, name FROM items WHERE category = ?1",
            &[SqliteValue::Text("fruit".into())],
        )
        .unwrap();

    assert_eq!(rows.len(), 4, "should find 4 fruit rows after insert");
    let ids: Vec<i64> = rows
        .iter()
        .filter_map(|r| r.values.first().and_then(|v| v.as_integer()))
        .collect();
    assert!(ids.contains(&6), "fig should be found after insert");
}

/// B4.5: Prepared statement reuse with different params fires fast path each time.
#[test]
fn test_query_row_indexed_equality_prepared_reuse() {
    let _guard = B4ProfileGuard::new();
    let conn = Connection::open(":memory:").unwrap();
    setup_indexed_table(&conn);

    let stmt = conn
        .prepare("SELECT id, name FROM items WHERE category = ?1")
        .unwrap();

    // Warm.
    let _ = stmt
        .query_with_params(&[SqliteValue::Text("fruit".into())])
        .unwrap();
    reset_hot_path_profile();

    let before = hot_path_profile_snapshot();
    let rows_fruit = stmt
        .query_with_params(&[SqliteValue::Text("fruit".into())])
        .unwrap();
    assert_eq!(rows_fruit.len(), 3, "should find 3 fruits");

    let rows_veg = stmt
        .query_with_params(&[SqliteValue::Text("vegetable".into())])
        .unwrap();
    assert_eq!(rows_veg.len(), 2, "should find 2 vegetables");

    let rows_none = stmt
        .query_with_params(&[SqliteValue::Text("meat".into())])
        .unwrap();
    assert_eq!(rows_none.len(), 0, "should find no meat");
    let after = hot_path_profile_snapshot();

    let hits_delta = after
        .direct_indexed_equality_query_hits
        .saturating_sub(before.direct_indexed_equality_query_hits);
    eprintln!("[B4.5] prepared reuse: direct_indexed_equality_query_hits delta: {hits_delta}");
    assert!(
        hits_delta >= 3,
        "fast path should fire for all 3 param values: hits_delta={hits_delta}"
    );
}

/// B4.6: Unique index returns exactly one row.
#[test]
fn test_query_row_indexed_equality_unique_index() {
    let _guard = B4ProfileGuard::new();
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT UNIQUE, name TEXT)")
        .unwrap();
    conn.execute("CREATE UNIQUE INDEX idx_users_email ON users(email)")
        .unwrap();
    conn.execute("INSERT INTO users VALUES (1, 'a@b.com', 'Alice')")
        .unwrap();
    conn.execute("INSERT INTO users VALUES (2, 'c@d.com', 'Bob')")
        .unwrap();

    // Warm.
    let _ = conn
        .query_with_params(
            "SELECT id, name FROM users WHERE email = ?1",
            &[SqliteValue::Text("a@b.com".into())],
        )
        .unwrap();
    reset_hot_path_profile();

    let before = hot_path_profile_snapshot();
    let rows = conn
        .query_with_params(
            "SELECT id, name FROM users WHERE email = ?1",
            &[SqliteValue::Text("a@b.com".into())],
        )
        .unwrap();
    let after = hot_path_profile_snapshot();

    assert_eq!(rows.len(), 1, "unique index should return exactly one row");
    assert_eq!(
        rows[0].values[1],
        SqliteValue::Text("Alice".into()),
        "should return Alice"
    );

    let hits_delta = after
        .direct_indexed_equality_query_hits
        .saturating_sub(before.direct_indexed_equality_query_hits);
    eprintln!("[B4.6] unique index: direct_indexed_equality_query_hits delta: {hits_delta}");
    assert!(
        hits_delta >= 1,
        "fast path should fire for unique index lookup: hits_delta={hits_delta}"
    );
}

/// B4.7: Integer column indexed equality.
#[test]
fn test_query_row_indexed_equality_integer_column() {
    let _guard = B4ProfileGuard::new();
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE scores (id INTEGER PRIMARY KEY, player_id INTEGER, score INTEGER)")
        .unwrap();
    conn.execute("CREATE INDEX idx_scores_player ON scores(player_id)")
        .unwrap();
    conn.execute("INSERT INTO scores VALUES (1, 10, 100)")
        .unwrap();
    conn.execute("INSERT INTO scores VALUES (2, 20, 200)")
        .unwrap();
    conn.execute("INSERT INTO scores VALUES (3, 10, 150)")
        .unwrap();

    // Warm.
    let _ = conn
        .query_with_params(
            "SELECT id, score FROM scores WHERE player_id = ?1",
            &[SqliteValue::Integer(10)],
        )
        .unwrap();
    reset_hot_path_profile();

    let before = hot_path_profile_snapshot();
    let rows = conn
        .query_with_params(
            "SELECT id, score FROM scores WHERE player_id = ?1",
            &[SqliteValue::Integer(10)],
        )
        .unwrap();
    let after = hot_path_profile_snapshot();

    assert_eq!(rows.len(), 2, "player 10 should have 2 scores");

    let hits_delta = after
        .direct_indexed_equality_query_hits
        .saturating_sub(before.direct_indexed_equality_query_hits);
    eprintln!("[B4.7] integer column: direct_indexed_equality_query_hits delta: {hits_delta}");
    assert!(
        hits_delta >= 1,
        "fast path should fire for integer column lookup: hits_delta={hits_delta}"
    );
}
