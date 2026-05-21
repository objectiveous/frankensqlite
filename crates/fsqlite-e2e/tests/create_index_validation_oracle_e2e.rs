//! bd-qr5w0 — Oracle-parity e2e: CREATE INDEX validation vs rusqlite.
//!
//! index_features_oracle covers building and using indexes. This pins the
//! CREATE INDEX validation errors: indexing a non-existent column ("no such
//! column"), a non-existent table ("no such table"), and creating an index whose
//! name already exists ("index ... already exists"). The valid contrasts — a
//! plain index, a multi-column index, and `IF NOT EXISTS` on an existing index —
//! must succeed. Only statement success/failure is compared; the base table is
//! recreated per case.

use fsqlite::Connection;

/// Run `setup` (assumed to succeed on both), then `test`, returning a mismatch
/// description if the engines disagree on success/failure of `test`.
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
        (Err(e), Ok(())) => Some(format!("FRANK_ERR / CSQL_OK: `{test}`\n  frank: ERROR({e})")),
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

const BASE: &[&str] = &["CREATE TABLE t (a, b)"];

#[test]
fn create_index_valid_ok() {
    check(
        &[
            (BASE, "CREATE INDEX i ON t(a)"),
            (BASE, "CREATE INDEX i2 ON t(a, b)"),
            (
                &["CREATE TABLE t (a, b)", "CREATE INDEX i ON t(a)"],
                "CREATE INDEX IF NOT EXISTS i ON t(a)", // already exists, but IF NOT EXISTS -> ok
            ),
        ],
        "create_index_valid_ok",
    );
}

#[test]
fn create_index_errors() {
    check(
        &[
            (BASE, "CREATE INDEX i ON t(nope)"), // no such column
            (BASE, "CREATE INDEX i ON nope(a)"), // no such table
            (
                &["CREATE TABLE t (a, b)", "CREATE INDEX i ON t(a)"],
                "CREATE INDEX i ON t(b)", // duplicate index name -> error
            ),
        ],
        "create_index_errors",
    );
}
