use fsqlite_core::connection::Connection;
use fsqlite_types::value::SqliteValue;

#[derive(Clone, Copy)]
struct QueryCase {
    name: &'static str,
    sql: &'static str,
}

#[derive(Clone, Copy)]
struct StatementCase {
    name: &'static str,
    sql: &'static str,
}

struct CoreSqlConformanceHarness {
    franken: Connection,
    sqlite: rusqlite::Connection,
}

impl CoreSqlConformanceHarness {
    fn new(setup_sql: &str) -> Self {
        let franken = Connection::open(":memory:").expect("open FrankenSQLite in-memory database");
        let sqlite =
            rusqlite::Connection::open_in_memory().expect("open rusqlite in-memory database");
        franken
            .execute_batch(setup_sql)
            .expect("execute FrankenSQLite setup SQL");
        sqlite
            .execute_batch(setup_sql)
            .expect("execute rusqlite setup SQL");
        Self { franken, sqlite }
    }

    fn assert_queries_match(&self, family: &str, cases: &[QueryCase]) {
        for case in cases {
            assert_eq!(
                self.franken_query_rows(case.sql),
                self.sqlite_query_rows(case.sql),
                "{family} conformance case failed: {} ({})",
                case.name,
                case.sql
            );
        }
    }

    fn assert_statement_errors_match(&self, family: &str, cases: &[StatementCase]) {
        for case in cases {
            let franken_error = self.franken.execute_batch(case.sql).is_err();
            let sqlite_error = self.sqlite.execute_batch(case.sql).is_err();
            assert!(
                sqlite_error,
                "{family} conformance case expected rusqlite error: {} ({})",
                case.name, case.sql
            );
            assert_eq!(
                franken_error, sqlite_error,
                "{family} conformance case failed: {} ({})",
                case.name, case.sql
            );
        }
    }

    fn execute_script(&self, sql: &str) {
        self.franken
            .execute_batch(sql)
            .expect("execute FrankenSQLite SQL script");
        self.sqlite
            .execute_batch(sql)
            .expect("execute rusqlite SQL script");
    }

    fn franken_query_rows(&self, sql: &str) -> Vec<Vec<String>> {
        self.franken
            .query(sql)
            .expect("query FrankenSQLite")
            .iter()
            .map(|row| row.values().iter().map(format_franken_value).collect())
            .collect()
    }

    fn sqlite_query_rows(&self, sql: &str) -> Vec<Vec<String>> {
        let mut stmt = self.sqlite.prepare(sql).expect("prepare rusqlite query");
        let column_count = stmt.column_count();
        stmt.query_map([], |row| {
            let mut values = Vec::with_capacity(column_count);
            for index in 0..column_count {
                let value: rusqlite::types::Value = row.get(index)?;
                values.push(format_rusqlite_value(&value));
            }
            Ok(values)
        })
        .expect("query rusqlite rows")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("read rusqlite rows")
    }
}

fn format_franken_value(value: &SqliteValue) -> String {
    match value {
        SqliteValue::Null => "NULL".to_owned(),
        SqliteValue::Integer(number) => number.to_string(),
        SqliteValue::Float(number) => format!("{number}"),
        SqliteValue::Text(text) => format!("'{text}'"),
        SqliteValue::Blob(bytes) => format!(
            "X'{}'",
            bytes
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<String>()
        ),
    }
}

fn format_rusqlite_value(value: &rusqlite::types::Value) -> String {
    match value {
        rusqlite::types::Value::Null => "NULL".to_owned(),
        rusqlite::types::Value::Integer(number) => number.to_string(),
        rusqlite::types::Value::Real(number) => format!("{number}"),
        rusqlite::types::Value::Text(text) => format!("'{text}'"),
        rusqlite::types::Value::Blob(bytes) => format!(
            "X'{}'",
            bytes
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<String>()
        ),
    }
}

const SALES_SETUP: &str = "
    CREATE TABLE regions (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
    CREATE TABLE stores (id INTEGER PRIMARY KEY, region_id INTEGER, name TEXT NOT NULL);
    CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT NOT NULL, base_price REAL NOT NULL);
    CREATE TABLE sales (
        id INTEGER PRIMARY KEY,
        store_id INTEGER NOT NULL,
        product_id INTEGER NOT NULL,
        qty INTEGER NOT NULL,
        sale_date TEXT NOT NULL
    );
    CREATE TABLE returns (id INTEGER PRIMARY KEY, sale_id INTEGER NOT NULL, qty INTEGER NOT NULL);

    INSERT INTO regions VALUES (1, 'North'), (2, 'South'), (3, 'East');
    INSERT INTO stores VALUES
        (10, 1, 'Store-A'),
        (20, 1, 'Store-B'),
        (30, 2, 'Store-C'),
        (40, 3, 'Store-D');
    INSERT INTO products VALUES
        (100, 'Widget', 9.99),
        (200, 'Gadget', 24.99),
        (300, 'Bolt', 1.50);
    INSERT INTO sales VALUES
        (1, 10, 100, 5, '2025-01-15'),
        (2, 10, 200, 2, '2025-01-16'),
        (3, 20, 100, 10, '2025-02-01'),
        (4, 30, 300, 100, '2025-02-10'),
        (5, 40, 200, 3, '2025-03-01'),
        (6, 30, 100, 7, '2025-03-05'),
        (7, 10, 300, 50, '2025-03-10');
    INSERT INTO returns VALUES (1, 1, 1), (2, 4, 5), (3, 6, 2);
";

const SELECT_JOIN_GROUP_AGGREGATE_CASES: &[QueryCase] = &[
    QueryCase {
        name: "four table inner join",
        sql: "SELECT r.name, st.name, p.name, s.qty FROM regions r JOIN stores st ON st.region_id = r.id JOIN sales s ON s.store_id = st.id JOIN products p ON p.id = s.product_id ORDER BY r.name, st.name, s.id",
    },
    QueryCase {
        name: "left join with null fill",
        sql: "SELECT s.id, p.name, s.qty, COALESCE(ret.qty, 0) FROM sales s JOIN products p ON p.id = s.product_id LEFT JOIN returns ret ON ret.sale_id = s.id ORDER BY s.id",
    },
    QueryCase {
        name: "group by aggregate by region",
        sql: "SELECT r.name, COUNT(*) AS sale_count, SUM(s.qty) AS total_qty, MIN(s.qty), MAX(s.qty) FROM regions r JOIN stores st ON st.region_id = r.id JOIN sales s ON s.store_id = st.id GROUP BY r.name ORDER BY r.name",
    },
    QueryCase {
        name: "having aggregate expression",
        sql: "SELECT st.name, SUM(s.qty * p.base_price) AS revenue FROM stores st JOIN sales s ON s.store_id = st.id JOIN products p ON p.id = s.product_id GROUP BY st.name HAVING revenue > 50 ORDER BY revenue DESC, st.name",
    },
    QueryCase {
        name: "aggregate over expression group",
        sql: "SELECT substr(s.sale_date, 1, 7) AS month, COUNT(*), SUM(s.qty), AVG(s.qty) FROM sales s GROUP BY substr(s.sale_date, 1, 7) ORDER BY month",
    },
];

