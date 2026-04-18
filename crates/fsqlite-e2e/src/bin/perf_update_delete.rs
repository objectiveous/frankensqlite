//! Narrow profiling binary for UPDATE/DELETE fsqlite hot path.
//!
//! Runs the same fsqlite UPDATE/DELETE workload as `comprehensive-bench`'s
//! Section 6, but without the C SQLite comparison or the benchmark reporting
//! ceremony, so perf/flamegraph stacks stay focused on the fsqlite engine.
//!
//! Usage:
//!   perf-update-delete                 # default: 10_000 rows, 10 iters, update+delete
//!   perf-update-delete 100000 3 update
//!   perf-update-delete 1000   5 delete
//!
//! Arguments:
//!   [rows]   Number of rows to pre-populate (default 10_000)
//!   [iters]  Number of outer iterations for profiling (default 10)
//!   [which]  "update" | "delete" | "both" (default "both")

use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let rows: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let iters: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(10);
    let which: String = args.next().unwrap_or_else(|| "both".to_owned());

    let do_update = which != "delete";
    let do_delete = which != "update";

    eprintln!(
        "perf-update-delete: rows={rows} iters={iters} which={which} \
        (do_update={do_update} do_delete={do_delete})",
    );

    let update_count = rows / 10;
    let delete_count = rows / 20;

    let t_all = Instant::now();
    let mut total_update_ns: u128 = 0;
    let mut total_delete_ns: u128 = 0;
    let mut total_populate_ns: u128 = 0;

    for iter in 0..iters {
        let conn = fsqlite::Connection::open(":memory:").unwrap();
        conn.execute(
            "CREATE TABLE bench (\
                id INTEGER PRIMARY KEY,\
                value REAL,\
                name TEXT\
            );",
        )
        .unwrap();
        conn.execute("BEGIN").unwrap();
        let stmt = conn
            .prepare("INSERT INTO bench(id, value, name) VALUES (?1, ?1, 'row');")
            .unwrap();
        let t0 = Instant::now();
        #[allow(clippy::cast_possible_wrap)]
        for i in 0..rows as i64 {
            stmt.execute_with_params(&[fsqlite::SqliteValue::Integer(i)])
                .unwrap();
        }
        conn.execute("COMMIT").unwrap();
        total_populate_ns += t0.elapsed().as_nanos();

        if do_update {
            conn.execute("BEGIN").unwrap();
            let update = conn
                .prepare("UPDATE bench SET value = ?2 WHERE id = ?1")
                .unwrap();
            let t0 = Instant::now();
            #[allow(clippy::cast_possible_wrap)]
            for i in 0..update_count as i64 {
                let id = i * 10;
                update
                    .execute_with_params(&[
                        fsqlite::SqliteValue::Integer(id),
                        fsqlite::SqliteValue::Float(999.99),
                    ])
                    .unwrap();
            }
            conn.execute("COMMIT").unwrap();
            total_update_ns += t0.elapsed().as_nanos();
        }

        if do_delete {
            conn.execute("BEGIN").unwrap();
            let delete = conn.prepare("DELETE FROM bench WHERE id = ?1").unwrap();
            let t0 = Instant::now();
            #[allow(clippy::cast_possible_wrap)]
            for i in 0..delete_count as i64 {
                let id = i * 20;
                delete
                    .execute_with_params(&[fsqlite::SqliteValue::Integer(id)])
                    .unwrap();
            }
            conn.execute("COMMIT").unwrap();
            total_delete_ns += t0.elapsed().as_nanos();
        }

        if iter == 0 {
            eprintln!("  (first iter complete)");
        }
    }

    let total_ns = t_all.elapsed().as_nanos();
    let per_row_update = if do_update {
        total_update_ns as f64 / (update_count * iters) as f64
    } else {
        0.0
    };
    let per_row_delete = if do_delete {
        total_delete_ns as f64 / (delete_count * iters) as f64
    } else {
        0.0
    };
    eprintln!(
        "total={}ms populate={}ms update={}ms delete={}ms  |  \
        per-row-update={per_row_update:.0}ns  per-row-delete={per_row_delete:.0}ns",
        total_ns / 1_000_000,
        total_populate_ns / 1_000_000,
        total_update_ns / 1_000_000,
        total_delete_ns / 1_000_000,
    );
}
