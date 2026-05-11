# Direct DELETE Scratch Guard Probe

Measured date: 2026-05-11T05:19:51Z
Patch base: `7331d4793c9e1a21c2e2e77c0b64a24b2267cd06`

## Change

Direct simple DELETE used to install `PreparedDirectInsertScratchResetGuard`
for every row, even when the leaf-run fast path did not touch any direct-insert
scratch buffers. The probe keeps the reset guard only inside the
count/sum-cache physical fallback that borrows those buffers.

## Focused DML

Current reference artifact:
`tests/artifacts/perf/codex-current-head-dml-profile-20260511T043335Z/`

| row | reference ratio | run 1 ratio | run 2 ratio |
| --- | ---: | ---: | ---: |
| 100 rows / delete 5 rows | 6.16x | 5.17x | 3.53x |
| 1000 rows / delete 50 rows | 2.06x | 2.07x | 2.02x |
| 10000 rows / delete 500 rows | 1.87x | 1.95x | 1.85x |

Run 1 had a noisy 10k DELETE result, but run 2 improved every DELETE row versus
the reference. UPDATE did not show a repeatable regression; the 100-row UPDATE
row varied from faster in run 1 to slower in run 2, consistent with the high-CV
small-row noise already present in this section.

## Full Quick Gate

Current reference artifact:
`tests/artifacts/perf/codex-current-head-full-quick-20260511T042453Z/full-quick.json`

| metric | reference | fullquick1 | fullquick2 |
| --- | ---: | ---: | ---: |
| weighted score | 0.380103 | 0.376093 | 0.373722 |
| average ratio | 0.510353 | 0.508077 | 0.498677 |
| geomean ratio | 0.278852 | 0.276232 | 0.272873 |
| median ratio | 0.294670 | 0.288643 | 0.293817 |
| p90 ratio | 1.098564 | 1.065389 | 1.039205 |
| p99 ratio | 3.554413 | 3.546980 | 3.448637 |
| F faster / comparable / C faster | 80 / 2 / 11 | 78 / 4 / 11 | 80 / 4 / 9 |

Decision: keep the patch. Both full quick runs are primary-score neutral or
better, and the second full quick improves the red-row count.