const AGGREGATE_EDGE_CASES: &[QueryCase] = &[
    QueryCase {
        name: "empty input aggregate identities",
        sql: "SELECT COUNT(*), COUNT(qty), SUM(qty), AVG(qty), MIN(qty), MAX(qty) FROM sales WHERE qty > 1000",
    },
    QueryCase {
        name: "left join aggregate null skipping",
        sql: "SELECT COUNT(*), COUNT(ret.qty), SUM(ret.qty), COALESCE(SUM(ret.qty), 0) FROM sales s LEFT JOIN returns ret ON ret.sale_id = s.id",
    },
    QueryCase {
        name: "distinct aggregate per group",
        sql: "SELECT product_id, COUNT(*), COUNT(DISTINCT store_id), SUM(DISTINCT qty) FROM sales GROUP BY product_id ORDER BY product_id",
    },
    QueryCase {
        name: "aggregate filter on whole table",
        sql: "SELECT COUNT(*) FILTER (WHERE qty >= 10), SUM(qty) FILTER (WHERE sale_date >= '2025-03-01') FROM sales",
    },
    QueryCase {
        name: "aggregate filter per joined group",
        sql: "SELECT st.region_id, COUNT(*) FILTER (WHERE s.qty >= 10), SUM(s.qty) FILTER (WHERE p.name = 'Bolt') FROM stores st JOIN sales s ON s.store_id = st.id JOIN products p ON p.id = s.product_id GROUP BY st.region_id ORDER BY st.region_id",
    },
    QueryCase {
        name: "having with distinct aggregate alias",
        sql: "SELECT st.name, COUNT(DISTINCT s.product_id) AS product_count, SUM(s.qty) AS total_qty FROM stores st LEFT JOIN sales s ON s.store_id = st.id GROUP BY st.name HAVING product_count >= 2 OR total_qty IS NULL ORDER BY st.name",
    },
    QueryCase {
        name: "case expression inside aggregate",
        sql: "SELECT SUM(CASE WHEN qty >= 10 THEN qty END), COUNT(CASE WHEN qty < 10 THEN 1 END) FROM sales",
    },
];

const UPSERT_SETUP: &str = "
    CREATE TABLE kv (
        id INTEGER PRIMARY KEY,
        key TEXT NOT NULL UNIQUE,
        value INTEGER NOT NULL,
        updated INTEGER NOT NULL DEFAULT 0
    );
    INSERT INTO kv(id, key, value, updated) VALUES
        (1, 'alpha', 10, 0),
        (2, 'beta', 20, 0),
        (3, 'gamma', 30, 0);
";

const UPSERT_SCRIPT: &str = "
    INSERT INTO kv(id, key, value)
        VALUES (4, 'delta', 40)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated = updated + 1;
    INSERT INTO kv(id, key, value)
        VALUES (5, 'alpha', 15)
        ON CONFLICT(key) DO UPDATE SET value = value + excluded.value, updated = updated + 1;
    INSERT INTO kv(id, key, value)
        VALUES (2, 'beta-replaced', 99)
        ON CONFLICT(id) DO UPDATE SET key = excluded.key, value = excluded.value, updated = updated + 1;
    INSERT INTO kv(id, key, value)
        VALUES (6, 'gamma', 999)
        ON CONFLICT(key) DO NOTHING;
";

const UPSERT_CASES: &[QueryCase] = &[
    QueryCase {
        name: "final row order",
        sql: "SELECT id, key, value, updated FROM kv ORDER BY id",
    },
    QueryCase {
        name: "updated row count",
        sql: "SELECT COUNT(*) FROM kv WHERE updated > 0",
    },
    QueryCase {
        name: "value aggregate",
        sql: "SELECT SUM(value), MIN(value), MAX(value) FROM kv",
    },
];

const WITH_UPSERT_RETURNING_SETUP: &str = UPSERT_SETUP;

const WITH_UPSERT_RETURNING_CASES: &[QueryCase] = &[
    QueryCase {
        name: "with insert-select upsert returning conflict and insert rows",
        sql: "WITH incoming(key, value) AS (VALUES ('alpha', 5), ('epsilon', 50)) INSERT INTO kv(key, value) SELECT key, value FROM incoming WHERE 1 ON CONFLICT(key) DO UPDATE SET value = value + excluded.value, updated = updated + 1 RETURNING key, value, updated",
    },
    QueryCase {
        name: "with insert-select do nothing returning only inserted rows",
        sql: "WITH incoming(key, value) AS (VALUES ('beta', 70), ('zeta', 60)) INSERT INTO kv(key, value) SELECT key, value FROM incoming WHERE 1 ON CONFLICT(key) DO NOTHING RETURNING key, value, updated",
    },
    QueryCase {
        name: "final rows after with upsert returning",
        sql: "SELECT key, value, updated FROM kv ORDER BY key",
    },
];

const CTE_SETUP: &str = "
    CREATE TABLE employees (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        manager_id INTEGER,
        salary INTEGER NOT NULL
    );
    INSERT INTO employees VALUES
        (1, 'Ada', NULL, 100),
        (2, 'Bert', 1, 80),
        (3, 'Cara', 1, 90),
        (4, 'Drew', 2, 60),
        (5, 'Eli', 2, 55),
        (6, 'Fay', 3, 70);

    CREATE TABLE facts (category TEXT NOT NULL, value INTEGER NOT NULL);
    INSERT INTO facts VALUES
        ('a', 10),
        ('a', 20),
        ('b', 7),
        ('b', 13),
        ('c', 5);
";

const CTE_CASES: &[QueryCase] = &[
    QueryCase {
        name: "ordinary aggregate cte",
        sql: "WITH totals AS (SELECT category, SUM(value) AS total FROM facts GROUP BY category) SELECT category, total FROM totals WHERE total >= 20 ORDER BY category",
    },
    QueryCase {
        name: "multi cte join",
        sql: "WITH totals AS (SELECT category, SUM(value) AS total FROM facts GROUP BY category), counts AS (SELECT category, COUNT(*) AS cnt FROM facts GROUP BY category) SELECT totals.category, totals.total, counts.cnt FROM totals JOIN counts ON counts.category = totals.category ORDER BY totals.category",
    },
    QueryCase {
        name: "recursive hierarchy",
        sql: "WITH RECURSIVE org(id, name, depth) AS (SELECT id, name, 0 FROM employees WHERE manager_id IS NULL UNION ALL SELECT e.id, e.name, org.depth + 1 FROM employees e JOIN org ON e.manager_id = org.id) SELECT name, depth FROM org ORDER BY depth, name",
    },
    QueryCase {
        name: "recursive numeric aggregate",
        sql: "WITH RECURSIVE cnt(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM cnt WHERE x < 8) SELECT SUM(x), COUNT(*), MAX(x) FROM cnt",
    },
    QueryCase {
        name: "cte self join materialization",
        sql: "WITH vals AS (SELECT 1 AS n UNION ALL SELECT 2 UNION ALL SELECT 3) SELECT a.n, b.n FROM vals a, vals b WHERE a.n < b.n ORDER BY a.n, b.n",
    },
];

const WINDOW_SETUP: &str = "
    CREATE TABLE compensation (
        id INTEGER PRIMARY KEY,
        dept TEXT NOT NULL,
        employee TEXT NOT NULL,
        salary INTEGER NOT NULL
    );
    INSERT INTO compensation VALUES
        (1, 'eng', 'Ada', 120),
        (2, 'eng', 'Bert', 95),
        (3, 'eng', 'Cara', 120),
        (4, 'ops', 'Drew', 85),
        (5, 'ops', 'Eli', 90),
        (6, 'ops', 'Fay', 85);
