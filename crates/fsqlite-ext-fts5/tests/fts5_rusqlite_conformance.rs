use fsqlite_ext_fts5::{Fts5HighlightFunc, Fts5SnippetFunc, Fts5Table};
use fsqlite_func::ScalarFunction;
use fsqlite_func::vtab::{ColumnContext, VirtualTable, VirtualTableCursor};
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

#[derive(Clone, Copy)]
struct MultiMatchCase {
    name: &'static str,
    left_query: &'static str,
    right_query: &'static str,
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
struct WeightedRankingCase {
    name: &'static str,
    query: &'static str,
    weights: &'static [f64],
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

const PHRASE_CONCAT_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "one two three",
        body: "plain body",
    },
    Doc {
        rowid: 2,
        title: "one gap two three",
        body: "one two threefold",
    },
    Doc {
        rowid: 3,
        title: "one two throne",
        body: "one two four",
    },
    Doc {
        rowid: 4,
        title: "plain title",
        body: "one gap two three",
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

const UNICODE61_DISTINCT_DIACRITIC_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Accent record",
        body: "café crème résumé",
    },
    Doc {
        rowid: 2,
        title: "Plain record",
        body: "cafe creme resume",
    },
    Doc {
        rowid: 3,
        title: "Mixed record",
        body: "café resume",
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

const PORTER_UNICODE61_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Accent running",
        body: "résumés running cafés",
    },
    Doc {
        rowid: 2,
        title: "Plain run",
        body: "resume run cafe",
    },
    Doc {
        rowid: 3,
        title: "Runner note",
        body: "runner cafeteria",
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

const TRIGRAM_MATCH_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Needle start",
        body: "abcdef ghi",
    },
    Doc {
        rowid: 2,
        title: "Embedded upper",
        body: "zzABCyy",
    },
    Doc {
        rowid: 3,
        title: "Overlap tail",
        body: "bcdefg",
    },
    Doc {
        rowid: 4,
        title: "Short body",
        body: "ab",
    },
];

const TRIGRAM_CASE_SENSITIVE_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Upper ABC",
        body: "ABCdef GHI",
    },
    Doc {
        rowid: 2,
        title: "Lower abc",
        body: "abcdef ghi",
    },
    Doc {
        rowid: 3,
        title: "Mixed AbC",
        body: "zzAbCyy",
    },
    Doc {
        rowid: 4,
        title: "Short AB",
        body: "AB",
    },
];

const TRIGRAM_DIACRITIC_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Accent upper",
        body: "ábC sigma",
    },
    Doc {
        rowid: 2,
        title: "Plain lower",
        body: "abc tail",
    },
    Doc {
        rowid: 3,
        title: "Accent short",
        body: "éAB mixed",
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

const UNICODE61_DEFAULT_SEPARATOR_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "Dot separated",
        body: "alpha.beta gamma",
    },
    Doc {
        rowid: 2,
        title: "Hyphen separated",
        body: "alpha beta-gamma",
    },
    Doc {
        rowid: 3,
        title: "Underscore separated",
        body: "alpha_beta delta",
    },
    Doc {
        rowid: 4,
        title: "Joined token",
        body: "alphabeta gamma",
    },
];

const ASCII_TOKENIZER_DOCS: &[Doc] = &[
    Doc {
        rowid: 1,
        title: "ASCII cafe",
        body: "cafe HELLO world",
    },
    Doc {
        rowid: 2,
        title: "Accent record",
        body: "café hello world",
    },
    Doc {
        rowid: 3,
        title: "Upper token",
        body: "HELLO-writer ABC123",
    },
];

