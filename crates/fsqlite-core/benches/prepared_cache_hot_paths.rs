use std::env;
use std::hint::black_box;
use std::time::Instant;

use fsqlite_core::connection::Connection;
use fsqlite_types::SqliteValue;
use tempfile::NamedTempFile;

const INSERT_SQL: &str = "INSERT INTO bench (id, payload) VALUES (?1, ?2)";

fn open_mt_mvcc_prepare_conn() -> (Connection, NamedTempFile) {
    let tmp = NamedTempFile::new().expect("tempfile");
    let path = tmp
        .path()
        .to_str()
        .expect("tempfile path must be utf-8")
        .to_owned();
    let conn = Connection::open(path).expect("open connection");
    conn.execute("CREATE TABLE bench (id INTEGER PRIMARY KEY, payload TEXT);")
        .expect("create table");
    conn.execute("BEGIN;").expect("begin transaction");
    (conn, tmp)
}

fn bench_mt_mvcc_prepare_hit(iterations: u64) -> f64 {
    let (conn, _tmp) = open_mt_mvcc_prepare_conn();
    let warmed = conn.prepare(INSERT_SQL).expect("warm prepare");
    black_box(&warmed);

    let start = Instant::now();
    for _ in 0..iterations {
        let stmt = conn.prepare(black_box(INSERT_SQL)).expect("prepare hit");
        black_box(stmt);
    }
    start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64
}

fn bench_mt_mvcc_prepare_then_execute_cycle(iterations: u64) -> f64 {
    let (conn, _tmp) = open_mt_mvcc_prepare_conn();
    let warmed = conn.prepare(INSERT_SQL).expect("warm prepare");
    let warmed_params = [
        SqliteValue::Integer(0),
        SqliteValue::Text(String::from("warmup").into()),
    ];
    warmed
        .execute_with_params(&warmed_params)
        .expect("warm execute");
    black_box(&warmed);

    let start = Instant::now();
    for row_id in 1..=iterations {
        let stmt = conn.prepare(black_box(INSERT_SQL)).expect("prepare hit");
        let params = [
            SqliteValue::Integer(i64::try_from(row_id).expect("row id fits i64")),
            SqliteValue::Text(format!("payload_{row_id}").into()),
        ];
        let inserted = stmt.execute_with_params(&params).expect("execute");
        black_box(inserted);
    }
    start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64
}

fn parse_iterations() -> u64 {
    let mut args = env::args().skip(1);
    let mut iterations = 2_000_000_u64;
    let mut filter = None;
    while let Some(arg) = args.next() {
        if arg == "--iterations" {
            if let Some(value) = args.next() {
                match value.parse() {
                    Ok(parsed) => iterations = parsed,
                    Err(_) => {
                        eprintln!("invalid --iterations value: {value}");
                        std::process::exit(2);
                    }
                }
            }
        } else if arg == "--filter" {
            filter = args.next();
        }
    }
    if let Some(filter) = filter {
        match filter.as_str() {
            "prepare_hit" => {
                let prepare_hit_ns = bench_mt_mvcc_prepare_hit(iterations);
                println!(
                    "prepared_cache_hot_paths mt_mvcc_prepare_hit_ns_per_op={prepare_hit_ns:.2} iterations={iterations}"
                );
                std::process::exit(0);
            }
            "prepare_execute" => {
                let prepare_execute_ns =
                    bench_mt_mvcc_prepare_then_execute_cycle(iterations.min(200_000));
                println!(
                    "prepared_cache_hot_paths mt_mvcc_prepare_then_execute_cycle_ns_per_op={prepare_execute_ns:.2} iterations={}",
                    iterations.min(200_000)
                );
                std::process::exit(0);
            }
            _ => {
                eprintln!("invalid --filter value: {filter}");
                std::process::exit(2);
            }
        }
    }
    iterations
}

fn main() {
    let iterations = parse_iterations();
    let prepare_hit_ns = bench_mt_mvcc_prepare_hit(iterations);
    let prepare_execute_ns = bench_mt_mvcc_prepare_then_execute_cycle(iterations.min(200_000));

    println!(
        "prepared_cache_hot_paths mt_mvcc_prepare_hit_ns_per_op={prepare_hit_ns:.2} iterations={iterations}"
    );
    println!(
        "prepared_cache_hot_paths mt_mvcc_prepare_then_execute_cycle_ns_per_op={prepare_execute_ns:.2} iterations={}",
        iterations.min(200_000)
    );
}