";

const WINDOW_CASES: &[QueryCase] = &[
    QueryCase {
        name: "row number partition",
        sql: "SELECT dept, employee, salary, ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC, employee) FROM compensation ORDER BY dept, salary DESC, employee",
    },
    QueryCase {
        name: "rank and dense rank ties",
        sql: "SELECT employee, salary, RANK() OVER (ORDER BY salary DESC), DENSE_RANK() OVER (ORDER BY salary DESC) FROM compensation ORDER BY salary DESC, employee",
    },
    QueryCase {
        name: "running partition aggregate",
        sql: "SELECT dept, employee, salary, SUM(salary) OVER (PARTITION BY dept ORDER BY employee ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM compensation ORDER BY dept, employee",
    },
    QueryCase {
        name: "sliding frame aggregate",
        sql: "SELECT id, salary, SUM(salary) OVER (ORDER BY id ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) FROM compensation ORDER BY id",
    },
    QueryCase {
        name: "lag lead defaults",
        sql: "SELECT id, salary, LAG(salary, 1, -1) OVER (ORDER BY id), LEAD(salary, 1, -1) OVER (ORDER BY id) FROM compensation ORDER BY id",
    },
];

const CAST_COLLATION_SETUP: &str = "
    CREATE TABLE typed (
        id INTEGER PRIMARY KEY,
        raw TEXT,
        amount TEXT
    );
    INSERT INTO typed VALUES
        (1, '456', '3.14'),
        (2, 'abc', '123.45abc'),
        (3, '', ''),
        (4, NULL, NULL);

    CREATE TABLE tags (tag TEXT COLLATE NOCASE);
    INSERT INTO tags VALUES
        ('Rust'),
        ('rust'),
        ('RUST'),
        ('Python'),
        ('python');

    CREATE TABLE words (w TEXT COLLATE NOCASE);
    INSERT INTO words VALUES
        ('banana'),
        ('Apple'),
        ('cherry'),
        ('APRICOT'),
        ('Blueberry');

    CREATE TABLE padded (id INTEGER PRIMARY KEY, label TEXT COLLATE RTRIM);
    INSERT INTO padded VALUES
        (1, 'abc'),
        (2, 'abc  '),
        (3, 'ABC'),
        (4, 'abd');
";

const CAST_COLLATION_CASES: &[QueryCase] = &[
    QueryCase {
        name: "cast text numeric prefixes",
        sql: "SELECT id, CAST(raw AS INTEGER), typeof(CAST(raw AS INTEGER)), CAST(amount AS REAL), typeof(CAST(amount AS REAL)) FROM typed ORDER BY id",
    },
    QueryCase {
        name: "cast scalar storage classes",
        sql: "SELECT CAST(123 AS TEXT), typeof(CAST(123 AS TEXT)), CAST(3.99 AS INTEGER), CAST(NULL AS TEXT), typeof(CAST(NULL AS TEXT))",
    },
    QueryCase {
        name: "nocase distinct aggregate",
        sql: "SELECT COUNT(DISTINCT tag), MIN(tag), MAX(tag) FROM tags",
    },
    QueryCase {
        name: "nocase grouping",
        sql: "SELECT tag, COUNT(*) FROM tags GROUP BY tag ORDER BY tag",
    },
    QueryCase {
        name: "nocase ordering",
        sql: "SELECT w FROM words ORDER BY w",
    },
    QueryCase {
        name: "rtrim equality",
        sql: "SELECT id, label FROM padded WHERE label = 'abc' ORDER BY id",
    },
];

const COLLATION_EXPRESSION_CASES: &[QueryCase] = &[
    QueryCase {
        name: "explicit nocase comparison on expression",
        sql: "SELECT w FROM words WHERE w COLLATE NOCASE = 'apple' ORDER BY w COLLATE BINARY",
    },
    QueryCase {
        name: "explicit binary comparison overrides column nocase",
        sql: "SELECT w FROM words WHERE w COLLATE BINARY = 'Apple' ORDER BY w COLLATE BINARY",
    },
    QueryCase {
        name: "explicit binary grouping overrides column nocase",
        sql: "SELECT tag COLLATE BINARY, COUNT(*) FROM tags GROUP BY tag COLLATE BINARY ORDER BY tag COLLATE BINARY",
    },
    QueryCase {
        name: "distinct aggregate respects explicit collation",
        sql: "SELECT COUNT(DISTINCT tag COLLATE BINARY), COUNT(DISTINCT tag COLLATE NOCASE) FROM tags",
    },
    QueryCase {
        name: "explicit binary comparison overrides column rtrim",
        sql: "SELECT id, label FROM padded WHERE label COLLATE BINARY = 'abc' ORDER BY id",
    },
    QueryCase {
        name: "explicit rtrim grouping on expression",
        sql: "SELECT label COLLATE RTRIM, COUNT(*) FROM padded GROUP BY label COLLATE RTRIM ORDER BY label COLLATE BINARY",
    },
];

const DML_SETUP: &str = "
    CREATE TABLE inventory (
        id INTEGER PRIMARY KEY,
        sku TEXT NOT NULL UNIQUE,
        category TEXT NOT NULL,
        qty INTEGER NOT NULL,
        price_cents INTEGER NOT NULL,
        active INTEGER NOT NULL DEFAULT 1
    );

    INSERT INTO inventory(id, sku, category, qty, price_cents, active) VALUES
        (1, 'bolt', 'hardware', 100, 50, 1),
        (2, 'screw', 'hardware', 80, 25, 1),
        (3, 'old', 'clearance', 4, 300, 1),
        (4, 'gizmo', 'gadget', 12, 500, 1);
";

const DML_SCRIPT: &str = "
    INSERT INTO inventory(id, sku, category, qty, price_cents)
        VALUES (5, 'nut', 'hardware', 40, 75);
    INSERT INTO inventory(id, sku, category, qty, price_cents, active)
        VALUES (6, 'kit', 'bundle', 37, 500, 1);

    UPDATE inventory
        SET qty = qty + 5
        WHERE category = 'hardware' AND active = 1;
    UPDATE inventory
        SET active = 0, price_cents = price_cents - 25
        WHERE sku = 'old';
    UPDATE inventory
        SET qty = qty - 15
        WHERE sku = 'bolt';
    UPDATE inventory
        SET qty = qty + 20
        WHERE sku = 'screw';

    DELETE FROM inventory
        WHERE active = 0 AND qty < 10;
    DELETE FROM inventory
        WHERE category = 'gadget' AND qty <= 12;
";

const DML_CASES: &[QueryCase] = &[
    QueryCase {
        name: "final mutated rows",
        sql: "SELECT id, sku, category, qty, price_cents, active FROM inventory ORDER BY id",
    },
    QueryCase {
        name: "category summaries after dml",
        sql: "SELECT category, COUNT(*), SUM(qty), MIN(price_cents), MAX(price_cents) FROM inventory GROUP BY category ORDER BY category",
    },
    QueryCase {
        name: "active inventory value",
        sql: "SELECT COUNT(*), SUM(qty), SUM(price_cents * qty) FROM inventory WHERE active = 1",
    },
    QueryCase {
        name: "deleted row absence",
        sql: "SELECT COUNT(*) FROM inventory WHERE sku IN ('old', 'gizmo')",
    },
];

