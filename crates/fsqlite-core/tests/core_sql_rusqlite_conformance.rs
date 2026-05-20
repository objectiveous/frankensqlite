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

#[test]
fn select_join_group_by_aggregates_match_rusqlite() {
    let harness = CoreSqlConformanceHarness::new(SALES_SETUP);
    harness.assert_queries_match(
        "SELECT/JOIN/GROUP BY/aggregate",
        SELECT_JOIN_GROUP_AGGREGATE_CASES,
    );
}
