//! `mt-mvcc-bench` — real multi-threaded MVCC writer benchmark (IMPL-4a).
//!
//! Why this exists: `comprehensive_bench::bench_concurrent_writers` runs
//! FrankenSQLite writers *sequentially* on ONE `Connection` because
//! `Connection` is `!Send + !Sync`. That means the previously-reported
//! "concurrent" FrankenSQLite numbers were really single-threaded loops
//! compared against genuinely multi-threaded C SQLite WAL. This bench fixes
//! that by spawning N OS threads, each with its OWN `Connection::open(path)`
//! against the SAME shared file-backed database, so the MVCC page-lock
//! table, commit coordinator, and SSI validator are exercised under real
//! contention.
//!
//! For each thread count we measure:
//!   - FrankenSQLite file-backed database, one Connection per thread,
//!     `PRAGMA fsqlite.concurrent_mode=ON` + `BEGIN CONCURRENT`.
//!   - C SQLite (rusqlite) file-backed WAL, one Connection per thread,
//!     `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`.
//!
//! Each thread inserts `--rows-per-thread` rows into the shared table
//! `bench(id INTEGER PRIMARY KEY, payload TEXT)` using disjoint rowid
//! ranges (`thread_id * 1_000_000 + i`) so there are no logical row
//! conflicts — only page-level contention on the table's btree.
//!
//! Output is a tab-separated table suitable for grepping / redirection:
//!
//! ```text
//! threads | fsqlite_wps | sqlite_wps | throughput_ratio | fsqlite_ms_p50 | ...
//!       1 | 12345       | 23456      | 0.53x            | 81.00          | ...
//! ```
//!
//! `throughput_ratio = fsqlite_wps / sqlite_wps`. Values above 1.0x mean
//! FrankenSQLite is faster than C SQLite WAL under equal multi-threaded load.
//! `time_ratio = fsqlite_batch_ms / sqlite_batch_ms`; lower is better.
//!
//! ## CLI
//!
//! ```text
//! mt-mvcc-bench [--rows-per-thread=1000] [--threads=1,2,4,8,16] [--iters=3]
//! [--json-output=PATH] [--summary-md=PATH]
//! ```
//!
//! ## Caveats
//!
//! * `BEGIN CONCURRENT` requires `PRAGMA fsqlite.concurrent_mode=ON;` to be
//!   set on each per-thread connection (see
//!   `crates/fsqlite-harness/tests/bd_3plop_4_lock_contention_storms.rs`).
//!   If that PRAGMA fails on a given build, we fall back to plain `BEGIN`
//!   and print a warning (honest measurement over a fake win).
//! * We retry transient errors (`FrankenError::is_transient()`) per-row up
//!   to `MAX_RETRIES`; hard failures are counted in `failed_rows` and
//!   included in the report so you can tell when the numbers are bogus.
//! * Each iteration creates a fresh tempfile so there's no state carried
//!   across runs. `--iters=3` reports p50/p95/p99 across those 3 samples.

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use std::{fmt::Write as _, fs, path::Path, path::PathBuf};

use serde::Serialize;

// ─── Defaults ─────────────────────────────────────────────────────────────

const DEFAULT_ROWS_PER_THREAD: usize = 1_000;
const DEFAULT_THREADS: &[usize] = &[1, 2, 4, 8, 16];
const DEFAULT_ITERS: usize = 3;
const ROWID_BASE_STRIDE: i64 = 1_000_000;
const MAX_RETRIES: usize = 32;
const RETRY_SLEEP_MS: u64 = 1;
const REPORT_SCHEMA_V1: &str = "fsqlite-e2e.mt_mvcc_bench_report.v1";

// ─── CLI parsing (manual — no clap in workspace) ─────────────────────────

#[derive(Debug, Clone)]
struct Options {
    rows_per_thread: usize,
    threads: Vec<usize>,
    iters: usize,
    json_output: Option<PathBuf>,
    summary_md: Option<PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            rows_per_thread: DEFAULT_ROWS_PER_THREAD,
            threads: DEFAULT_THREADS.to_vec(),
            iters: DEFAULT_ITERS,
            json_output: None,
            summary_md: None,
        }
    }
}