const ATTACHED_UPDATE_SETUP: &str = "
    ATTACH DATABASE ':memory:' AS aux;

    CREATE TABLE main_filter (id INTEGER PRIMARY KEY);
    INSERT INTO main_filter VALUES (1), (3);

    CREATE TABLE aux.t (
        id INTEGER PRIMARY KEY,
        value TEXT NOT NULL UNIQUE,
        qty INTEGER NOT NULL
    );
    INSERT INTO aux.t VALUES
        (1, 'alpha', 10),
        (2, 'beta', 20),
        (3, 'gamma', 30);
";

const ATTACHED_UPDATE_CASES: &[QueryCase] = &[
    QueryCase {
        name: "attached update returning single row",
        sql: "UPDATE aux.t SET qty = qty + 1 WHERE id = 1 RETURNING id, value, qty",
    },
    QueryCase {
        name: "attached update returning with main subquery",
        sql: "UPDATE aux.t SET value = value || '-hit' WHERE id IN (SELECT id FROM main_filter) RETURNING id, value, qty",
    },
    QueryCase {
        name: "attached update with cte materialization",
        sql: "WITH picked(id) AS (SELECT id FROM main_filter WHERE id <> 1) UPDATE aux.t SET qty = qty + 10 WHERE id IN (SELECT id FROM picked) RETURNING id, qty",
    },
    QueryCase {
        name: "attached update persisted rows",
        sql: "SELECT id, value, qty FROM aux.t ORDER BY id",
    },
    QueryCase {
        name: "attached update aggregate",
        sql: "SELECT COUNT(*), SUM(qty), MIN(value), MAX(value) FROM aux.t",
    },
];

const ATTACHED_INSERT_SELECT_SETUP: &str = "
    ATTACH DATABASE ':memory:' AS aux;

    CREATE TABLE source_items (
        id INTEGER PRIMARY KEY,
        value TEXT NOT NULL,
        qty INTEGER NOT NULL
    );
    INSERT INTO source_items VALUES
        (1, 'alpha', 10),
        (2, 'beta', 20),
        (3, 'gamma', 30),
        (4, 'delta', 40);

    CREATE TABLE main_filter (id INTEGER PRIMARY KEY);
    INSERT INTO main_filter VALUES (1), (3);

    CREATE TABLE aux.t (
        id INTEGER PRIMARY KEY,
        value TEXT NOT NULL UNIQUE,
        qty INTEGER NOT NULL
    );

    INSERT INTO aux.t(id, value, qty)
        SELECT id, value, qty
        FROM source_items
        WHERE qty >= 20
        ORDER BY id;

    INSERT INTO aux.t(id, value, qty)
        SELECT id + 10, value || '-copy', qty + 1
        FROM source_items
        WHERE id IN (SELECT id FROM main_filter)
        ORDER BY id;
";

const ATTACHED_INSERT_SELECT_CASES: &[QueryCase] = &[
    QueryCase {
        name: "attached insert select persisted rows",
        sql: "SELECT id, value, qty FROM aux.t ORDER BY id",
    },
    QueryCase {
        name: "attached insert select aggregate",
        sql: "SELECT COUNT(*), SUM(qty), MIN(value), MAX(value) FROM aux.t",
    },
    QueryCase {
        name: "attached insert select copied filtered rows",
        sql: "SELECT id, value, qty FROM aux.t WHERE id >= 10 ORDER BY id",
    },
];

const ATTACHED_DROP_SETUP: &str = "
    ATTACH DATABASE ':memory:' AS aux;

    CREATE TABLE aux.keep (
        id INTEGER PRIMARY KEY,
        value TEXT NOT NULL
    );
    INSERT INTO aux.keep VALUES (1, 'stay');

    CREATE TABLE aux.drop_me (
        id INTEGER PRIMARY KEY,
        value TEXT NOT NULL
    );
    INSERT INTO aux.drop_me VALUES (1, 'gone');

    CREATE TABLE aux.drop_index_only (
        id INTEGER PRIMARY KEY,
        value TEXT NOT NULL
    );
    INSERT INTO aux.drop_index_only VALUES (1, 'indexed'), (2, 'also-indexed');

    CREATE INDEX aux.idx_drop_me_value ON drop_me(value);
    CREATE INDEX aux.idx_drop_index_only_value ON drop_index_only(value);
";

const ATTACHED_DROP_SCRIPT: &str = "
    DROP INDEX aux.idx_drop_index_only_value;
    DROP TABLE aux.drop_me;
    DROP TABLE IF EXISTS aux.missing_table;
";

const ATTACHED_DROP_CASES: &[QueryCase] = &[
    QueryCase {
        name: "attached drop removes table and indexes",
        sql: "SELECT type, name FROM aux.sqlite_master WHERE name IN ('drop_me', 'idx_drop_me_value', 'drop_index_only', 'idx_drop_index_only_value', 'keep') ORDER BY type, name",
    },
    QueryCase {
        name: "attached drop preserves unrelated table rows",
        sql: "SELECT id, value FROM aux.keep ORDER BY id",
    },
    QueryCase {
        name: "attached drop index preserves table rows",
        sql: "SELECT id, value FROM aux.drop_index_only ORDER BY id",
    },
];

const ATTACHED_CREATE_VIEW_SETUP: &str = "
    ATTACH DATABASE ':memory:' AS aux;

    CREATE TABLE aux.base (
        id INTEGER PRIMARY KEY,
        value TEXT NOT NULL,
        qty INTEGER NOT NULL
    );
    INSERT INTO aux.base VALUES
        (1, 'alpha', 10),
        (2, 'beta', 20),
        (3, 'gamma', 30);

    CREATE VIEW aux.filtered AS
        SELECT id, value, qty * 2 AS doubled
        FROM base
        WHERE qty >= 20;
";

const ATTACHED_CREATE_VIEW_CASES: &[QueryCase] = &[
    QueryCase {
        name: "attached create view projected rows",
        sql: "SELECT id, value, doubled FROM aux.filtered ORDER BY id",
    },
    QueryCase {
        name: "attached create view sqlite_master row",
        sql: "SELECT type, name FROM aux.sqlite_master WHERE name = 'filtered'",
    },
    QueryCase {
        name: "attached create view aggregate",
        sql: "SELECT COUNT(*), SUM(doubled), MIN(value), MAX(value) FROM aux.filtered",
    },
];

const ATTACHED_VACUUM_SETUP: &str = "
    ATTACH DATABASE ':memory:' AS aux;

    CREATE TABLE aux.items (
        id INTEGER PRIMARY KEY,
        value TEXT NOT NULL,
        qty INTEGER NOT NULL
    );
    INSERT INTO aux.items VALUES
        (1, 'alpha', 10),
        (2, 'beta', 20),
        (3, 'gamma', 30);
    DELETE FROM aux.items WHERE id = 2;

    VACUUM aux;
";

const ATTACHED_VACUUM_CASES: &[QueryCase] = &[
    QueryCase {
        name: "attached vacuum preserves live rows",
        sql: "SELECT id, value, qty FROM aux.items ORDER BY id",
    },
    QueryCase {
        name: "attached vacuum preserves schema row",
        sql: "SELECT type, name FROM aux.sqlite_master WHERE name = 'items'",
    },
    QueryCase {
        name: "attached vacuum aggregate",
        sql: "SELECT COUNT(*), SUM(qty), MIN(value), MAX(value) FROM aux.items",
    },
];

