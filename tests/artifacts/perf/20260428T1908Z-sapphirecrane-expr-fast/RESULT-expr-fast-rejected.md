# Prepared Direct Insert Expression Fast Path — Rejected

Date: 2026-04-28

## Hypothesis

The current `perf-update-delete 10000 100 both` flat profile shows time in
`Connection::eval_prepared_direct_simple_insert_expr`,
`Connection::push_prepared_direct_simple_insert_value`, and
`core::str::from_utf8`. The benchmark insert shape is:

```sql
INSERT INTO bench VALUES (?1, ('user_' || ?1), (?1 * 0.137))
```

The tested patch compiled two narrower expression forms:

- text literals inside concat chains as direct `String` segments, avoiding
  repeated `SmallText::as_str()` UTF-8 validation for `'user_'`;
- `?N op literal` binary expressions as a borrowed-param expression, avoiding
  per-row clones of both operands before calling `eval_binary_op`.

## Result

Baseline current-head artifact:
`tests/artifacts/perf/20260428T1847Z-sapphirecrane-current-head/hyperfine-current.json`

Candidate artifact:
`tests/artifacts/perf/20260428T1908Z-sapphirecrane-expr-fast/hyperfine-expr-fast.json`

Same command for both:

```bash
perf-update-delete 10000 100 both
```

| Build | Mean | Median | Min | Max |
|---|---:|---:|---:|---:|
| Current HEAD | 1.261932s | 1.271083s | 1.215167s | 1.327614s |
| Expr fast-path patch | 1.306705s | 1.266319s | 1.219543s | 1.582119s |

Mean regressed by 3.55%. Median was flat/noise-level (-0.38%).

## Decision

Rejected and rolled back. The patch added representation complexity without a
stable throughput win. The candidate diff is preserved in `expr-fast.diff` as a
negative-result artifact so the same idea is not retried without stronger
evidence.

