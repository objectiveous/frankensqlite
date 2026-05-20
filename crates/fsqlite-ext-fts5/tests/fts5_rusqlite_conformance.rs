use fsqlite_ext_fts5::Fts5Table;
use fsqlite_func::vtab::VirtualTable;
use fsqlite_types::cx::Cx;
use rusqlite::Connection;

#[derive(Clone, Copy)]
struct Doc {
    rowid: i64,
    title: &'static str,
    body: &'static str,
}

#[derive(Clone, Copy)]
struct MatchCase {
    name: &'static str,
    query: &'static str,
}

const DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Rust search",
        body: "Rust language empowers fast search systems",
    },
    Doc {
        rowid: 2,
        title: "SQLite FTS",
        body: "SQLite full text search supports phrase and prefix queries",
    },
    Doc {
        rowid: 3,
        title: "Rust SQLite",
        body: "FrankenSQLite aims for SQLite compatibility with concurrent writers",
    },
    Doc {
        rowid: 4,
        title: "Cooking notes",
        body: "Bread and butter with fresh herbs",
    },
    Doc {
        rowid: 5,
        title: "Search cookbook",
        body: "Rust and SQLite examples for reliable search tests",
    },
];

const MATCH_CASES: &[MatchCase] = &[
    MatchCase {
        name: "single term",
        query: "rust",
    },
    MatchCase {
        name: "implicit column union",
        query: "sqlite",
    },
    MatchCase {
        name: "boolean and",
        query: "rust AND sqlite",
    },
    MatchCase {
        name: "boolean or",
        query: "rust OR bread",
    },
    MatchCase {
        name: "binary not",
        query: "sqlite NOT cooking",
    },
    MatchCase {
        name: "title column filter",
        query: "title:rust",
    },
    MatchCase {
        name: "body column filter",
        query: "body:search",
    },
];

const PHRASE_PREFIX_NEAR_CASES: &[MatchCase] = &[
    MatchCase {
        name: "quoted phrase",
        query: r#""full text""#,
    },
    MatchCase {
        name: "quoted phrase across title terms",
        query: r#""rust sqlite""#,
    },
    MatchCase {
        name: "term prefix",
        query: "search*",
    },
    MatchCase {
        name: "phrase final prefix",
        query: "full + tex*",
    },
    MatchCase {
        name: "near terms",
        query: "NEAR(rust sqlite, 3)",
    },
    MatchCase {
        name: "near phrase and term",
        query: r#"NEAR("full text" prefix, 3)"#,
    },
];

struct Fts5ConformanceHarness {
    franken: Fts5Table,
    sqlite: Connection,
}

impl Fts5ConformanceHarness {
    fn new(options: &[&str]) -> Self {
        let mut args = vec!["fts5", "main", "docs", "title", "body"];
        args.extend_from_slice(options);

        let cx = Cx::new();
        let mut franken = Fts5Table::connect(&cx, &args).expect("connect FrankenSQLite FTS5 table");
        for doc in DOCS {
            franken.insert_document(doc.rowid, &[doc.title.to_owned(), doc.body.to_owned()]);
        }

        let sqlite = Connection::open_in_memory().expect("open rusqlite in-memory database");
        let sql_options = if options.is_empty() {
            String::new()
        } else {
            format!(", {}", options.join(", "))
        };
        sqlite
            .execute_batch(&format!(
                "CREATE VIRTUAL TABLE docs USING fts5(title, body{sql_options});"
            ))
            .expect("create rusqlite FTS5 table");
        for doc in DOCS {
            sqlite
                .execute(
                    "INSERT INTO docs(rowid, title, body) VALUES (?1, ?2, ?3)",
                    (doc.rowid, doc.title, doc.body),
                )
                .expect("insert rusqlite FTS5 row");
        }

        Self { franken, sqlite }
    }

    fn franken_match_rowids(&self, query: &str) -> Vec<i64> {
        let mut rowids: Vec<i64> = self
            .franken
            .search(query)
            .expect("FrankenSQLite FTS5 MATCH query")
            .into_iter()
            .map(|(rowid, _score)| rowid)
            .collect();
        rowids.sort_unstable();
        rowids
    }

    fn sqlite_match_rowids(&self, query: &str) -> Vec<i64> {
        let mut stmt = self
            .sqlite
            .prepare("SELECT rowid FROM docs WHERE docs MATCH ?1 ORDER BY rowid")
            .expect("prepare rusqlite FTS5 MATCH query");
        stmt.query_map([query], |row| row.get::<_, i64>(0))
            .expect("query rusqlite FTS5 rowids")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("read rusqlite FTS5 rowids")
    }
}

#[test]
fn match_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in MATCH_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "MATCH conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn phrase_prefix_near_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in PHRASE_PREFIX_NEAR_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "phrase/prefix/NEAR conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}
