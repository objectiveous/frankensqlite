# Record Serializer Stack-Plan Pass

Date: 2026-04-28

Scope:
- Code change: `crates/fsqlite-types/src/record.rs`
- Benchmark: `perf-update-delete 10000 100 both`
- Build: `release-perf`, `RUSTFLAGS='-C force-frame-pointers=yes'`
- Baseline HEAD recorded in `head.txt`: `c089d1f77d336b2af592e9aa0f8c37df07b6ff71` (pre-record-stack pass)
- Note: the early baseline and candidate runs included the same pre-existing dirty `crates/fsqlite-core/src/connection.rs` edit held by `IcyBluff`; the final repeat was run after that peer work landed. The measured improvement is still concentrated in the `record.rs` stack-plan change.

Change:
- Replaced the hot small-column precomputed-header `Vec` serializer path with a fixed 16-slot stack plan.
- The stack plan validates values, computes the exact body size, resizes the output buffer once, patches runtime serial-type bytes, and writes through the existing slice encoder.
- Wide records fall back to the original append-based path.
- Marked the tiny serial-type classifier and slice encoder `#[inline]` for this hot path.

Wall-clock result (`hyperfine --warmup 1 --runs 12`):

| Run | Mean | Median | Stddev | Min | Max |
|---|---:|---:|---:|---:|---:|
| Baseline | 1.519s | 1.441s | 0.292s | 1.259s | 2.298s |
| Stack plan + clippy-clean inline | 1.247s | 1.244s | 0.022s | 1.217s | 1.290s |

Perf evidence:
- Baseline report: `perf-report-baseline-flat.txt`
- Final report: `perf-report-final-flat.txt`
- Final perf run: `total=1248ms populate=741ms update=296ms delete=144ms`

Interpretation:
- Median improved by 13.7%.
- Mean improved by 17.9%, with substantially lower run-to-run variance.
- The remaining record serializer symbol is still visible, but the end-to-end benchmark improved enough to keep this pass.