fn print_usage_and_exit(code: i32) -> ! {
    eprintln!(
        "usage: mt-mvcc-bench [--rows-per-thread=N] [--threads=N,N,...] [--iters=N] \\\n\
         [--json-output=PATH] [--summary-md=PATH]\n\
         \n\
         defaults: --rows-per-thread={DEFAULT_ROWS_PER_THREAD} \
         --threads=1,2,4,8,16 --iters={DEFAULT_ITERS}"
    );
    std::process::exit(code);
}

fn parse_args() -> Options {
    let mut opts = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let (key, val) = if let Some(eq) = arg.find('=') {
            (arg[..eq].to_owned(), arg[eq + 1..].to_owned())
        } else if arg == "--help" || arg == "-h" {
            print_usage_and_exit(0);
        } else {
            // Support space-separated form.
            let v = args
                .next()
                .unwrap_or_else(|| panic!("missing value for argument `{arg}`"));
            (arg, v)
        };
        match key.as_str() {
            "--rows-per-thread" => {
                opts.rows_per_thread = val
                    .parse()
                    .unwrap_or_else(|_| panic!("invalid --rows-per-thread: {val}"));
            }
            "--threads" => {
                opts.threads = val
                    .split(',')
                    .map(|s| {
                        s.trim()
                            .parse::<usize>()
                            .unwrap_or_else(|_| panic!("invalid thread count in --threads: {s}"))
                    })
                    .collect();
                if opts.threads.is_empty() {
                    panic!("--threads must contain at least one value");
                }
            }
            "--iters" => {
                opts.iters = val
                    .parse()
                    .unwrap_or_else(|_| panic!("invalid --iters: {val}"));
                if opts.iters == 0 {
                    panic!("--iters must be >= 1");
                }
            }
            "--json-output" => {
                opts.json_output = Some(PathBuf::from(val));
            }
            "--summary-md" => {
                opts.summary_md = Some(PathBuf::from(val));
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_usage_and_exit(2);
            }
        }
    }
    opts
}

// ─── Reported per-config result ───────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct RunResult {
    /// Wall-clock duration across threads (max of per-thread times), best of
    /// `iters` iterations.
    best_elapsed: Duration,
    /// Total rows written (across all threads) in the best iteration.
    total_rows: usize,
    /// Total rows that hit a hard failure after `MAX_RETRIES`.
    failed_rows: usize,
}

impl RunResult {
    fn writes_per_sec(&self) -> f64 {
        let secs = self.best_elapsed.as_secs_f64();
        if secs <= 0.0 {
            0.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            let n = self.total_rows as f64;
            n / secs
        }
    }

    fn elapsed_ms(&self) -> f64 {
        self.best_elapsed.as_secs_f64() * 1_000.0
    }
}

#[derive(Debug, Clone)]
struct RunStats {
    samples: Vec<RunResult>,
}

impl RunStats {
    fn new(samples: Vec<RunResult>) -> Self {
        Self { samples }
    }

    fn total_failed_rows(&self) -> usize {
        self.samples.iter().map(|sample| sample.failed_rows).sum()
    }

    fn p50_writes_per_sec(&self) -> f64 {
        self.percentile_by(RunResult::writes_per_sec, 0.50)
    }

    fn p95_writes_per_sec(&self) -> f64 {
        self.percentile_by(RunResult::writes_per_sec, 0.95)
    }

    fn p99_writes_per_sec(&self) -> f64 {
        self.percentile_by(RunResult::writes_per_sec, 0.99)
    }

    fn p50_elapsed_ms(&self) -> f64 {
        self.percentile_by(RunResult::elapsed_ms, 0.50)
    }

    fn p95_elapsed_ms(&self) -> f64 {
        self.percentile_by(RunResult::elapsed_ms, 0.95)
    }

