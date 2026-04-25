# Reload Rowid/Payload COW Perf Pass

Run ID: `20260425T1847Z-azurepine-reload-rowid-payload-cow`

Base commit: `e5dfc9d8842ba642d51867df034304e61f3f8b39`

Workload: `comprehensive-bench --quick --filter mixed --no-html`

Build profile:

```bash
env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-prof-e5df \
  CARGO_PROFILE_RELEASE_PERF_DEBUG=true \
  CARGO_PROFILE_RELEASE_PERF_STRIP=false \
  RUSTFLAGS='-C force-frame-pointers=yes' \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

## Result

| Run | C median | Franken median | Ratio F/C | C CV | F CV |
|---|---:|---:|---:|---:|---:|
| Current `e5dfc9d8` | 254.93 ms | 300.24 ms | 1.178x | 2.54% | 2.55% |
| After fused rowid/payload, profiled | 242.66 ms | 215.45 ms | 0.888x | 0.50% | 2.81% |
| After fused rowid/payload, repeat | 236.05 ms | 204.59 ms | 0.867x | 1.20% | 2.99% |

The repeated post-change run reduced FrankenSQLite median mixed OLTP time from
`300.24 ms` to `204.59 ms` on the same host and benchmark binary shape.

## Hotspot Ledger

| Hypothesis | Verdict | Evidence |
|---|---|---|
| The memdb reload loop was parsing the current B-tree cell twice per row, once for rowid and once for payload. | Supports | `perf_current_e5dfc9d8_top.txt` shows `TransactionPageIo<dyn TransactionHandle>::parse_cell_at` at `7.71%`, with reload frames at `3.07%`, `1.46%`, and `0.65%`. |
| Fusing rowid and payload retrieval through `rowid_and_payload_cow` preserves behavior while removing duplicate cell parsing. | Supports | `cargo check -p fsqlite-core --all-targets`, `cargo clippy -p fsqlite-core --all-targets -- -D warnings`, and the repeated mixed workload completed successfully. |
| Raw B-tree cell-pointer reads are the current mixed workload target. | Rejects | The earlier full flat profile showed `read_cell_pointer_inline` around `0.01%`; the reload parse path dominated this slice instead. |

## Isomorphism Proof

- Ordering preserved: yes. The cursor still visits rows in the same order and still calls `cursor.next(cx)?` at the same point.
- Tie-breaking unchanged: N/A.
- Floating-point unchanged: N/A.
- RNG seeds unchanged: N/A.
- Record semantics preserved: yes. The same payload bytes are passed to `parse_record`; the patch only uses the existing fused accessor to obtain rowid and payload from one parsed cell.
- Concurrency defaults unchanged: yes. No transaction mode, locking, or concurrent-writer default was modified.

## Published Files

- `current_e5dfc9d8_perf.json`
- `current_e5dfc9d8_perf.stdout`
- `current_e5dfc9d8_perf.stderr`
- `after_reload_rowid_payload_cow_perf.json`
- `after_reload_rowid_payload_cow_perf.stdout`
- `after_reload_rowid_payload_cow_perf.stderr`
- `after_reload_rowid_payload_cow_repeat_perf.json`
- `after_reload_rowid_payload_cow_repeat_perf.stdout`
- `after_reload_rowid_payload_cow_repeat_perf.stderr`
- `perf_current_e5dfc9d8_top.txt`
- `perf_after_reload_rowid_payload_cow_top.txt`
- `fingerprint.txt`
