//! bd-avjdi — Oracle-parity e2e: DROP INDEX/VIEW/TRIGGER nonexistent rejection.
//!
//! drop_semantics_oracle covers IF EXISTS on missing targets (silent), and the
//! `DROP TABLE nope` (no IF EXISTS) error. SQLite raises "no such ..." for the
//! other object kinds too: `DROP INDEX nope`, `DROP VIEW nope`, `DROP TRIGGER
//! nope`. This pins those three (frank already gets DROP TABLE right). Only
//! statement success/failure is compared, on a fresh connection per case.

use fsqlite::Connection;

fn ddl_case(setup: &[&str], test: &str) -> Option<String> {
    let f = Connection::open(":memory:").expect("open frank");
    let r = rusqlite::Connection::open_in_memory().expect("open rusqlite");
    for s in setup {
        f.execute(s).unwrap();
        r.execute_batch(s).unwrap();
    }
    let fe = f.execute(test);
    let re = r.execute_batch(test);
    match (&fe, &re) {
        (Ok(_), Ok(())) | (Err(_), Err(_)) => None,
        (Ok(_), Err(e)) => Some(format!("FRANK_OK / CSQL_ERR: `{test}`\n  csql: ERROR({e})")),
        (Err(e), Ok(())) => Some(format!(
            "FRANK_ERR / CSQL_OK: `{test}`\n  frank: ERROR({e})"
        )),
    }
}

fn check(cases: &[(&[&str], &str)], label: &str) {
    let mismatches: Vec<String> = cases
        .iter()
        .filter_map(|(setup, test)| ddl_case(setup, test))
        .collect();
    assert!(
        mismatches.is_empty(),
        "{label}: {} mismatch(es)\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

const SETUP: &[&str] = &["CREATE TABLE t (a INTEGER)"];

#[test]
fn drop_if_exists_on_missing_silent() {
    check(
        &[
            (SETUP, "DROP INDEX IF EXISTS no_such_index"),
            (SETUP, "DROP VIEW IF EXISTS no_such_view"),
            (SETUP, "DROP TRIGGER IF EXISTS no_such_trigger"),
        ],
        "drop_if_exists_on_missing_silent",
    );
}

#[test]
fn drop_nonexistent_without_if_exists_rejected() {
    check(
        &[
            (SETUP, "DROP INDEX nope"),
            (SETUP, "DROP VIEW nope"),
            (SETUP, "DROP TRIGGER nope"),
        ],
        "drop_nonexistent_without_if_exists_rejected",
    );
}
