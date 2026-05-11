# INSERT bulk page-run split profile

Run date: 2026-05-11T00:37:20Z

Base commit: `07d7874e95cedc2029d5b00d2938774511511651`

Source state: dirty by design. The only tracked source delta was profiling instrumentation for bulk INSERT page-run grouping, leaf-page build/write, and interior/root build/write.

Command:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-insert-bulk-page-split-target/release-perf/comprehensive-bench \
  --quick --filter insert \
  --json-out tests/artifacts/perf/codex-insert-bulk-page-split-20260511T003720Z/insert-profile.json \
  --no-html
```

Environment:

- CPU: AMD Ryzen Threadripper PRO 5995WX 64-Cores
- Rust: `rustc 1.97.0-nightly (82bee9650 2026-05-09)`
- Build profile: `release-perf`

Summary:

- Scenarios: 25
- FSQLite faster: 17
- Comparable: 1
- C SQLite faster: 7
- Geomean ratio: 0.8349934368942403
- Median ratio: 0.8092369571662344
- P90 ratio: 1.1632724358315694
- P99 ratio: 1.1680005353319058
- Weighted score: 0.8270819164706716

Rows still slower than C SQLite:

| Scenario | FSQLite/C SQLite | FSQLite median ms | C SQLite median ms |
| --- | ---: | ---: | ---: |
| `insertthroughput-single-transaction-tiny-1col__100-rows` | 1.0634515173188923 | 0.07285599999999999 | 0.068509 |
| `insertthroughput-single-transaction-small-3col__100-rows` | 1.1680005353319058 | 0.087273 | 0.07472000000000001 |
| `insertthroughput-single-transaction-medium-6col__100-rows` | 1.1089407210838205 | 0.11115799999999999 | 0.100238 |
| `insertthroughput-single-transaction-large-10col__100-rows` | 1.1632724358315694 | 0.169818 | 0.145983 |
| `insertthroughput-single-transaction-large-10col__10000-rows` | 1.0622245163013286 | 10.118721 | 9.525972000000001 |
| `insertthroughput-transaction-strategy-comparison-small-3col__100-rows-batched-100-txn` | 1.1667531524051684 | 0.089938 | 0.077084 |
| `insertthroughput-transaction-strategy-comparison-small-3col__100-rows-single-txn` | 1.1605891260938903 | 0.086602 | 0.074619 |

Selected profiler split:

| Scenario | rows | direct flush ns | row build ns | btree insert ns | bulk grouping | bulk leaf build | bulk leaf write | bulk interior build | bulk interior write |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `fs_insert_single_txn_tiny_1col_100` | 100 | 4899 | 3318 | 3287 | 1/591 | 1/1683 | 1/1453 | 0/0 | 0/0 |
| `fs_insert_single_txn_small_3col_100` | 100 | 4819 | 11633 | 3413 | 1/501 | 1/1714 | 1/1312 | 0/0 | 0/0 |
| `fs_insert_single_txn_medium_6col_100` | 100 | 10780 | 19927 | 3622 | 3/1613 | 5/2424 | 5/3007 | 1/271 | 1/290 |
| `fs_insert_single_txn_large_10col_100` | 100 | 26620 | 40809 | 6150 | 3/1755 | 20/5581 | 20/8585 | 1/491 | 1/411 |
| `fs_insert_single_txn_large_10col_10000` | 10000 | 3187126 | 3982069 | 804755 | 5/140192 | 2000/1086578 | 2000/703908 | 5/20048 | 5/2807 |
| `fs_insert_record_size_large_10col_10000` | 10000 | 2902754 | 3849329 | 718597 | 5/137216 | 2000/911046 | 2000/655386 | 5/17083 | 5/1944 |

Interpretation:

- The new split rules out interior/root publication as the main large-row cost: it is tiny relative to row build and direct flush.
- Leaf publication is real on large rows: `fs_insert_record_size_large_10col_10000` spends 911046 ns building leaf pages and 655386 ns writing them.
- The remaining source frontier is still broader than page writes alone: row-build and direct-flush costs dominate the same large-row cases, so the next implementation candidate should fuse record-body construction with page layout rather than only shaving the pager write handoff.

Files:

- `insert-profile.json` - benchmark report
- `stderr.txt` - raw benchmark progress and `insert_profile` lines
- `stdout.txt` - benchmark stdout
- `insert-profile-fields.tsv` - parsed profile fields from stderr
- `ratios-over-1.tsv` - rows where FSQLite is slower than C SQLite
- `checksums.sha256` - SHA-256 manifest for generated files