const ERROR_PATH_SETUP: &str = "
    CREATE TABLE constrained (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        sku TEXT NOT NULL,
        qty INTEGER NOT NULL
    );
    CREATE UNIQUE INDEX idx_constrained_sku ON constrained(sku);
    INSERT INTO constrained(id, name, sku, qty) VALUES
        (1, 'alpha', 'sku-a', 10),
        (2, 'beta', 'sku-b', 20);
";

const ERROR_PATH_CASES: &[StatementCase] = &[
    StatementCase {
        name: "duplicate integer primary key",
        sql: "INSERT INTO constrained(id, name, sku, qty) VALUES (1, 'dupe', 'sku-c', 1)",
    },
    StatementCase {
        name: "duplicate unique index value",
        sql: "INSERT INTO constrained(id, name, sku, qty) VALUES (3, 'gamma', 'sku-a', 1)",
    },
    StatementCase {
        name: "insert null into not null",
        sql: "INSERT INTO constrained(id, name, sku, qty) VALUES (4, NULL, 'sku-d', 1)",
    },
    StatementCase {
        name: "insert missing not null column",
        sql: "INSERT INTO constrained(id, name, sku) VALUES (5, 'missing-qty', 'sku-e')",
    },
    StatementCase {
        name: "update not null column to null",
        sql: "UPDATE constrained SET name = NULL WHERE id = 1",
    },
    StatementCase {
        name: "unknown table lookup",
        sql: "SELECT * FROM missing_table",
    },
    StatementCase {
        name: "unknown column lookup",
        sql: "SELECT missing_column FROM constrained",
    },
];

const DDL_SETUP: &str = "
    CREATE TABLE accounts (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        tier TEXT DEFAULT 'free',
        score INTEGER DEFAULT 0
    );
    CREATE INDEX idx_accounts_tier ON accounts(tier);
    INSERT INTO accounts(id, name) VALUES
        (1, 'Ada'),
        (2, 'Bert');
    INSERT INTO accounts(id, name, tier, score) VALUES
        (3, 'Cara', 'pro', 20),
        (4, 'Drew', 'free', 5),
        (5, 'Eli', 'enterprise', 50);

    ALTER TABLE accounts ADD COLUMN active INTEGER DEFAULT 1;
    UPDATE accounts SET active = 0 WHERE id = 2;
    CREATE VIEW active_accounts AS
        SELECT id, name, tier, score FROM accounts WHERE active = 1;
";

const DDL_CASES: &[QueryCase] = &[
    QueryCase {
        name: "defaults and altered column values",
        sql: "SELECT id, name, tier, score, active FROM accounts ORDER BY id",
    },
    QueryCase {
        name: "grouping after ddl defaults",
        sql: "SELECT tier, COUNT(*), SUM(score) FROM accounts GROUP BY tier ORDER BY tier",
    },
    QueryCase {
        name: "view projection and filter",
        sql: "SELECT name, tier, score FROM active_accounts ORDER BY score DESC, name",
    },
    QueryCase {
        name: "index schema registration",
        sql: "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'index' AND name = 'idx_accounts_tier'",
    },
    QueryCase {
        name: "view schema registration",
        sql: "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'view' AND name = 'active_accounts'",
    },
];

const TRANSACTION_SETUP: &str = "
    CREATE TABLE ledger (
        id INTEGER PRIMARY KEY,
        label TEXT NOT NULL,
        amount INTEGER NOT NULL
    );
";

const TRANSACTION_SCRIPT: &str = "
    BEGIN;
    INSERT INTO ledger(id, label, amount) VALUES
        (1, 'opening', 100),
        (2, 'fee', -10);
    SAVEPOINT adjust;
    UPDATE ledger SET amount = amount + 50 WHERE id = 1;
    INSERT INTO ledger(id, label, amount) VALUES (3, 'transient', 999);
    ROLLBACK TO adjust;
    RELEASE adjust;
    INSERT INTO ledger(id, label, amount) VALUES (4, 'settled', 25);
    COMMIT;

    BEGIN;
    INSERT INTO ledger(id, label, amount) VALUES (5, 'rolled-back', 500);
    UPDATE ledger SET amount = amount - 100 WHERE id = 4;
    ROLLBACK;
";

const TRANSACTION_CASES: &[QueryCase] = &[
    QueryCase {
        name: "committed rows survive savepoint rollback",
        sql: "SELECT id, label, amount FROM ledger ORDER BY id",
    },
    QueryCase {
        name: "rolled back rows absent",
        sql: "SELECT COUNT(*) FROM ledger WHERE id IN (3, 5)",
    },
    QueryCase {
        name: "aggregate after transaction boundaries",
        sql: "SELECT COUNT(*), SUM(amount), MIN(amount), MAX(amount) FROM ledger",
    },
];

const COMPOUND_SETUP: &str = "
    CREATE TABLE left_values (group_name TEXT NOT NULL, value INTEGER NOT NULL);
    CREATE TABLE right_values (group_name TEXT NOT NULL, value INTEGER NOT NULL);

    INSERT INTO left_values VALUES
        ('a', 1),
        ('a', 2),
        ('a', 3),
        ('b', 3),
        ('b', 4);
    INSERT INTO right_values VALUES
        ('a', 3),
        ('a', 4),
        ('a', 5),
        ('b', 4),
        ('b', 6);
";

const COMPOUND_CASES: &[QueryCase] = &[
    QueryCase {
        name: "union distinct",
        sql: "SELECT value FROM left_values WHERE group_name = 'a' UNION SELECT value FROM right_values WHERE group_name = 'a' ORDER BY value",
    },
    QueryCase {
        name: "union all preserves duplicates",
        sql: "SELECT value FROM left_values WHERE value >= 3 UNION ALL SELECT value FROM right_values WHERE value <= 4 ORDER BY value",
    },
    QueryCase {
        name: "intersect",
        sql: "SELECT value FROM left_values INTERSECT SELECT value FROM right_values ORDER BY value",
    },
    QueryCase {
        name: "except",
        sql: "SELECT value FROM left_values EXCEPT SELECT value FROM right_values ORDER BY value",
    },
    QueryCase {
        name: "compound order limit offset",
        sql: "SELECT value FROM left_values UNION ALL SELECT value FROM right_values ORDER BY value LIMIT 4 OFFSET 2",
    },
];

const CASE_NULL_SETUP: &str = "
    CREATE TABLE expr_rows (
        id INTEGER PRIMARY KEY,
        a INTEGER,
        b INTEGER,
        label TEXT
    );

    INSERT INTO expr_rows VALUES
        (1, 10, 5, 'high'),
        (2, 5, 5, 'same'),
        (3, 2, 9, NULL),
        (4, NULL, 7, 'missing-a'),
        (5, 4, NULL, NULL);
";

const CASE_NULL_CASES: &[QueryCase] = &[
    QueryCase {
        name: "searched case null aware comparisons",
        sql: "SELECT id, CASE WHEN a IS NULL THEN 'missing-a' WHEN b IS NULL THEN 'missing-b' WHEN a > b THEN 'gt' WHEN a = b THEN 'eq' ELSE 'lt' END FROM expr_rows ORDER BY id",
    },
    QueryCase {
        name: "simple case expression",
        sql: "SELECT id, CASE label WHEN 'high' THEN 1 WHEN 'same' THEN 2 ELSE 0 END FROM expr_rows ORDER BY id",
    },
    QueryCase {
        name: "case aggregate over nulls",
        sql: "SELECT SUM(CASE WHEN label IS NULL THEN 1 ELSE 0 END), SUM(CASE WHEN a IS NULL OR b IS NULL THEN 1 ELSE 0 END) FROM expr_rows",
    },
    QueryCase {
        name: "coalesce projection",
        sql: "SELECT id, COALESCE(label, 'none'), COALESCE(a, b, 0) FROM expr_rows ORDER BY id",
    },
    QueryCase {
        name: "boolean precedence with null predicates",
        sql: "SELECT id FROM expr_rows WHERE NOT (a > 3 AND b > 3) OR label IS NULL ORDER BY id",
    },
];

