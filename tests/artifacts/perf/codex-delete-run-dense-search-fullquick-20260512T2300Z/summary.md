# Retained DELETE dense-rowid search candidate

Date: 2026-05-12

Head: `6789e50b3f7bb53cbf7887d57641c2b3bd11310d`

Candidate: add first/last rowid checks plus a dense-rowid exact-slot probe to
`TableLeafDeleteRun::search_table_leaf`.

Verdict: rejected and removed from source before commit.

Focused DML screen:

- Baseline:
  `tests/artifacts/perf/codex-6789e50b-current-dml-screen-20260512T2245Z/`
- Candidate:
  `tests/artifacts/perf/codex-delete-run-dense-search-20260512T2255Z/`
- `fs_delete_10000` search bucket improved from `560/39571ns` to
  `560/17444ns`.
- `10000 rows / delete 500 rows` FSQLite median improved from `0.282699ms` to
  `0.236423ms`.

Full quick keep gate:

- Artifact: `full-quick.json`
- Total scenarios: `93`
- Faster / comparable / C-faster: `78 / 5 / 10`
- Average F/C ratio: `0.5019444810`
- Geomean F/C ratio: `0.2770222826`
- Primary `per_category_weighted.score`: `0.3732603712`

Current recorded frontier before this candidate:

- Faster / comparable / C-faster: `81 / 3 / 9`
- Average F/C ratio: `0.4886618611`
- Geomean F/C ratio: `0.2714660338`
- Primary `per_category_weighted.score`: `0.3676859704`

The focused DELETE micro-counter moved, but the full matrix regressed the
primary score and added one C-faster row. Do not retry this as a standalone
search micro-patch; the remaining DELETE gap needs the broader transaction-local
DML mutation operator or a same-window fullquick-neutral proof.
