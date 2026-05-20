use fsqlite_core::connection::Connection;
use fsqlite_types::value::SqliteValue;

#[derive(Clone, Copy)]
struct QueryCase {
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

#[test]
fn select_join_group_by_aggregates_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(SALES_SETUP);
    harness.assert_queries_match(
        "SELECT/JOIN/GROUP BY/aggregate",
        SELECT_JOIN_GROUP_AGGREGATE_CASES,
    );
}

#[test]
fn upsert_conflict_handling_matches_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(UPSERT_SETUP);
    harness.execute_script(UPSERT_SCRIPT);
    harness.assert_queries_match("UPSERT", UPSERT_CASES);
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
fn dml_insert_update_delete_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(DML_SETUP);
    harness.execute_script(DML_SCRIPT);
    harness.assert_queries_match("DML", DML_CASES);
}

#[test]
fn subqueries_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(SUBQUERY_SETUP);
    harness.assert_queries_match("subquery", SUBQUERY_CASES);
}