const SUBQUERY_SETUP: &str = "
    CREATE TABLE customers (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        tier TEXT NOT NULL
    );
    CREATE TABLE orders (
        id INTEGER PRIMARY KEY,
        customer_id INTEGER NOT NULL,
        amount INTEGER NOT NULL,
        status TEXT NOT NULL
    );
    CREATE TABLE flags (
        customer_id INTEGER PRIMARY KEY,
        blocked INTEGER NOT NULL
    );

    INSERT INTO customers VALUES
        (1, 'Ada', 'gold'),
        (2, 'Bert', 'silver'),
        (3, 'Cara', 'gold'),
        (4, 'Drew', 'bronze'),
        (5, 'Eli', 'silver');
    INSERT INTO orders VALUES
        (10, 1, 120, 'shipped'),
        (11, 1, 40, 'pending'),
        (12, 2, 75, 'shipped'),
        (13, 3, 200, 'cancelled'),
        (14, 3, 30, 'shipped'),
        (15, 5, 15, 'pending');
    INSERT INTO flags VALUES
        (2, 1),
        (4, 1);
";

const SUBQUERY_CASES: &[QueryCase] = &[
    QueryCase {
        name: "scalar correlated aggregate",
        sql: "SELECT c.name, (SELECT SUM(o.amount) FROM orders o WHERE o.customer_id = c.id) AS total_amount FROM customers c ORDER BY c.id",
    },
    QueryCase {
        name: "in grouped subquery",
        sql: "SELECT name FROM customers WHERE id IN (SELECT customer_id FROM orders GROUP BY customer_id HAVING SUM(amount) >= 100) ORDER BY name",
    },
    QueryCase {
        name: "correlated exists",
        sql: "SELECT c.name FROM customers c WHERE EXISTS (SELECT 1 FROM orders o WHERE o.customer_id = c.id AND o.status = 'shipped') ORDER BY c.name",
    },
    QueryCase {
        name: "correlated not exists",
        sql: "SELECT c.name FROM customers c WHERE NOT EXISTS (SELECT 1 FROM flags f WHERE f.customer_id = c.id AND f.blocked = 1) ORDER BY c.name",
    },
    QueryCase {
        name: "derived table aggregate",
        sql: "SELECT tier, COUNT(*), SUM(total_amount) FROM (SELECT c.id, c.tier, SUM(o.amount) AS total_amount FROM customers c JOIN orders o ON o.customer_id = c.id GROUP BY c.id, c.tier) totals GROUP BY tier ORDER BY tier",
    },
    QueryCase {
        name: "scalar threshold subquery",
        sql: "SELECT id, amount FROM orders WHERE amount > (SELECT AVG(amount) FROM orders) ORDER BY id",
    },
];