const UNICODE61_DIACRITIC_OPTIONS: &[&str] = &["tokenize='unicode61 remove_diacritics 2'"];
const UNICODE61_KEEP_DIACRITIC_OPTIONS: &[&str] = &[r#"tokenize="unicode61 remove_diacritics 0""#];
const UNICODE61_CODE_TOKEN_OPTIONS: &[&str] = &[r#"tokenize="unicode61 tokenchars '-_./:@#$%'""#];
const UNICODE61_SEPARATOR_OPTIONS: &[&str] = &[r#"tokenize="unicode61 separators '_.'""#];
const ASCII_TOKENIZER_OPTIONS: &[&str] = &["tokenize='ascii'"];
const UNINDEXED_COLUMN_SPECS: &[&str] = &["title", "body UNINDEXED"];
const COLUMN_SIZE_ZERO_OPTIONS: &[&str] = &["columnsize=0"];
const CONTENTLESS_OPTIONS: &[&str] = &["content=''"];
const PREFIX_INDEX_OPTIONS: &[&str] = &["prefix='2 3'"];
const DETAIL_COLUMN_OPTIONS: &[&str] = &["detail=column"];
const DETAIL_NONE_OPTIONS: &[&str] = &["detail=none"];
const PORTER_OPTIONS: &[&str] = &["tokenize='porter'"];
const PORTER_UNICODE61_OPTIONS: &[&str] = &["tokenize='porter unicode61 remove_diacritics 2'"];
const TRIGRAM_OPTIONS: &[&str] = &["tokenize='trigram'"];
const TRIGRAM_CASE_SENSITIVE_OPTIONS: &[&str] = &["tokenize='trigram case_sensitive 1'"];
const TRIGRAM_REMOVE_DIACRITICS_OPTIONS: &[&str] = &["tokenize='trigram remove_diacritics 1'"];

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

const IMPLICIT_AND_CASES: &[MatchCase] = &[
    MatchCase {
        name: "two adjacent bare terms",
        query: "rust search",
    },
    MatchCase {
        name: "three adjacent bare terms",
        query: "rust sqlite search",
    },
    MatchCase {
        name: "adjacent column filters",
        query: "title:rust body:search",
    },
    MatchCase {
        name: "adjacent column filter and body term",
        query: "title:rust body:writers",
    },
    MatchCase {
        name: "column filter followed by implicit union term",
        query: "title:rust sqlite",
    },
];

const CASE_FOLDING_CASES: &[MatchCase] = &[
    MatchCase {
        name: "uppercase bare term",
        query: "RUST",
    },
    MatchCase {
        name: "mixed-case implicit column union",
        query: "Sqlite",
    },
    MatchCase {
        name: "uppercase phrase",
        query: r#""FULL TEXT""#,
    },
    MatchCase {
        name: "mixed-case prefix",
        query: "Sea*",
    },
    MatchCase {
        name: "uppercase column filter",
        query: "title:RUST",
    },
    MatchCase {
        name: "uppercase boolean expression",
        query: "RUST AND SQLITE",
    },
];

const DEFAULT_DIACRITIC_CASES: &[MatchCase] = &[
    MatchCase {
        name: "plain query matches accented text",
        query: "cafe",
    },
    MatchCase {
        name: "accented query matches plain text",
        query: "résumé",
    },
    MatchCase {
        name: "plain phrase matches accented phrase",
        query: r#""cafe creme""#,
    },
    MatchCase {
        name: "accented phrase matches plain phrase",
        query: r#""café crème""#,
    },
    MatchCase {
        name: "plain prefix matches accented token",
        query: "crem*",
    },
    MatchCase {
        name: "accented prefix matches plain token",
        query: "résum*",
    },
];

const KEEP_DIACRITIC_CASES: &[MatchCase] = &[
    MatchCase {
        name: "accented term stays distinct",
        query: "café",
    },
    MatchCase {
        name: "plain term stays distinct",
        query: "cafe",
    },
    MatchCase {
        name: "accented phrase stays distinct",
        query: r#""café crème""#,
    },
    MatchCase {
        name: "plain phrase stays distinct",
        query: r#""cafe creme""#,
    },
    MatchCase {
        name: "accented prefix stays distinct",
        query: "résum*",
    },
    MatchCase {
        name: "plain prefix stays distinct",
        query: "resum*",
    },
];

const DEFAULT_SEPARATOR_CASES: &[MatchCase] = &[
    MatchCase {
        name: "dot separator exposes right token",
        query: "beta",
    },
    MatchCase {
        name: "underscore separator exposes both tokens",
        query: "alpha AND delta",
    },
    MatchCase {
        name: "hyphen separator supports adjacent phrase",
        query: r#""beta gamma""#,
    },
    MatchCase {
        name: "body column phrase over punctuation",
        query: r#"body:"alpha beta""#,
    },
    MatchCase {
        name: "prefix includes joined and split tokens",
        query: "alph*",
    },
];

const UNICODE61_TOKENCHAR_CASES: &[MatchCase] = &[
    MatchCase {
        name: "underscore token stays intact",
        query: "my_function",
    },
    MatchCase {
        name: "dotted identifier stays intact",
        query: r#""AuthController.ts""#,
    },
    MatchCase {
        name: "slash token stays intact",
        query: r#""endpoint/v1""#,
    },
    MatchCase {
        name: "plain split token remains searchable",
        query: "endpoint",
    },
    MatchCase {
        name: "body column code token",
        query: r#"body:"my_function""#,
    },
    MatchCase {
        name: "dotted router token",
        query: r#""Router.js""#,
    },
    MatchCase {
        name: "prefix over code token",
        query: "Auth*",
    },
];

const UNICODE61_CUSTOM_SEPARATOR_CASES: &[MatchCase] = &[
    MatchCase {
        name: "dot separator exposes right token",
        query: "beta",
    },
    MatchCase {
        name: "underscore separator exposes right token",
        query: "gamma",
    },
    MatchCase {
        name: "custom separators support adjacent phrase",
        query: r#""beta gamma""#,
    },
    MatchCase {
        name: "joined token remains distinct",
        query: "alphabetagamma",
    },
    MatchCase {
        name: "split phrase excludes joined token",
        query: r#""alpha beta""#,
    },
    MatchCase {
        name: "body column custom separator phrase",
        query: r#"body:"beta gamma""#,
    },
];

const ASCII_TOKENIZER_CASES: &[MatchCase] = &[
    MatchCase {
        name: "ascii case folds uppercase body",
        query: "hello",
    },
    MatchCase {
        name: "ascii exact token excludes accented suffix",
        query: "cafe",
    },
    MatchCase {
        name: "ascii prefix fragment excludes non-ascii token",
        query: "caf",
    },
    MatchCase {
        name: "non-ascii query normalizes through ascii tokenizer",
        query: "café",
    },
    MatchCase {
        name: "ascii phrase across folded tokens",
        query: r#""hello world""#,
    },
    MatchCase {
        name: "ascii alnum token",
        query: "abc123",
    },
    MatchCase {
        name: "ascii prefix over folded token",
        query: "writ*",
    },
    MatchCase {
        name: "ascii title column filter",
        query: "title:ascii",
    },
];

const PORTER_UNICODE61_CASES: &[MatchCase] = &[
    MatchCase {
        name: "porter strips plural after unicode61 diacritic removal",
        query: "resume",
    },
    MatchCase {
        name: "accented query normalizes through porter unicode61",
        query: "résumé",
    },
    MatchCase {
        name: "porter stems running to run",
        query: "run",
    },
    MatchCase {
        name: "porter phrase after diacritic removal",
        query: r#""resume run""#,
    },
    MatchCase {
        name: "porter strips plural cafe token",
        query: "cafe",
    },
    MatchCase {
        name: "porter title column filter",
        query: "title:run",
    },
];

const TRIGRAM_MATCH_CASES: &[MatchCase] = &[
    MatchCase {
        name: "case-folded three-character term",
        query: "abc",
    },
    MatchCase {
        name: "overlapping three-character term",
        query: "bcd",
    },
    MatchCase {
        name: "multi-trigram term requires every trigram",
        query: "abcdef",
    },
    MatchCase {
        name: "quoted multi-trigram phrase",
        query: r#""bcdef""#,
    },
    MatchCase {
        name: "title column multi-trigram term",
        query: "title:needle",
    },
    MatchCase {
        name: "body column suffix term",
        query: "body:ghi",
    },
    MatchCase {
        name: "leading context multi-trigram term",
        query: "zzabc",
    },
];

const TRIGRAM_CASE_SENSITIVE_CASES: &[MatchCase] = &[
    MatchCase {
        name: "uppercase trigram stays uppercase",
        query: "ABC",
    },
    MatchCase {
        name: "lowercase trigram stays lowercase",
        query: "abc",
    },
    MatchCase {
        name: "mixed-case trigram stays mixed",
        query: "AbC",
    },
    MatchCase {
        name: "uppercase multi-trigram term",
        query: "ABCd",
    },
    MatchCase {
        name: "lowercase multi-trigram term",
        query: "abcd",
    },
    MatchCase {
        name: "uppercase title column filter",
        query: "title:ABC",
    },
    MatchCase {
        name: "lowercase body column filter",
        query: "body:ghi",
    },
];

const TRIGRAM_REMOVE_DIACRITIC_CASES: &[MatchCase] = &[
    MatchCase {
        name: "plain query matches accented trigram",
        query: "abc",
    },
    MatchCase {
        name: "accented query matches plain trigram",
        query: "ábC",
    },
    MatchCase {
        name: "accented leading trigram normalizes",
        query: "éab",
    },
    MatchCase {
        name: "multi-trigram term after diacritic removal",
        query: "sigma",
    },
    MatchCase {
        name: "title column multi-trigram term",
        query: "title:accent",
    },
    MatchCase {
        name: "body column accented trigram",
        query: "body:éab",
    },
];

const MULTIPLE_MATCH_CASES: &[MultiMatchCase] = &[
    MultiMatchCase {
        name: "intersect bare terms",
        left_query: "rust",
        right_query: "sqlite",
    },
    MultiMatchCase {
        name: "intersect title and body filters",
        left_query: "title:rust",
        right_query: "body:search",
    },
    MultiMatchCase {
        name: "reverse column filters",
        left_query: "body:search",
        right_query: "title:sqlite",
    },
    MultiMatchCase {
        name: "compound boolean intersection",
        left_query: "rust OR bread",
        right_query: "sqlite NOT cooking",
    },
];

const NO_RESULT_CASES: &[MatchCase] = &[
    MatchCase {
        name: "absent term",
        query: "wal",
    },
    MatchCase {
        name: "absent phrase",
        query: r#""rust cooking""#,
    },
    MatchCase {
        name: "absent prefix",
        query: "zz*",
    },
    MatchCase {
        name: "column filter excludes body-only term",
        query: "title:bread",
    },
    MatchCase {
        name: "initial-token term appears only later",
        query: "^writers",
    },
    MatchCase {
        name: "near terms never co-occur",
        query: "NEAR(rust bread, 1)",
    },
];

const BOOLEAN_PRECEDENCE_CASES: &[MatchCase] = &[
    MatchCase {
        name: "and binds tighter than or",
        query: "rust OR sqlite AND cooking",
    },
    MatchCase {
        name: "parenthesized or with and",
        query: "(rust OR sqlite) AND search",
    },
    MatchCase {
        name: "and with parenthesized or",
        query: "rust AND (sqlite OR bread)",
    },
    MatchCase {
        name: "parenthesized left operand with binary not",
        query: "(rust OR bread) NOT cooking",
    },
    MatchCase {
        name: "near expression with or",
        query: "NEAR(rust sqlite, 3) OR bread",
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

const NEAR_DEFAULT_COLUMN_CASES: &[MatchCase] = &[
    MatchCase {
        name: "default near distance",
        query: "NEAR(rust sqlite)",
    },
    MatchCase {
        name: "near phrase and term default distance",
        query: r#"NEAR("full text" search)"#,
    },
    MatchCase {
        name: "title-filtered near",
        query: "title:NEAR(rust sqlite)",
    },
    MatchCase {
        name: "body-filtered near",
        query: "body:NEAR(sqlite search)",
    },
    MatchCase {
        name: "three operand near explicit distance",
        query: "NEAR(rust sqlite search, 6)",
    },
];

const PHRASE_CONCAT_CASES: &[MatchCase] = &[
    MatchCase {
        name: "tight bare terms",
        query: "one+two",
    },
    MatchCase {
        name: "quoted phrase plus bare term",
        query: r#""one two" + three"#,
    },
    MatchCase {
        name: "multi-part final prefix",
        query: "one + two + thr*",
    },
    MatchCase {
        name: "title column tight phrase",
        query: "title:one+two",
    },
    MatchCase {
        name: "body column tight phrase",
        query: "body:one+two",
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

const COLUMN_FILTER_SET_CASES: &[MatchCase] = &[
    MatchCase {
        name: "negative braced column set",
        query: "-{title body}:rust",
    },
    MatchCase {
        name: "spaced negative braced column set",
        query: "- {title body}: rust",
    },
    MatchCase {
        name: "braced title or body filter",
        query: "{title}:rust OR {body}:writers",
    },
    MatchCase {
        name: "braced set with body exclusion",
        query: "{title body}:search NOT body:systems",
    },
    MatchCase {
        name: "negative braced title filter",
        query: "-{title}:rust",
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

const DETAIL_COLUMN_CASES: &[MatchCase] = &[
    MatchCase {
        name: "detail column term",
        query: "rust",
    },
    MatchCase {
        name: "detail column prefix",
        query: "search*",
    },
    MatchCase {
        name: "detail column title filter",
        query: "title:sqlite",
    },
    MatchCase {
        name: "detail column boolean",
        query: "rust AND sqlite",
    },
];

const DETAIL_NONE_CASES: &[MatchCase] = &[
    MatchCase {
        name: "detail none term",
        query: "rust",
    },
    MatchCase {
        name: "detail none implicit column union",
        query: "sqlite",
    },
    MatchCase {
        name: "detail none boolean or",
        query: "rust OR bread",
    },
    MatchCase {
        name: "detail none binary not",
        query: "sqlite NOT cooking",
    },
];

const MUTATION_CASES: &[MatchCase] = &[
    MatchCase {
        name: "deleted row old term",
        query: "bread",
    },
    MatchCase {
        name: "replaced row removed term",
        query: "full",
    },
    MatchCase {
        name: "replaced row new term",
        query: "bytecode",
    },
    MatchCase {
        name: "inserted row new term",
        query: "mutation",
    },
    MatchCase {
        name: "retained term after mutations",
        query: "sqlite",
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

const TITLE_HEAVY_WEIGHTS: &[f64] = &[8.0, 1.0];
const BODY_HEAVY_WEIGHTS: &[f64] = &[1.0, 4.0];
const SQLITE_TITLE_HEAVY_WEIGHTS: &[f64] = &[5.0, 1.0];
const BM25_SCORE_EPSILON: f64 = 1.0e-12;

const WEIGHTED_BM25_CASES: &[WeightedRankingCase] = &[
    WeightedRankingCase {
        name: "title-heavy rust search",
        query: "rust search",
        weights: TITLE_HEAVY_WEIGHTS,
    },
    WeightedRankingCase {
        name: "body-heavy rust search",
        query: "rust search",
        weights: BODY_HEAVY_WEIGHTS,
    },
    WeightedRankingCase {
        name: "title-heavy sqlite search",
        query: "sqlite search",
        weights: SQLITE_TITLE_HEAVY_WEIGHTS,
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
        name: "trigram case sensitive uppercase",
        options: TRIGRAM_CASE_SENSITIVE_OPTIONS,
        docs: TRIGRAM_DOCS,
        query: "ABC",
    },
    TokenizerCase {
        name: "trigram case sensitive lowercase",
        options: TRIGRAM_CASE_SENSITIVE_OPTIONS,
        docs: TRIGRAM_DOCS,
        query: "abc",
    },
    TokenizerCase {
        name: "trigram remove diacritics",
        options: TRIGRAM_REMOVE_DIACRITICS_OPTIONS,
        docs: TRIGRAM_DIACRITIC_DOCS,
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

    fn insert_or_replace_doc(&mut self, doc: Doc) {
        let columns = vec![doc.title.to_owned(), doc.body.to_owned()];
        self.franken.insert_document(doc.rowid, &columns);
        self.sqlite
            .execute(
                "INSERT OR REPLACE INTO docs(rowid, title, body) VALUES (?1, ?2, ?3)",
                (doc.rowid, doc.title, doc.body),
            )
            .expect("insert or replace rusqlite FTS5 row");
    }

    fn delete_doc(&mut self, rowid: i64) {
        self.franken.delete_document(rowid);
        self.sqlite
            .execute("DELETE FROM docs WHERE rowid = ?1", [rowid])
            .expect("delete rusqlite FTS5 row");
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

    fn franken_multiple_match_rowids(&self, left_query: &str, right_query: &str) -> Vec<i64> {
        let cx = Cx::new();
        let mut cursor = self.franken.open().expect("open FrankenSQLite FTS5 cursor");
        cursor
            .filter(
                &cx,
                1,
                None,
                &[
                    SqliteValue::Text(SmallText::from_string(left_query.to_owned())),
                    SqliteValue::Text(SmallText::from_string(right_query.to_owned())),
                ],
            )
            .expect("filter FrankenSQLite FTS5 cursor with multiple MATCH constraints");

        let mut rowids = Vec::new();
        while !cursor.eof() {
            rowids.push(cursor.rowid().expect("read FrankenSQLite FTS5 rowid"));
            cursor.next(&cx).expect("advance FrankenSQLite FTS5 cursor");
        }
        rowids.sort_unstable();
        rowids
    }

    fn sqlite_multiple_match_rowids(&self, left_query: &str, right_query: &str) -> Vec<i64> {
        let mut stmt = self
            .sqlite
            .prepare(
                "SELECT rowid FROM docs \
                 WHERE docs MATCH ?1 AND docs MATCH ?2 \
                 ORDER BY rowid",
            )
            .expect("prepare rusqlite FTS5 multiple MATCH query");
        stmt.query_map([left_query, right_query], |row| row.get::<_, i64>(0))
            .expect("query rusqlite FTS5 multiple MATCH rowids")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("read rusqlite FTS5 multiple MATCH rowids")
    }

    fn franken_full_scan_rows(&self) -> Vec<(i64, String, String)> {
        let cx = Cx::new();
        let mut cursor = self.franken.open().expect("open FrankenSQLite FTS5 cursor");
        cursor
            .filter(&cx, 0, None, &[])
            .expect("filter FrankenSQLite FTS5 cursor for full scan");

        let mut rows = Vec::new();
        while !cursor.eof() {
            let rowid = cursor.rowid().expect("read FrankenSQLite FTS5 rowid");

            let mut title_ctx = ColumnContext::new();
            cursor
                .column(&mut title_ctx, 0)
                .expect("read FrankenSQLite FTS5 title column");
            let title = title_ctx
                .take_value()
                .expect("FrankenSQLite FTS5 title value")
                .to_text();

            let mut body_ctx = ColumnContext::new();
            cursor
                .column(&mut body_ctx, 1)
                .expect("read FrankenSQLite FTS5 body column");
            let body = body_ctx
                .take_value()
                .expect("FrankenSQLite FTS5 body value")
                .to_text();

            rows.push((rowid, title, body));
            cursor.next(&cx).expect("advance FrankenSQLite FTS5 cursor");
        }
        rows
    }

    fn sqlite_full_scan_rows(&self) -> Vec<(i64, String, String)> {
        let mut stmt = self
            .sqlite
            .prepare("SELECT rowid, title, body FROM docs ORDER BY rowid")
            .expect("prepare rusqlite FTS5 full scan query");
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .expect("query rusqlite FTS5 full scan rows")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("read rusqlite FTS5 full scan rows")
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

    fn franken_bm25_scores(&self, query: &str) -> Vec<(i64, f64)> {
        let mut ranked = self
            .franken
            .search(query)
            .expect("FrankenSQLite FTS5 ranked score query");
        ranked.sort_by(|left, right| {
            left.0.cmp(&right.0).then_with(|| {
                left.1
                    .partial_cmp(&right.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
        ranked
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

    fn sqlite_bm25_scores(&self, query: &str) -> Vec<(i64, f64)> {
        let mut stmt = self
            .sqlite
            .prepare("SELECT rowid, bm25(docs) FROM docs WHERE docs MATCH ?1 ORDER BY rowid")
            .expect("prepare rusqlite FTS5 BM25 score query");
        stmt.query_map([query], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })
        .expect("query rusqlite FTS5 BM25 scores")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("read rusqlite FTS5 BM25 scores")
    }

    fn franken_rank_column_scores(&self, query: &str, rank_column: i32) -> Vec<(i64, f64)> {
        let cx = Cx::new();
        let mut cursor = self.franken.open().expect("open FrankenSQLite FTS5 cursor");
        cursor
            .filter(
                &cx,
                1,
                None,
                &[SqliteValue::Text(SmallText::from_string(query.to_owned()))],
            )
            .expect("filter FrankenSQLite FTS5 cursor");

        let mut scores = Vec::new();
        while !cursor.eof() {
            let rowid = cursor.rowid().expect("read FrankenSQLite FTS5 rowid");
            let mut ctx = ColumnContext::new();
            cursor
                .column(&mut ctx, rank_column)
                .expect("read FrankenSQLite FTS5 rank column");
            let value = ctx
                .take_value()
                .expect("FrankenSQLite FTS5 rank column value");
            let SqliteValue::Float(score) = value else {
                panic!("FrankenSQLite FTS5 rank column returned {value:?}");
            };
            scores.push((rowid, score));
            cursor.next(&cx).expect("advance FrankenSQLite FTS5 cursor");
        }
        scores.sort_by_key(|(rowid, _score)| *rowid);
        scores
    }

    fn sqlite_rank_column_scores(&self, query: &str) -> Vec<(i64, f64)> {
        let mut stmt = self
            .sqlite
            .prepare("SELECT rowid, rank FROM docs WHERE docs MATCH ?1 ORDER BY rowid")
            .expect("prepare rusqlite FTS5 rank query");
        stmt.query_map([query], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })
        .expect("query rusqlite FTS5 rank scores")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("read rusqlite FTS5 rank scores")
    }

    fn franken_weighted_ranked_rowids(&self, query: &str, weights: &[f64]) -> Vec<i64> {
        let mut ranked = self
            .franken
            .search_queries_with_weights(&[query], weights)
            .expect("FrankenSQLite FTS5 weighted ranked query");
        ranked.sort_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        ranked.into_iter().map(|(rowid, _score)| rowid).collect()
    }

    fn franken_weighted_bm25_scores(&self, query: &str, weights: &[f64]) -> Vec<(i64, f64)> {
        let mut ranked = self
            .franken
            .search_queries_with_weights(&[query], weights)
            .expect("FrankenSQLite FTS5 weighted ranked score query");
        ranked.sort_by(|left, right| {
            left.0.cmp(&right.0).then_with(|| {
                left.1
                    .partial_cmp(&right.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
        ranked
    }

    fn sqlite_weighted_bm25_rowids(&self, query: &str, weights: &[f64]) -> Vec<i64> {
        let weights_sql = weights
            .iter()
            .map(|weight| weight.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let rank_expr = if weights_sql.is_empty() {
            "bm25(docs)".to_owned()
        } else {
            format!("bm25(docs, {weights_sql})")
        };
        let sql = format!("SELECT rowid FROM docs WHERE docs MATCH ?1 ORDER BY {rank_expr}, rowid");
        let mut stmt = self
            .sqlite
            .prepare(&sql)
            .expect("prepare rusqlite FTS5 weighted BM25 query");
        stmt.query_map([query], |row| row.get::<_, i64>(0))
            .expect("query rusqlite FTS5 weighted BM25 rowids")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("read rusqlite FTS5 weighted BM25 rowids")
    }

    fn sqlite_weighted_bm25_scores(&self, query: &str, weights: &[f64]) -> Vec<(i64, f64)> {
        let weights_sql = weights
            .iter()
            .map(|weight| weight.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let rank_expr = if weights_sql.is_empty() {
            "bm25(docs)".to_owned()
        } else {
            format!("bm25(docs, {weights_sql})")
        };
        let sql = format!("SELECT rowid, {rank_expr} FROM docs WHERE docs MATCH ?1 ORDER BY rowid");
        let mut stmt = self
            .sqlite
            .prepare(&sql)
            .expect("prepare rusqlite FTS5 weighted BM25 score query");
        stmt.query_map([query], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })
        .expect("query rusqlite FTS5 weighted BM25 scores")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("read rusqlite FTS5 weighted BM25 scores")
    }

    fn franken_highlight_rows(&self, query: &str, column: usize) -> Vec<(i64, String)> {
        let func = Fts5HighlightFunc;
        let mut rows: Vec<(i64, String)> = self
            .franken
            .search_rows(query)
            .expect("FrankenSQLite FTS5 rows for highlight")
            .into_iter()
            .map(|(rowid, _score, columns)| {
                let text = columns.get(column).cloned().unwrap_or_default();
                let highlighted = func
                    .invoke(&[
                        SqliteValue::Text(SmallText::from_string(text)),
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

    fn sqlite_highlight_rows(&self, query: &str, column: usize) -> Vec<(i64, String)> {
        let sql = format!(
            "SELECT rowid, highlight(docs, {column}, '<b>', '</b>') \
             FROM docs WHERE docs MATCH ?1 ORDER BY rowid"
        );
        let mut stmt = self
            .sqlite
            .prepare(&sql)
            .expect("prepare rusqlite FTS5 highlight query");
        stmt.query_map([query], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .expect("query rusqlite FTS5 highlight rows")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("read rusqlite FTS5 highlight rows")
    }

    fn franken_snippet_rows(&self, query: &str, column: usize) -> Vec<(i64, String)> {
        let func = Fts5SnippetFunc;
        let mut rows: Vec<(i64, String)> = self
            .franken
            .search_rows(query)
            .expect("FrankenSQLite FTS5 rows for snippet")
            .into_iter()
            .map(|(rowid, _score, columns)| {
                let text = columns.get(column).cloned().unwrap_or_default();
                let snippet = func
                    .invoke(&[
                        SqliteValue::Text(SmallText::from_string(text)),
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

    fn sqlite_snippet_rows(&self, query: &str, column: usize) -> Vec<(i64, String)> {
        let sql = format!(
            "SELECT rowid, snippet(docs, {column}, '[', ']', '...', 64) \
             FROM docs WHERE docs MATCH ?1 ORDER BY rowid"
        );
        let mut stmt = self
            .sqlite
            .prepare(&sql)
            .expect("prepare rusqlite FTS5 snippet query");
        stmt.query_map([query], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .expect("query rusqlite FTS5 snippet rows")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("read rusqlite FTS5 snippet rows")
    }
}

fn assert_bm25_scores_match(
    franken: &[(i64, f64)],
    sqlite: &[(i64, f64)],
    context: impl std::fmt::Display,
) {
    assert_eq!(
        franken.len(),
        sqlite.len(),
        "BM25 score row count mismatch: {context}"
    );

    for ((franken_rowid, franken_score), (sqlite_rowid, sqlite_score)) in franken.iter().zip(sqlite)
    {
        assert_eq!(
            franken_rowid, sqlite_rowid,
            "BM25 score rowid mismatch: {context}"
        );
        assert!(
            (franken_score - sqlite_score).abs() <= BM25_SCORE_EPSILON,
            "BM25 score mismatch for rowid {franken_rowid}: FrankenSQLite={franken_score:?}, rusqlite={sqlite_score:?}, {context}"
        );
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
fn implicit_and_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in IMPLICIT_AND_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "implicit AND conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn case_folding_match_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in CASE_FOLDING_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "case-folding MATCH conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn default_unicode61_diacritic_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(&[], UNICODE61_DIACRITIC_DOCS);

    for case in DEFAULT_DIACRITIC_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "default unicode61 diacritic conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn unicode61_keep_diacritic_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(
        UNICODE61_KEEP_DIACRITIC_OPTIONS,
        UNICODE61_DISTINCT_DIACRITIC_DOCS,
    );

    for case in KEEP_DIACRITIC_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "unicode61 keep-diacritic conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn default_unicode61_separator_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(&[], UNICODE61_DEFAULT_SEPARATOR_DOCS);

    for case in DEFAULT_SEPARATOR_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "default unicode61 separator conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn unicode61_custom_separator_queries_match_rusqlite_reference() {
    let harness =
        Fts5ConformanceHarness::with_docs(UNICODE61_SEPARATOR_OPTIONS, UNICODE61_SEPARATOR_DOCS);

    for case in UNICODE61_CUSTOM_SEPARATOR_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "unicode61 custom separator conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn unicode61_tokenchar_queries_match_rusqlite_reference() {
    let harness =
        Fts5ConformanceHarness::with_docs(UNICODE61_CODE_TOKEN_OPTIONS, UNICODE61_CODE_TOKEN_DOCS);

    for case in UNICODE61_TOKENCHAR_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "unicode61 tokenchars conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn ascii_tokenizer_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(ASCII_TOKENIZER_OPTIONS, ASCII_TOKENIZER_DOCS);

    for case in ASCII_TOKENIZER_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "ascii tokenizer conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn porter_unicode61_queries_match_rusqlite_reference() {
    let harness =
        Fts5ConformanceHarness::with_docs(PORTER_UNICODE61_OPTIONS, PORTER_UNICODE61_DOCS);

    for case in PORTER_UNICODE61_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "porter unicode61 tokenizer conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn trigram_match_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(TRIGRAM_OPTIONS, TRIGRAM_MATCH_DOCS);

    for case in TRIGRAM_MATCH_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "trigram MATCH conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn trigram_case_sensitive_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(
        TRIGRAM_CASE_SENSITIVE_OPTIONS,
        TRIGRAM_CASE_SENSITIVE_DOCS,
    );

    for case in TRIGRAM_CASE_SENSITIVE_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "trigram case-sensitive conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn trigram_remove_diacritic_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(
        TRIGRAM_REMOVE_DIACRITICS_OPTIONS,
        TRIGRAM_DIACRITIC_DOCS,
    );

    for case in TRIGRAM_REMOVE_DIACRITIC_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "trigram remove-diacritic conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn multiple_match_constraints_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in MULTIPLE_MATCH_CASES {
        assert_eq!(
            harness.franken_multiple_match_rowids(case.left_query, case.right_query),
            harness.sqlite_multiple_match_rowids(case.left_query, case.right_query),
            "multiple MATCH conformance case failed: {} ({} AND {})",
            case.name,
            case.left_query,
            case.right_query
        );
    }
}

#[test]
fn full_scan_rows_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    assert_eq!(
        harness.franken_full_scan_rows(),
        harness.sqlite_full_scan_rows(),
        "full-scan rowid and column conformance failed"
    );
}

#[test]
fn no_result_match_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in NO_RESULT_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "no-result MATCH conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn boolean_precedence_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in BOOLEAN_PRECEDENCE_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "boolean precedence conformance case failed: {} ({})",
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
fn near_default_and_column_filter_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in NEAR_DEFAULT_COLUMN_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "NEAR default/column-filter conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn phrase_concatenation_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(&[], PHRASE_CONCAT_DOCS);

    for case in PHRASE_CONCAT_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "phrase concatenation conformance case failed: {} ({})",
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
fn column_filter_set_queries_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in COLUMN_FILTER_SET_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "column-filter set conformance case failed: {} ({})",
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
fn columnsize_zero_tables_match_rusqlite_reference() {
    let match_harness = Fts5ConformanceHarness::with_docs(COLUMN_SIZE_ZERO_OPTIONS, DOCS);

    for case in MATCH_CASES {
        assert_eq!(
            match_harness.franken_match_rowids(case.query),
            match_harness.sqlite_match_rowids(case.query),
            "columnsize=0 MATCH conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }

    let ranking_harness = Fts5ConformanceHarness::with_docs(COLUMN_SIZE_ZERO_OPTIONS, BM25_DOCS);

    for case in BM25_CASES {
        assert_eq!(
            ranking_harness.franken_ranked_rowids(case.query),
            ranking_harness.sqlite_bm25_rowids(case.query),
            "columnsize=0 BM25 conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }

    for case in WEIGHTED_BM25_CASES {
        assert_eq!(
            ranking_harness.franken_weighted_ranked_rowids(case.query, case.weights),
            ranking_harness.sqlite_weighted_bm25_rowids(case.query, case.weights),
            "columnsize=0 weighted BM25 conformance case failed: {} ({}, {:?})",
            case.name,
            case.query,
            case.weights
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
fn detail_mode_queries_match_rusqlite_reference() {
    let detail_column = Fts5ConformanceHarness::with_docs(DETAIL_COLUMN_OPTIONS, DOCS);
    for case in DETAIL_COLUMN_CASES {
        assert_eq!(
            detail_column.franken_match_rowids(case.query),
            detail_column.sqlite_match_rowids(case.query),
            "detail=column conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }

    let detail_none = Fts5ConformanceHarness::with_docs(DETAIL_NONE_OPTIONS, DOCS);
    for case in DETAIL_NONE_CASES {
        assert_eq!(
            detail_none.franken_match_rowids(case.query),
            detail_none.sqlite_match_rowids(case.query),
            "detail=none conformance case failed: {} ({})",
            case.name,
            case.query
        );
    }
}

#[test]
fn mutation_queries_match_rusqlite_reference() {
    let mut harness = Fts5ConformanceHarness::new(&[]);
    harness.delete_doc(4);
    harness.insert_or_replace_doc(Doc {
        rowid: 2,
        title: "Planner notes",
        body: "query planner builds bytecode search paths",
    });
    harness.insert_or_replace_doc(Doc {
        rowid: 6,
        title: "Fresh rust",
        body: "fresh rust token for mutation search",
    });

    for case in MUTATION_CASES {
        assert_eq!(
            harness.franken_match_rowids(case.query),
            harness.sqlite_match_rowids(case.query),
            "mutation conformance case failed: {} ({})",
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
fn weighted_bm25_ranking_matches_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(&[], BM25_DOCS);

    for case in WEIGHTED_BM25_CASES {
        assert_eq!(
            harness.franken_weighted_ranked_rowids(case.query, case.weights),
            harness.sqlite_weighted_bm25_rowids(case.query, case.weights),
            "weighted BM25 conformance case failed: {} ({}, {:?})",
            case.name,
            case.query,
            case.weights
        );
    }
}

#[test]
fn bm25_scores_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(&[], BM25_DOCS);

    for case in BM25_CASES {
        assert_bm25_scores_match(
            &harness.franken_bm25_scores(case.query),
            &harness.sqlite_bm25_scores(case.query),
            format_args!("{} ({})", case.name, case.query),
        );
    }

    for case in WEIGHTED_BM25_CASES {
        assert_bm25_scores_match(
            &harness.franken_weighted_bm25_scores(case.query, case.weights),
            &harness.sqlite_weighted_bm25_scores(case.query, case.weights),
            format_args!("{} ({}, {:?})", case.name, case.query, case.weights),
        );
    }
}

#[test]
fn rank_column_matches_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::with_docs(&[], BM25_DOCS);

    for case in BM25_CASES {
        let sqlite = harness.sqlite_rank_column_scores(case.query);
        assert_bm25_scores_match(
            &harness.franken_rank_column_scores(case.query, -1),
            &sqlite,
            format_args!("hidden rank column {} ({})", case.name, case.query),
        );
        assert_bm25_scores_match(
            &harness.franken_rank_column_scores(case.query, 2),
            &sqlite,
            format_args!("trailing rank column {} ({})", case.name, case.query),
        );
    }
}

#[test]
fn highlight_and_snippet_match_rusqlite_reference() {
    let harness = Fts5ConformanceHarness::new(&[]);

    for case in AUXILIARY_CASES {
        for column in [0, 1] {
            assert_eq!(
                harness.franken_highlight_rows(case.query, column),
                harness.sqlite_highlight_rows(case.query, column),
                "highlight conformance case failed: {} ({}, column {column})",
                case.name,
                case.query
            );
            assert_eq!(
                harness.franken_snippet_rows(case.query, column),
                harness.sqlite_snippet_rows(case.query, column),
                "snippet conformance case failed: {} ({}, column {column})",
                case.name,
                case.query
            );
        }
    }
}