    fn p99_elapsed_ms(&self) -> f64 {
        self.percentile_by(RunResult::elapsed_ms, 0.99)
    }

    fn percentile_by(&self, value: fn(&RunResult) -> f64, percentile: f64) -> f64 {
        let values = self.samples.iter().map(value).collect();
        percentile_value(values, percentile)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct ThreadComparisonReport {
    threads: usize,
    fsqlite_wps_p50: f64,
    fsqlite_wps_p95: f64,
    fsqlite_wps_p99: f64,
    sqlite_wps_p50: f64,
    sqlite_wps_p95: f64,
    sqlite_wps_p99: f64,
    throughput_ratio: f64,
    fsqlite_ms_p50: f64,
    fsqlite_ms_p95: f64,
    fsqlite_ms_p99: f64,
    sqlite_ms_p50: f64,
    sqlite_ms_p95: f64,
    sqlite_ms_p99: f64,
    time_ratio: f64,
    fsqlite_failed_rows: usize,
    sqlite_failed_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct MtMvccBenchReport {
    schema_version: &'static str,
    rows_per_thread: usize,
    iterations: usize,
    thread_results: Vec<ThreadComparisonReport>,
}

fn build_thread_report(
    threads: usize,
    fsqlite: &RunStats,
    sqlite: &RunStats,
) -> ThreadComparisonReport {
    let fsqlite_wps_p50 = fsqlite.p50_writes_per_sec();
    let sqlite_wps_p50 = sqlite.p50_writes_per_sec();
    let throughput_ratio = if sqlite_wps_p50 > 0.0 {
        fsqlite_wps_p50 / sqlite_wps_p50
    } else {
        0.0
    };
    let fsqlite_ms_p50 = fsqlite.p50_elapsed_ms();
    let sqlite_ms_p50 = sqlite.p50_elapsed_ms();
    let time_ratio = if sqlite_ms_p50 > 0.0 {
        fsqlite_ms_p50 / sqlite_ms_p50
    } else {
        0.0
    };

    ThreadComparisonReport {
        threads,
        fsqlite_wps_p50,
        fsqlite_wps_p95: fsqlite.p95_writes_per_sec(),
        fsqlite_wps_p99: fsqlite.p99_writes_per_sec(),
        sqlite_wps_p50,
        sqlite_wps_p95: sqlite.p95_writes_per_sec(),
        sqlite_wps_p99: sqlite.p99_writes_per_sec(),
        throughput_ratio,
        fsqlite_ms_p50,
        fsqlite_ms_p95: fsqlite.p95_elapsed_ms(),
        fsqlite_ms_p99: fsqlite.p99_elapsed_ms(),
        sqlite_ms_p50,
        sqlite_ms_p95: sqlite.p95_elapsed_ms(),
        sqlite_ms_p99: sqlite.p99_elapsed_ms(),
        time_ratio,
        fsqlite_failed_rows: fsqlite.total_failed_rows(),
        sqlite_failed_rows: sqlite.total_failed_rows(),
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create parent directory {}: {error}", parent.display()))?;
    }
    Ok(())
}

fn render_markdown_summary(report: &MtMvccBenchReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# mt-mvcc-bench Summary\n");
    let _ = writeln!(out, "- Rows per thread: `{}`", report.rows_per_thread);
    let _ = writeln!(out, "- Iterations: `{}`", report.iterations);
    let _ = writeln!(out, "- Schema: `{}`\n", report.schema_version);
    let _ = writeln!(
        out,
        "| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |"
    );
    let _ = writeln!(
        out,
        "|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|"
    );
    for row in &report.thread_results {
        let _ = writeln!(
            out,
            "| {} | {:.0} | {:.0} | {:.2}x | {:.2} | {:.2} | {:.2}x | {} | {} |",
            row.threads,
            row.fsqlite_wps_p50,
            row.sqlite_wps_p50,
            row.throughput_ratio,
            row.fsqlite_ms_p50,
            row.sqlite_ms_p50,
            row.time_ratio,
            row.fsqlite_failed_rows,
            row.sqlite_failed_rows
        );
    }
    out
}

fn write_json_report(path: &Path, report: &MtMvccBenchReport) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let json = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("serialize mt-mvcc bench report: {error}"))?;
    fs::write(path, json).map_err(|error| format!("write json report {}: {error}", path.display()))
}

fn write_markdown_summary(path: &Path, report: &MtMvccBenchReport) -> Result<(), String> {
    ensure_parent_dir(path)?;
    fs::write(path, render_markdown_summary(report))
        .map_err(|error| format!("write markdown summary {}: {error}", path.display()))
}

#[allow(clippy::cast_precision_loss)]
fn percentile_value(mut values: Vec<f64>, percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    if values.len() == 1 {
        return values[0];
    }
    let rank = percentile.clamp(0.0, 1.0) * (values.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        return values[lower];
    }
    let fraction = rank - lower as f64;
    (values[upper] - values[lower]).mul_add(fraction, values[lower])
}

// ─── FrankenSQLite workload ──────────────────────────────────────────────

fn run_fsqlite(threads: usize, rows_per_thread: usize) -> RunResult {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let path = tmp
        .path()
        .to_str()
        .expect("tempfile path is utf-8")
        .to_owned();

    // Initialise schema with a single connection before spawning workers.
    {
        let conn = fsqlite::Connection::open(path.clone()).expect("fsqlite open (init)");
        for pragma in [
            "PRAGMA page_size=4096;",
            "PRAGMA journal_mode=WAL;",
            "PRAGMA synchronous=NORMAL;",
            "PRAGMA cache_size=-64000;",
        ] {
            let _ = conn.execute(pragma);
        }
        conn.execute("CREATE TABLE IF NOT EXISTS bench (id INTEGER PRIMARY KEY, payload TEXT)")
            .expect("create table");
    }

    let path = Arc::new(path);
    let barrier = Arc::new(Barrier::new(threads));
    let mut handles = Vec::with_capacity(threads);

    let t0 = Instant::now();
    for tid in 0..threads {
        let path = Arc::clone(&path);
        let barrier = Arc::clone(&barrier);
        let handle = thread::spawn(move || -> (Duration, usize) {
            // Each thread owns its own Connection (Connection: !Send + !Sync).
            let conn =
                fsqlite::Connection::open(path.as_str().to_owned()).expect("fsqlite open (worker)");
            // MVCC + BEGIN CONCURRENT opt-in.
            let concurrent_ok = conn.execute("PRAGMA fsqlite.concurrent_mode=ON;").is_ok();
            let _ = conn.execute("PRAGMA busy_timeout=5000;");

            barrier.wait();
            let start = Instant::now();

            #[allow(clippy::cast_possible_wrap)]
            let base = tid as i64 * ROWID_BASE_STRIDE;
            let mut failed = 0usize;

            // Prepare the INSERT once per transaction attempt; bind params per
            // iteration. This matches the rusqlite reference loop (L412-446
            // below) so both sides parse+plan the insert a single time and
            // the per-row cost is just bind + execute.
            //
            // Using `format!` per-iter on the fsqlite side was an
            // apples-to-oranges artifact that pinned `Lexer::tokenize_into`
            // at 2.53% self-time and drove 12%+ allocator churn on MT 8t
            // (2026-04-23 capture `fsqlite-t3b-validation-185110`).
            let insert_sql = "INSERT INTO bench (id, payload) VALUES (?1, ?2)";

            // Single transaction spanning all rows; retry on transient
            // conflicts by rolling back and reopening the transaction.
            let mut retry_count = 0usize;
            'outer: loop {
                let begin_sql = if concurrent_ok {
                    "BEGIN CONCURRENT"
                } else {
                    "BEGIN"
                };
                if let Err(e) = conn.execute(begin_sql) {
                    if e.is_transient() && retry_count < MAX_RETRIES {
                        retry_count += 1;
                        thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                        continue;
                    }
                    eprintln!("[fsqlite t{tid}] BEGIN failed: {e}");
                    return (start.elapsed(), rows_per_thread);
                }

                let stmt = match conn.prepare(insert_sql) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("[fsqlite t{tid}] prepare failed: {e}");
                        let _ = conn.execute("ROLLBACK");
                        return (start.elapsed(), rows_per_thread);
                    }
                };

                #[allow(clippy::cast_possible_wrap)]
                for i in 0..rows_per_thread as i64 {
                    let id = base + i;
                    let payload = format!("tid{tid}_i{i}");
                    let params = [
                        fsqlite::SqliteValue::Integer(id),
                        fsqlite::SqliteValue::Text(payload.into()),
                    ];
                    match stmt.execute_with_params(&params) {
                        Ok(_) => {}
                        Err(e) if e.is_transient() && retry_count < MAX_RETRIES => {
                            let _ = conn.execute("ROLLBACK");
                            retry_count += 1;
                            thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                            continue 'outer;
                        }
                        Err(e) => {
                            eprintln!("[fsqlite t{tid}] INSERT {id} failed: {e}");
                            failed += 1;
                        }
                    }
                }

                match conn.execute("COMMIT") {
                    Ok(_) => break 'outer,
                    Err(e) if e.is_transient() && retry_count < MAX_RETRIES => {
                        let _ = conn.execute("ROLLBACK");
                        retry_count += 1;
                        thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                    }
                    Err(e) => {
                        eprintln!("[fsqlite t{tid}] COMMIT failed: {e}");
                        let _ = conn.execute("ROLLBACK");
                        failed += rows_per_thread;
                        break 'outer;
                    }
                }
            }

            (start.elapsed(), failed)
        });
        handles.push(handle);
    }

    let mut total_failed = 0usize;
    for h in handles {
        let (_d, failed) = h.join().expect("thread join");
        total_failed += failed;
    }
    let elapsed = t0.elapsed();

    RunResult {
        best_elapsed: elapsed,
        total_rows: threads * rows_per_thread,
        failed_rows: total_failed,
    }
}