const PRAGMA_SETUP: &str = "
    PRAGMA foreign_keys = ON;
    PRAGMA user_version = 123;
    PRAGMA application_id = 456789;

    CREATE TABLE pragma_parent (
        id INTEGER PRIMARY KEY,
        code TEXT NOT NULL UNIQUE
    );
    CREATE TABLE pragma_child (
        id INTEGER PRIMARY KEY,
        parent_id INTEGER NOT NULL REFERENCES pragma_parent(id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        score INTEGER DEFAULT 0,
        note TEXT DEFAULT 'new'
    );
    CREATE UNIQUE INDEX idx_pragma_child_parent_score_name
        ON pragma_child(parent_id, score, name);
";

const PRAGMA_CASES: &[QueryCase] = &[
    QueryCase {
        name: "table info includes constraints and defaults",
        sql: "PRAGMA table_info(pragma_child)",
    },
    QueryCase {
        name: "index list includes uniqueness and origin",
        sql: "PRAGMA index_list(pragma_child)",
    },
    QueryCase {
        name: "index info preserves indexed column order",
        sql: "PRAGMA index_info(idx_pragma_child_parent_score_name)",
    },
    QueryCase {
        name: "foreign key list exposes referenced table and action",
        sql: "PRAGMA foreign_key_list(pragma_child)",
    },
    QueryCase {
        name: "user version round trip",
        sql: "PRAGMA user_version",
    },
    QueryCase {
        name: "application id round trip",
        sql: "PRAGMA application_id",
    },
    QueryCase {
        name: "foreign keys setting round trip",
        sql: "PRAGMA foreign_keys",
    },
];

const TRIGGER_SETUP: &str = "
    CREATE TABLE trigger_items (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        qty INTEGER NOT NULL,
        active INTEGER NOT NULL DEFAULT 1
    );
    CREATE TABLE trigger_audit (
        seq INTEGER PRIMARY KEY,
        action TEXT NOT NULL,
        item_id INTEGER NOT NULL,
        old_name TEXT,
        new_name TEXT,
        old_qty INTEGER,
        new_qty INTEGER
    );

    CREATE TRIGGER trigger_items_ai
        AFTER INSERT ON trigger_items
        BEGIN
            INSERT INTO trigger_audit(action, item_id, old_name, new_name, old_qty, new_qty)
            VALUES ('insert', NEW.id, NULL, NEW.name, NULL, NEW.qty);
        END;
    CREATE TRIGGER trigger_items_au
        AFTER UPDATE ON trigger_items
        BEGIN
            INSERT INTO trigger_audit(action, item_id, old_name, new_name, old_qty, new_qty)
            VALUES ('update', NEW.id, OLD.name, NEW.name, OLD.qty, NEW.qty);
        END;
    CREATE TRIGGER trigger_items_ad
        AFTER DELETE ON trigger_items
        BEGIN
            INSERT INTO trigger_audit(action, item_id, old_name, new_name, old_qty, new_qty)
            VALUES ('delete', OLD.id, OLD.name, NULL, OLD.qty, NULL);
        END;
";

const TRIGGER_SCRIPT: &str = "
    INSERT INTO trigger_items(id, name, qty) VALUES (1, 'bolt', 10);
    INSERT INTO trigger_items(id, name, qty) VALUES (2, 'nut', 20);
    UPDATE trigger_items
        SET name = 'bolt-plus', qty = qty + 5
        WHERE id = 1;
    UPDATE trigger_items
        SET active = 0
        WHERE id = 2;
    DELETE FROM trigger_items
        WHERE active = 0;
";

const TRIGGER_CASES: &[QueryCase] = &[
    QueryCase {
        name: "final trigger table rows",
        sql: "SELECT id, name, qty, active FROM trigger_items ORDER BY id",
    },
    QueryCase {
        name: "trigger audit old new values",
        sql: "SELECT action, item_id, old_name, new_name, old_qty, new_qty FROM trigger_audit ORDER BY seq",
    },
    QueryCase {
        name: "trigger action aggregate",
        sql: "SELECT action, COUNT(*) FROM trigger_audit GROUP BY action ORDER BY action",
    },
    QueryCase {
        name: "trigger schema registration",
        sql: "SELECT type, name, tbl_name FROM sqlite_schema WHERE type = 'trigger' ORDER BY name",
    },
];

const DATE_TIME_CASES: &[QueryCase] = &[
    QueryCase {
        name: "basic date time datetime projections",
        sql: "SELECT date('2024-01-15'), time('2024-01-15 13:45:30'), datetime('2024-01-15 13:45:30')",
    },
    QueryCase {
        name: "calendar boundary modifiers",
        sql: "SELECT date('2024-01-31', '+1 month'), date('2024-02-29', '+1 year'), date('2024-03-01', '-1 day')",
    },
    QueryCase {
        name: "time arithmetic modifiers",
        sql: "SELECT datetime('2024-01-15 12:00:00', '+90 minutes'), time('23:30:00', '+2 hours')",
    },
    QueryCase {
        name: "strftime calendar fields",
        sql: "SELECT strftime('%Y-%m-%d %H:%M:%S', '2024-02-29 23:59:01'), strftime('%j', '2024-12-31')",
    },
    QueryCase {
        name: "unixepoch and julianday storage class",
        sql: "SELECT unixepoch('1970-01-02 00:00:00'), typeof(julianday('2024-01-15'))",
    },
    QueryCase {
        name: "invalid and null date inputs",
        sql: "SELECT date(NULL), time('not-a-date'), datetime('2024-01-15', 'bogus')",
    },
];

const DATE_TIME_EDGE_CASES: &[QueryCase] = &[
    QueryCase {
        name: "start modifiers preserve SQLite modifier order",
        sql: "SELECT date('2024-03-15', 'start of month', '+1 day'), date('2024-03-15', '+1 day', 'start of month')",
    },
    QueryCase {
        name: "weekday modifier advances or noops",
        sql: "SELECT date('2024-03-15', 'weekday 0'), date('2024-03-17', 'weekday 0')",
    },
    QueryCase {
        name: "numeric input modifiers",
        sql: "SELECT datetime(0, 'unixepoch'), datetime(1710531045, 'auto'), date(2460384.5, 'auto')",
    },
    QueryCase {
        name: "iso separator and bare time inputs",
        sql: "SELECT datetime('2024-03-15T14:30:00'), date('12:30:00'), time('2024-03-15T14:30:45')",
    },
    QueryCase {
        name: "timezone offset normalization",
        sql: "SELECT datetime('2026-04-07T16:00:00Z'), datetime('2026-04-07T16:00:00+01:00'), datetime('2026-04-07T16:00:00-05:30')",
    },
    QueryCase {
        name: "strftime epoch and weekday fields",
        sql: "SELECT strftime('%s', '1970-01-02 00:00:00'), strftime('%w', '2024-03-17'), strftime('%W', '2024-03-18')",
    },
];

const JSON1_CASES: &[QueryCase] = &[
    QueryCase {
        name: "json validation and canonicalization",
        sql: r#"SELECT json('[1,2,3]'), json_valid('{"a":1}'), json_valid('[1,2,]')"#,
    },
    QueryCase {
        name: "json extract scalar and multi path",
        sql: r#"SELECT json_extract('{"a":[10,20,30],"b":{"c":"see"}}', '$.a[1]'), json_extract('{"a":1,"b":2}', '$.a', '$.b', '$.missing')"#,
    },
    QueryCase {
        name: "json type and array length",
        sql: r#"SELECT json_type('{"a":[1,null,"x"]}', '$.a'), json_type('{"a":[1,null,"x"]}', '$.a[1]'), json_array_length('{"a":[1,2,3]}', '$.a')"#,
    },
    QueryCase {
        name: "json constructors and quote",
        sql: r#"SELECT json_array(1, 'two', NULL), json_object('a', 1, 'b', json('[2,3]')), json_quote('needs "quotes"')"#,
    },
    QueryCase {
        name: "json mutators",
        sql: r#"SELECT json_set('{"a":[1,2]}', '$.a[#]', 3), json_insert('{"a":1}', '$.a', 99, '$.b', 2), json_replace('{"a":1,"b":2}', '$.a', 7, '$.missing', 8)"#,
    },
    QueryCase {
        name: "json remove and patch",
        sql: r#"SELECT json_remove('{"a":[1,2,3],"b":4}', '$.a[#-2]', '$.b'), json_patch('{"a":{"b":1,"c":2},"d":3}', '{"a":{"b":9,"c":null},"e":4}')"#,
    },
    QueryCase {
        name: "json_each table valued rows",
        sql: r#"SELECT key, value, type FROM json_each('["a",2,null,true]') ORDER BY key"#,
    },
];

const STRING_FUNCTION_CASES: &[QueryCase] = &[
    QueryCase {
        name: "substr positive and omitted length",
        sql: "SELECT substr('hello world', 7), substr('hello world', 7, 5), substr('abcdef', 2, 3)",
    },
    QueryCase {
        name: "substr negative and zero positions",
        sql: "SELECT substr('hello world', -5), substr('hello', 0, 3), substr('hello', -2, 1)",
    },
    QueryCase {
        name: "substr zero and negative lengths",
        sql: "SELECT substr('hello', 1, 0), substr('hello', 1, -1), substr('', 1, 1), substr(NULL, 1, 3)",
    },
    QueryCase {
        name: "replace normal overlapping and empty needle",
        sql: "SELECT replace('hello world', 'world', 'there'), replace('aaaa', 'aa', 'b'), replace('hello', '', 'x')",
    },
    QueryCase {
        name: "replace deletion and null propagation",
        sql: "SELECT replace('banana', 'na', ''), replace(NULL, 'a', 'b'), replace('abc', NULL, 'x')",
    },
    QueryCase {
        name: "instr hits misses and empty needle",
        sql: "SELECT instr('hello world', 'world'), instr('hello world', 'xyz'), instr('hello', ''), instr('', '')",
    },
    QueryCase {
        name: "instr null propagation and case sensitivity",
        sql: "SELECT instr(NULL, 'x'), instr('abc', NULL), instr('Hello', 'he'), instr('Hello', 'He')",
    },
];

const MATH_FUNCTION_CASES: &[QueryCase] = &[
    QueryCase {
        name: "abs nulls and storage class",
        sql: "SELECT abs(-42), abs(42), abs(NULL), typeof(abs(-42))",
    },
    QueryCase {
        name: "sqrt pow and power",
        sql: "SELECT sqrt(144), pow(2, 10), power(3, 4)",
    },
    QueryCase {
        name: "logarithm exact powers",
        sql: "SELECT log(10), log(2, 8), log2(8), log10(1000), ln(1), exp(0)",
    },
    QueryCase {
        name: "rounding family positive and negative",
        sql: "SELECT ceil(1.2), ceiling(-1.2), floor(1.8), floor(-1.2), trunc(1.8), trunc(-1.8)",
    },
    QueryCase {
        name: "domain errors become null",
        sql: "SELECT sqrt(-1), log(-1), ln(0), acos(2), asin(2)",
    },
    QueryCase {
        name: "mod and angle conversion",
        sql: "SELECT mod(10, 3), mod(10, 0), degrees(0), radians(0)",
    },
];

const PATTERN_SETUP: &str = "
    CREATE TABLE patterns (
        id INTEGER PRIMARY KEY,
        name TEXT
    );
    INSERT INTO patterns VALUES
        (1, 'hello'),
        (2, 'HELLO'),
        (3, 'he_lo'),
        (4, 'he%llo'),
        (5, 'help'),
        (6, 'shell'),
        (7, '');
";

const PATTERN_CASES: &[QueryCase] = &[
    QueryCase {
        name: "like ascii case insensitive prefix",
        sql: "SELECT id FROM patterns WHERE name LIKE 'he%' ORDER BY id",
    },
    QueryCase {
        name: "like escape literal underscore",
        sql: "SELECT id FROM patterns WHERE name LIKE 'he!_%' ESCAPE '!' ORDER BY id",
    },
    QueryCase {
        name: "like escape literal percent",
        sql: "SELECT id FROM patterns WHERE name LIKE 'he!%%' ESCAPE '!' ORDER BY id",
    },
    QueryCase {
        name: "not like and null predicate",
        sql: "SELECT id FROM patterns WHERE name NOT LIKE 'he%' OR name LIKE NULL ORDER BY id",
    },
    QueryCase {
        name: "glob case sensitive prefix",
        sql: "SELECT id FROM patterns WHERE name GLOB 'he*' ORDER BY id",
    },
    QueryCase {
        name: "glob case sensitive uppercase prefix",
        sql: "SELECT id FROM patterns WHERE name GLOB 'HE*' ORDER BY id",
    },
    QueryCase {
        name: "glob question wildcard",
        sql: "SELECT id FROM patterns WHERE name GLOB 'he?lo' ORDER BY id",
    },
    QueryCase {
        name: "not glob case sensitive",
        sql: "SELECT id FROM patterns WHERE name NOT GLOB 'he*' ORDER BY id",
    },
    QueryCase {
        name: "scalar like glob null behavior",
        sql: "SELECT 'abc' LIKE 'ABC', 'abc' GLOB 'ABC', NULL LIKE 'a%', 'abc' GLOB NULL",
    },
];

const REGEXP_ERROR_CASES: &[StatementCase] = &[
    StatementCase {
        name: "regexp without registered function",
        sql: "SELECT 'abc' REGEXP 'a.*'",
    },
    StatementCase {
        name: "not regexp without registered function",
        sql: "SELECT 'abc' NOT REGEXP 'z.*'",
    },
    StatementCase {
        name: "table regexp without registered function",
        sql: "SELECT id FROM patterns WHERE name REGEXP '^he'",
    },
];

#[test]
fn select_join_group_by_aggregates_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(SALES_SETUP);
    harness.assert_queries_match(
        "SELECT/JOIN/GROUP BY/aggregate",
        SELECT_JOIN_GROUP_AGGREGATE_CASES,
    );
}

