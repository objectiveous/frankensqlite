use fsqlite_ext_fts5::{Fts5HighlightFunc, Fts5SnippetFunc, Fts5Table};
use fsqlite_func::ScalarFunction;
use fsqlite_func::vtab::VirtualTable;
use fsqlite_types::cx::Cx;
use fsqlite_types::value::{SmallText, SqliteValue};
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

struct TokenizerCase {
    name: &'static str,
    options: &'static [&'static str],
    docs: &'static [Doc],
    query: &'static str,
}

#[derive(Clone, Copy)]
struct RankingCase {
    name: &'static str,
    query: &'static str,
}

#[derive(Clone, Copy)]
struct AuxiliaryCase {
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

const BM25_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Dense rust",
        body: "rust rust rust rust rust search",
    },
    Doc {
        rowid: 2,
        title: "Short rust",
        body: "rust rust search",
    },
    Doc {
        rowid: 3,
        title: "Long rust",
        body: "rust search compatibility writers pages tokens tokens tokens tokens tokens",
    },
    Doc {
        rowid: 4,
        title: "SQLite dense",
        body: "sqlite sqlite sqlite search",
    },
    Doc {
        rowid: 5,
        title: "Plain note",
        body: "plain cooking note",
    },
];

const UNICODE61_DIACRITIC_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Cafe accents",
        body: "café crème résumé",
    },
    Doc {
        rowid: 2,
        title: "Plain cafe",
        body: "cafe creme resume",
    },
    Doc {
        rowid: 3,
        title: "Tea notes",
        body: "oolong sencha",
    },
];

const PORTER_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Running tests",
        body: "I am running the reliable search tests",
    },
    Doc {
        rowid: 2,
        title: "Run book",
        body: "run fast and test often",
    },
    Doc {
        rowid: 3,
        title: "Runner notes",
        body: "runner teams write docs",
    },
];

const TRIGRAM_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Upper token",
        body: "ABC sigma",
    },
    Doc {
        rowid: 2,
        title: "Embedded token",
        body: "zabc tail",
    },
    Doc {
        rowid: 3,
        title: "Too short",
        body: "ab",
    },
];

const UNICODE61_CODE_TOKEN_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Code symbols",
        body: "Call my_function in AuthController.ts for endpoint/v1",
    },
    Doc {
        rowid: 2,
        title: "Split code words",
        body: "Call my function in AuthController ts for endpoint v1",
    },
    Doc {
        rowid: 3,
        title: "Other identifier",
        body: "Use handler_name in Router.js",
    },
];

const UNICODE61_SEPARATOR_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Delimited tags",
        body: "alpha.beta_gamma",
    },
    Doc {
        rowid: 2,
        title: "Undelimited tag",
        body: "alphabetagamma",
    },
    Doc {
        rowid: 3,
        title: "Only alpha",
        body: "alpha",
    },
];