// ─── C SQLite (rusqlite) workload ────────────────────────────────────────

fn run_rusqlite(threads: usize, rows_per_thread: usize) -> RunResult {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let path = tmp
        .path()
        .to_str()
        .expect("tempfile path is utf-8")
        .to_owned();

    {
        let conn = rusqlite::Connection::open(&path).expect("rusqlite open (init)");
        conn.execute_batch(
            "PRAGMA page_size=4096;\
             PRAGMA journal_mode=WAL;\
             PRAGMA synchronous=NORMAL;\
             PRAGMA cache_size=-64000;\
             CREATE TABLE IF NOT EXISTS bench (id INTEGER PRIMARY KEY, payload TEXT);",
        )
        .expect("init schema");
    }

    let path = Arc::new(path);
    let barrier = Arc::new(Barrier::new(threads));
    let mut handles = Vec::with_capacity(threads);

    let t0 = Instant::now();
    for tid in 0..threads {
        let path = Arc::clone(&path);
        let barrier = Arc::clone(&barrier);
        let handle = thread::spawn(move || -> usize {
            use rusqlite::OpenFlags;
            let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX;
            let conn = rusqlite::Connection::open_with_flags(path.as_str(), flags)
                .expect("rusqlite open (worker)");
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;\
                 PRAGMA synchronous=NORMAL;\
                 PRAGMA busy_timeout=5000;",
            )
            .expect("worker pragmas");

            barrier.wait();

            #[allow(clippy::cast_possible_wrap)]
            let base = tid as i64 * ROWID_BASE_STRIDE;
            let mut failed = 0usize;

            conn.execute_batch("BEGIN").expect("BEGIN");
            {
                let mut stmt = conn
                    .prepare("INSERT INTO bench (id, payload) VALUES (?1, ?2)")
                    .expect("prepare");
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..rows_per_thread as i64 {
                    let id = base + i;
                    let payload = format!("tid{tid}_i{i}");
                    let mut retry = 0usize;
                    loop {
                        match stmt.execute(rusqlite::params![id, &payload]) {
                            Ok(_) => break,
                            Err(e) => {
                                if retry < MAX_RETRIES
                                    && matches!(
                                        e.sqlite_error_code(),
                                        Some(
                                            rusqlite::ErrorCode::DatabaseBusy
                                                | rusqlite::ErrorCode::DatabaseLocked
                                        )
                                    )
                                {
                                    retry += 1;
                                    thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                                    continue;
                                }
                                eprintln!("[sqlite t{tid}] INSERT {id} failed: {e}");
                                failed += 1;
                                break;
                            }
                        }
                    }
                }
            }
            // Retry COMMIT on Busy — WAL writer serialisation can race.
            let mut retry = 0usize;
            loop {
                match conn.execute_batch("COMMIT") {
                    Ok(()) => break,
                    Err(e) => {
                        if retry < MAX_RETRIES
                            && matches!(
                                e.sqlite_error_code(),
                                Some(
                                    rusqlite::ErrorCode::DatabaseBusy
                                        | rusqlite::ErrorCode::DatabaseLocked
                                )
                            )
                        {
                            retry += 1;
                            thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                            continue;
                        }
                        eprintln!("[sqlite t{tid}] COMMIT failed: {e}");
                        let _ = conn.execute_batch("ROLLBACK");
                        failed += rows_per_thread;
                        break;
                    }
                }
            }

            failed
        });
        handles.push(handle);
    }

    let mut total_failed = 0usize;
    for h in handles {
        let failed = h.join().expect("thread join");
        total_failed += failed;
    }
    let elapsed = t0.elapsed();

    RunResult {
        best_elapsed: elapsed,
        total_rows: threads * rows_per_thread,
        failed_rows: total_failed,
    }
}