#[test]
fn aggregate_edge_cases_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(SALES_SETUP);
    harness.assert_queries_match("aggregate edge", AGGREGATE_EDGE_CASES);
}

#[test]
fn upsert_conflict_handling_matches_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(UPSERT_SETUP);
    harness.execute_script(UPSERT_SCRIPT);
    harness.assert_queries_match("UPSERT", UPSERT_CASES);
}

#[test]
fn with_upsert_returning_matches_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(WITH_UPSERT_RETURNING_SETUP);
    harness.assert_queries_match("WITH/UPSERT/RETURNING", WITH_UPSERT_RETURNING_CASES);
}

#[test]
fn cte_queries_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(CTE_SETUP);
    harness.assert_queries_match("CTE", CTE_CASES);
}

#[test]
fn window_functions_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(WINDOW_SETUP);
    harness.assert_queries_match("window", WINDOW_CASES);
}

#[test]
fn cast_and_collation_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(CAST_COLLATION_SETUP);
    harness.assert_queries_match("CAST/collation", CAST_COLLATION_CASES);
}

#[test]
fn collation_expression_edges_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(CAST_COLLATION_SETUP);
    harness.assert_queries_match("collation expression", COLLATION_EXPRESSION_CASES);
}

#[test]
fn dml_insert_update_delete_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(DML_SETUP);
    harness.execute_script(DML_SCRIPT);
    harness.assert_queries_match("DML", DML_CASES);
}

#[test]
fn attached_update_delegation_matches_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(ATTACHED_UPDATE_SETUP);
    harness.assert_queries_match("attached UPDATE", ATTACHED_UPDATE_CASES);
}

#[test]
fn attached_insert_select_delegation_matches_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(ATTACHED_INSERT_SELECT_SETUP);
    harness.assert_queries_match("attached INSERT SELECT", ATTACHED_INSERT_SELECT_CASES);
}

#[test]
fn attached_drop_delegation_matches_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(ATTACHED_DROP_SETUP);
    harness.execute_script(ATTACHED_DROP_SCRIPT);
    harness.assert_queries_match("attached DROP", ATTACHED_DROP_CASES);
}

#[test]
fn attached_create_view_delegation_matches_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(ATTACHED_CREATE_VIEW_SETUP);
    harness.assert_queries_match("attached CREATE VIEW", ATTACHED_CREATE_VIEW_CASES);
}

#[test]
fn attached_vacuum_delegation_matches_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(ATTACHED_VACUUM_SETUP);
    harness.assert_queries_match("attached VACUUM", ATTACHED_VACUUM_CASES);
}

#[test]
fn error_paths_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(ERROR_PATH_SETUP);
    harness.assert_statement_errors_match("error path", ERROR_PATH_CASES);
}

#[test]
fn ddl_defaults_and_views_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(DDL_SETUP);
    harness.assert_queries_match("DDL/default/view", DDL_CASES);
}

#[test]
fn transactions_and_savepoints_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(TRANSACTION_SETUP);
    harness.execute_script(TRANSACTION_SCRIPT);
    harness.assert_queries_match("transaction/savepoint", TRANSACTION_CASES);
}

#[test]
fn compound_selects_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(COMPOUND_SETUP);
    harness.assert_queries_match("compound SELECT", COMPOUND_CASES);
}

#[test]
fn case_and_null_logic_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(CASE_NULL_SETUP);
    harness.assert_queries_match("CASE/null logic", CASE_NULL_CASES);
}

#[test]
fn subqueries_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(SUBQUERY_SETUP);
    harness.assert_queries_match("subquery", SUBQUERY_CASES);
}

#[test]
fn pragmas_and_schema_introspection_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(PRAGMA_SETUP);
    harness.assert_queries_match("PRAGMA/schema introspection", PRAGMA_CASES);
}

#[test]
fn triggers_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(TRIGGER_SETUP);
    harness.execute_script(TRIGGER_SCRIPT);
    harness.assert_queries_match("trigger", TRIGGER_CASES);
}

#[test]
fn date_time_functions_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new("");
    harness.assert_queries_match("date/time", DATE_TIME_CASES);
}

#[test]
fn date_time_modifier_edges_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new("");
    harness.assert_queries_match("date/time modifier edge", DATE_TIME_EDGE_CASES);
}

#[test]
fn json1_functions_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new("");
    harness.assert_queries_match("JSON1", JSON1_CASES);
}

#[test]
fn string_functions_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new("");
    harness.assert_queries_match("string functions", STRING_FUNCTION_CASES);
}

#[test]
fn math_functions_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new("");
    harness.assert_queries_match("math functions", MATH_FUNCTION_CASES);
}

#[test]
fn pattern_matching_functions_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(PATTERN_SETUP);
    harness.assert_queries_match("LIKE/GLOB", PATTERN_CASES);
    harness.assert_statement_errors_match("REGEXP", REGEXP_ERROR_CASES);
}