const UNICODE61_DIACRITIC_OPTIONS: &[&str] = &["tokenize='unicode61 remove_diacritics 2'"];
const UNICODE61_CODE_TOKEN_OPTIONS: &[&str] = &[r#"tokenize="unicode61 tokenchars '-_./:@#$%'""#];
const UNICODE61_SEPARATOR_OPTIONS: &[&str] = &[r#"tokenize="unicode61 separators '_.'""#];
const UNINDEXED_COLUMN_SPECS: &[&str] = &["title", "body UNINDEXED"];
const CONTENTLESS_OPTIONS: &[&str] = &["content=''"];
const PREFIX_INDEX_OPTIONS: &[&str] = &["prefix='2 3'"];
const PORTER_OPTIONS: &[&str] = &["tokenize='porter'"];
const TRIGRAM_OPTIONS: &[&str] = &["tokenize='trigram'"];

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

const COLUMN_FILTER_INITIAL_CASES: &[MatchCase] = &[
    MatchCase {
        name: "braced multi-column filter",
        query: "{title body}:sqlite",
    },
    MatchCase {
        name: "braced phrase filter",
        query: r#"{title body}:"rust sqlite""#,
    },
    MatchCase {
        name: "negative title filter",
        query: "-title:rust",
    },
    MatchCase {
        name: "initial token across indexed columns",
        query: "^rust",
    },
    MatchCase {
        name: "initial token inside title filter",
        query: "title:^rust",
    },
    MatchCase {
        name: "initial phrase",
        query: r#"^"rust sqlite""#,
    },
];

const UNINDEXED_COLUMN_CASES: &[MatchCase] = &[
    MatchCase {
        name: "unindexed column filter",
        query: "body:search",
    },
    MatchCase {
        name: "indexed title filter",
        query: "title:rust",
    },
    MatchCase {
        name: "implicit union excludes unindexed body",
        query: "search",
    },
    MatchCase {
        name: "braced filter excludes unindexed body hits",
        query: "{title body}:sqlite",
    },
    MatchCase {
        name: "unindexed phrase filter",
        query: r#"body:"full text""#,
    },
];

const CONTENTLESS_CASES: &[MatchCase] = &[
    MatchCase {
        name: "contentless title term",
        query: "rust",
    },
    MatchCase {
        name: "contentless body term",
        query: "search",
    },
    MatchCase {
        name: "contentless title filter",
        query: "title:sqlite",
    },
    MatchCase {
        name: "contentless body filter",
        query: "body:writers",
    },
    MatchCase {
        name: "contentless prefix",
        query: "compat*",
    },
];

const PREFIX_INDEX_CASES: &[MatchCase] = &[
    MatchCase {
        name: "two-character prefix",
        query: "se*",
    },
    MatchCase {
        name: "three-character prefix",
        query: "rus*",
    },
    MatchCase {
        name: "phrase final prefix",
        query: "full + tex*",
    },
    MatchCase {
        name: "column prefix",
        query: "title:sq*",
    },
    MatchCase {
        name: "near prefix",
        query: "NEAR(rust compat*, 4)",
    },
];

const INVALID_QUERY_CASES: &[MatchCase] = &[
    MatchCase {
        name: "empty query",
        query: "",
    },
    MatchCase {
        name: "unclosed phrase",
        query: r#""rust"#,
    },
    MatchCase {
        name: "unbalanced parenthesis",
        query: "(rust",
    },
    MatchCase {
        name: "unary not",
        query: "NOT rust",
    },
    MatchCase {
        name: "unknown column filter",
        query: "summary:rust",
    },
];

const BM25_CASES: &[RankingCase] = &[
    RankingCase {
        name: "single term frequency",
        query: "rust",
    },
    RankingCase {
        name: "multi term frequency",
        query: "rust search",
    },
];

const AUXILIARY_CASES: &[AuxiliaryCase] = &[
    AuxiliaryCase {
        name: "single term",
        query: "rust",
    },
    AuxiliaryCase {
        name: "phrase span",
        query: r#""full text""#,
    },
    AuxiliaryCase {
        name: "prefix term",
        query: "search*",
    },
    AuxiliaryCase {
        name: "boolean terms",
        query: "rust AND sqlite",
    },
];

const TOKENIZER_CASES: &[TokenizerCase] = &[
    TokenizerCase {
        name: "unicode61 remove_diacritics",
        options: UNICODE61_DIACRITIC_OPTIONS,
        docs: UNICODE61_DIACRITIC_DOCS,
        query: "cafe",
    },
    TokenizerCase {
        name: "porter stemming",
        options: PORTER_OPTIONS,
        docs: PORTER_DOCS,
        query: "run",
    },
    TokenizerCase {
        name: "trigram case folding",
        options: TRIGRAM_OPTIONS,
        docs: TRIGRAM_DOCS,
        query: "abc",
    },
    TokenizerCase {
        name: "unicode61 tokenchars keep code symbols",
        options: UNICODE61_CODE_TOKEN_OPTIONS,
        docs: UNICODE61_CODE_TOKEN_DOCS,
        query: r#""AuthController.ts""#,
    },
    TokenizerCase {
        name: "unicode61 separators split punctuation",
        options: UNICODE61_SEPARATOR_OPTIONS,
        docs: UNICODE61_SEPARATOR_DOCS,
        query: r#""beta gamma""#,
    },
];

struct Fts5ConformanceHarness {
    franken: Fts5Table,
    sqlite: Connection,
}

impl Fts5ConformanceHarness {
    fn new(options: &[&str]) -> Self {
        Self::with_docs(options, DOCS)
    }

    fn with_docs(options: &[&str], docs: &[Doc]) -> Self {
        Self::with_schema(&["title", "body"], options, docs)
    }

    fn with_schema(column_specs: &[&str], options: &[&str], docs: &[Doc]) -> Self {
        let mut args = vec!["fts5", "main", "docs"];
        args.extend_from_slice(column_specs);
        args.extend_from_slice(options);

        let cx = Cx::new();
        let mut franken = Fts5Table::connect(&cx, &args).expect("connect FrankenSQLite FTS5 table");
        for doc in docs {
            franken.insert_document(doc.rowid, &[doc.title.to_owned(), doc.body.to_owned()]);
        }

        let sqlite = Connection::open_in_memory().expect("open rusqlite in-memory database");
        let columns = column_specs.join(", ");
        let sql_options = if options.is_empty() {
            String::new()
        } else {
            format!(", {}", options.join(", "))
        };
        sqlite
            .execute_batch(&format!(
                "CREATE VIRTUAL TABLE docs USING fts5({columns}{sql_options});"
            ))
            .expect("create rusqlite FTS5 table");
        for doc in docs {
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

    fn franken_match_is_error(&self, query: &str) -> bool {
        self.franken.search(query).is_err()
    }

    fn sqlite_match_is_error(&self, query: &str) -> bool {
        let mut stmt = self
            .sqlite
            .prepare("SELECT rowid FROM docs WHERE docs MATCH ?1 ORDER BY rowid")
            .expect("prepare rusqlite FTS5 MATCH error query");
        let mapped = stmt.query_map([query], |row| row.get::<_, i64>(0));
        match mapped {
            Ok(rows) => rows.collect::<std::result::Result<Vec<_>, _>>().is_err(),
            Err(_) => true,
        }
    }

    fn franken_ranked_rowids(&self, query: &str) -> Vec<i64> {
        let mut ranked = self
            .franken
            .search(query)
            .expect("FrankenSQLite FTS5 ranked query");
        ranked.sort_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        ranked.into_iter().map(|(rowid, _score)| rowid).collect()
    }

    fn sqlite_bm25_rowids(&self, query: &str) -> Vec<i64> {
        let mut stmt = self
            .sqlite
            .prepare("SELECT rowid FROM docs WHERE docs MATCH ?1 ORDER BY bm25(docs), rowid")
            .expect("prepare rusqlite FTS5 BM25 query");
        stmt.query_map([query], |row| row.get::<_, i64>(0))
            .expect("query rusqlite FTS5 BM25 rowids")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("read rusqlite FTS5 BM25 rowids")
    }

    fn franken_highlight_body_rows(&self, query: &str) -> Vec<(i64, String)> {
        let func = Fts5HighlightFunc;
        let mut rows: Vec<(i64, String)> = self
            .franken
            .search_rows(query)
            .expect("FrankenSQLite FTS5 rows for highlight")
            .into_iter()
            .map(|(rowid, _score, columns)| {
                let highlighted = func
                    .invoke(&[
                        SqliteValue::Text(SmallText::from_string(columns[1].clone())),
                        SqliteValue::Text(SmallText::from_string(query.to_owned())),
                        SqliteValue::Text(SmallText::from_string("<b>".to_owned())),
                        SqliteValue::Text(SmallText::from_string("</b>".to_owned())),
                    ])
                    .expect("FrankenSQLite FTS5 highlight");
                (rowid, highlighted.to_text())
            })
            .collect();
        rows.sort_by_key(|(rowid, _text)| *rowid);
        rows
    }

    fn sqlite_highlight_body_rows(&self, query: &str) -> Vec<(i64, String)> {
        let mut stmt = self
            .sqlite
            .prepare(
                "SELECT rowid, highlight(docs, 1, '<b>', '</b>') \
                 FROM docs WHERE docs MATCH ?1 ORDER BY rowid",
            )
            .expect("prepare rusqlite FTS5 highlight query");
        stmt.query_map([query], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .expect("query rusqlite FTS5 highlight rows")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("read rusqlite FTS5 highlight rows")
    }

    fn franken_snippet_body_rows(&self, query: &str) -> Vec<(i64, String)> {
        let func = Fts5SnippetFunc;
        let mut rows: Vec<(i64, String)> = self
            .franken
            .search_rows(query)
            .expect("FrankenSQLite FTS5 rows for snippet")
            .into_iter()
            .map(|(rowid, _score, columns)| {
                let snippet = func
                    .invoke(&[
                        SqliteValue::Text(SmallText::from_string(columns[1].clone())),
                        SqliteValue::Text(SmallText::from_string(query.to_owned())),
                        SqliteValue::Text(SmallText::from_string("[".to_owned())),
                        SqliteValue::Text(SmallText::from_string("]".to_owned())),
                        SqliteValue::Text(SmallText::from_string("...".to_owned())),
                        SqliteValue::Integer(64),
                    ])
                    .expect("FrankenSQLite FTS5 snippet");
                (rowid, snippet.to_text())
            })
            .collect();
        rows.sort_by_key(|(rowid, _text)| *rowid);
        rows
    }

    fn sqlite_snippet_body_rows(&self, query: &str) -> Vec<(i64, String)> {
        let mut stmt = self
            .sqlite
            .prepare(
                "SELECT rowid, snippet(docs, 1, '[', ']', '...', 64) \
                 FROM docs WHERE docs MATCH ?1 ORDER BY rowid",
            )
            .expect("prepare rusqlite FTS5 snippet query");
        stmt.query_map([query], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .expect("query rusqlite FTS5 snippet rows")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("read rusqlite FTS5 snippet rows")
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

#[test]
fn column_filter_and_initial_token_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in COLUMN_FILTER_INITIAL_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "column-filter/initial-token conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn unindexed_columns_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_schema(UNINDEXED_COLUMN_SPECS, &[], DOCS);

    for case in UNINDEXED_COLUMN_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "UNINDEXED column conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn contentless_tables_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(CONTENTLESS_OPTIONS, DOCS);

    for case in CONTENTLESS_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "contentless table conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn prefix_index_options_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(PREFIX_INDEX_OPTIONS, DOCS);

    for case in PREFIX_INDEX_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "prefix-index option conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn invalid_match_queries_error_like_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in INVALID_QUERY_CASES {
        assert_eq!(
            harness.franken_match_is_error(case.query),
            harness.sqlite_match_is_error(case.query),
            "invalid MATCH query conformance failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn tokenizer_queries_match_rusqlite_reference() {
    for case in TOKENIZER_CASES {
        let harness = Fts5ConformanceHarness::with_docs(case.options, case.docs);
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "tokenizer conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn bm25_ranking_matches_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(&[], BM25_DOCS);

    for case in BM25_CASES {
        assert_eq!(
            harness.franken_ranked_rowids(case.query),
            harness.sqlite_bm25_rowids(case.query),
            "BM25 conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn highlight_and_snippet_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in AUXILIARY_CASES {
        assert_eq!(
            harness.franken_highlight_body_rows(case.query),
            harness.sqlite_highlight_body_rows(case.query),
            "highlight conformance case failed: {} ({})",
            case.name,
            case.query
        );
        assert_eq!(
            harness.franken_snippet_body_rows(case.query),
            harness.sqlite_snippet_body_rows(case.query),
            "snippet conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}
