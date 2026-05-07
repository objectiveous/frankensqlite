# Retained autocommit prepared append hint

Status: kept.

Candidate: preserve `Connection::prepared_direct_insert_append_hint` when
`invalidate_cached_write_txn` is called but there is no cached writer to
invalidate. Retained `:memory:` autocommit prepared inserts reuse the retained
transaction, not `cached_write_txn`, so the unconditional clear forced every row
back onto full-cell assembly and cursor setup instead of the existing rightmost
leaf payload append path.

Correctness proof:

- `cargo fmt -p fsqlite-core --check`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-appendhint-crimsongorge-target cargo test -p fsqlite-core test_memory_retained_autocommit_preserves_prepared_insert_append_hint -- --nocapture`

Build proof:

- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-appendhint-crimsongorge-release cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`

Focused transaction-section gate:

- Baseline: `baseline-transaction.json`
- Candidate: `candidate-transaction.json`
- Command shape: `FSQLITE_BENCH_PROFILE_INSERT=1 comprehensive-bench --quick --filter transaction --json-out <out.json> --no-html`
- Baseline primary weighted score: `1.2162445262280113`
- Candidate primary weighted score: `0.916540477945728`
- Baseline geomean ratio: `1.087469767027892`
- Candidate geomean ratio: `1.0342998803808334`
- Baseline write-single geomean: `1.3007179177575143`
- Candidate write-single geomean: `0.852422206576282`
- Baseline C-faster / Franken-faster rows: `5 / 4`
- Candidate C-faster / Franken-faster rows: `3 / 5`

Autocommit hot-path evidence:

- 100 rows autocommit: FSQLite median `0.147036 ms -> 0.116028 ms`,
  ratio `1.1837503622838372 -> 0.9194196375508135`.
- 1000 rows autocommit: FSQLite median `1.123846 ms -> 0.739916 ms`,
  ratio `1.3507293615967713 -> 0.7883204524194597`.
- 10000 rows autocommit: FSQLite median `11.095064 ms -> 7.080644 ms`,
  ratio `1.3763246971265806 -> 0.8545701040392748`.
- 10000 rows autocommit profiler counters moved from
  `btree_cell_assembly_calls=10000`, `btree_leaf_payload_appends=0` to
  `btree_cell_assembly_calls=164`, `btree_leaf_payload_appends=9898`.

Full quick matrix gate:

- Baseline: `baseline-full.json`
- Candidate: `candidate-full.json`
- Command shape: `FSQLITE_BENCH_PROFILE_INSERT=1 comprehensive-bench --quick --json-out <out.json> --no-html`
- Baseline primary weighted score: `0.3673037319768524`
- Candidate primary weighted score: `0.35545807994845907`
- Baseline geomean ratio: `0.27430498790100927`
- Candidate geomean ratio: `0.27338133885996724`
- Baseline C-faster / Franken-faster rows: `14 / 73`
- Candidate C-faster / Franken-faster rows: `11 / 75`
- Baseline write-single geomean: `1.1618044085427914`
- Candidate write-single geomean: `1.003637642685041`
- Baseline write-bulk geomean: `0.915214838638832`
- Candidate write-bulk geomean: `0.9125165844314171`

Notes:

- The small 100-row batched transaction row remained slower, but the broader
  full matrix improved and the targeted autocommit rows moved decisively.
- The candidate does not disable concurrent writer defaults. It only avoids
  dropping a prepared append hint when the cached writer slot was already empty.