// ─── Driver ───────────────────────────────────────────────────────────────

fn collect_samples<F: FnMut() -> RunResult>(iters: usize, mut f: F) -> RunStats {
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters {
        samples.push(f());
    }
    RunStats::new(samples)
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn run() -> Result<(), String> {
    let opts = parse_args();

    eprintln!(
        "mt-mvcc-bench: rows_per_thread={} threads={:?} iters={}",
        opts.rows_per_thread, opts.threads, opts.iters,
    );

    println!(
        "threads | fsqlite_wps | sqlite_wps | throughput_ratio | fsqlite_wps_p95 | fsqlite_wps_p99 | sqlite_wps_p95 | sqlite_wps_p99 | fsqlite_ms_p50 | fsqlite_ms_p95 | fsqlite_ms_p99 | sqlite_ms_p50 | sqlite_ms_p95 | sqlite_ms_p99 | time_ratio | fsqlite_failed | sqlite_failed"
    );
    let mut thread_results = Vec::new();
    for &n in &opts.threads {
        if n == 0 {
            continue;
        }
        let fs = collect_samples(opts.iters, || run_fsqlite(n, opts.rows_per_thread));
        let cs = collect_samples(opts.iters, || run_rusqlite(n, opts.rows_per_thread));
        let report = build_thread_report(n, &fs, &cs);

        println!(
            "{n:>7} | {fs_wps:>11.0} | {cs_wps:>10.0} | {throughput_ratio:>16.2}x | {fs_wps_p95:>15.0} | {fs_wps_p99:>15.0} | {sqlite_wps_p95:>14.0} | {sqlite_wps_p99:>14.0} | {fs_ms_p50:>14.2} | {fs_ms_p95:>14.2} | {fs_ms_p99:>14.2} | {sqlite_ms_p50:>13.2} | {sqlite_ms_p95:>13.2} | {sqlite_ms_p99:>13.2} | {time_ratio:>10.2}x | {fs_failed:>14} | {sqlite_failed:>13}",
            fs_wps = report.fsqlite_wps_p50,
            cs_wps = report.sqlite_wps_p50,
            throughput_ratio = report.throughput_ratio,
            fs_wps_p95 = report.fsqlite_wps_p95,
            fs_wps_p99 = report.fsqlite_wps_p99,
            sqlite_wps_p95 = report.sqlite_wps_p95,
            sqlite_wps_p99 = report.sqlite_wps_p99,
            fs_ms_p50 = report.fsqlite_ms_p50,
            fs_ms_p95 = report.fsqlite_ms_p95,
            fs_ms_p99 = report.fsqlite_ms_p99,
            sqlite_ms_p50 = report.sqlite_ms_p50,
            sqlite_ms_p95 = report.sqlite_ms_p95,
            sqlite_ms_p99 = report.sqlite_ms_p99,
            time_ratio = report.time_ratio,
            fs_failed = report.fsqlite_failed_rows,
            sqlite_failed = report.sqlite_failed_rows
        );
        thread_results.push(report);
    }

    let full_report = MtMvccBenchReport {
        schema_version: REPORT_SCHEMA_V1,
        rows_per_thread: opts.rows_per_thread,
        iterations: opts.iters,
        thread_results,
    };

    if let Some(path) = opts.json_output.as_deref() {
        write_json_report(path, &full_report)?;
        eprintln!("mt-mvcc-bench: wrote json report {}", path.display());
    }
    if let Some(path) = opts.summary_md.as_deref() {
        write_markdown_summary(path, &full_report)?;
        eprintln!("mt-mvcc-bench: wrote markdown summary {}", path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_stats(elapsed_ms: u64, total_rows: usize, failed_rows: usize) -> RunStats {
        RunStats::new(vec![RunResult {
            best_elapsed: Duration::from_millis(elapsed_ms),
            total_rows,
            failed_rows,
        }])
    }

    #[test]
    fn thread_report_computes_expected_ratios() {
        let fsqlite = sample_stats(200, 1000, 3);
        let sqlite = sample_stats(100, 1000, 1);

        let report = build_thread_report(4, &fsqlite, &sqlite);

        assert_eq!(report.threads, 4);
        assert!((report.fsqlite_wps_p50 - 5000.0).abs() < 0.01);
        assert!((report.sqlite_wps_p50 - 10_000.0).abs() < 0.01);
        assert!((report.throughput_ratio - 0.5).abs() < 0.0001);
        assert!((report.time_ratio - 2.0).abs() < 0.0001);
        assert_eq!(report.fsqlite_failed_rows, 3);
        assert_eq!(report.sqlite_failed_rows, 1);
    }

    #[test]
    fn markdown_summary_renders_thread_rows() {
        let report = MtMvccBenchReport {
            schema_version: REPORT_SCHEMA_V1,
            rows_per_thread: 250,
            iterations: 1,
            thread_results: vec![ThreadComparisonReport {
                threads: 8,
                fsqlite_wps_p50: 6090.0,
                fsqlite_wps_p95: 6090.0,
                fsqlite_wps_p99: 6090.0,
                sqlite_wps_p50: 55_406.0,
                sqlite_wps_p95: 55_406.0,
                sqlite_wps_p99: 55_406.0,
                throughput_ratio: 0.11,
                fsqlite_ms_p50: 328.39,
                fsqlite_ms_p95: 328.39,
                fsqlite_ms_p99: 328.39,
                sqlite_ms_p50: 36.10,
                sqlite_ms_p95: 36.10,
                sqlite_ms_p99: 36.10,
                time_ratio: 9.10,
                fsqlite_failed_rows: 0,
                sqlite_failed_rows: 0,
            }],
        };

        let rendered = render_markdown_summary(&report);

        assert!(rendered.contains("# mt-mvcc-bench Summary"));
        assert!(rendered.contains("| 8 | 6090 | 55406 | 0.11x |"));
    }
}
