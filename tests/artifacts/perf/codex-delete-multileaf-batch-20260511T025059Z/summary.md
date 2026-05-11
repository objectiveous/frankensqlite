# Direct DELETE Multi-Leaf Batch Focused Run

This run measures the final lazy monotone multi-leaf direct DELETE batching
patch against the focused `--quick --filter update` matrix.

## Result

- Scenarios: 6
- FSQLite faster / comparable / C SQLite faster: 2 / 0 / 4
- Write-single geomean: 1.5969794704559102
- Previous focused DML frontier geomean: 1.6029243318693236
- Largest DELETE FSQLite median moved from 0.320110 ms to 0.296025 ms.

| Scenario | Ratio F/C | FSQLite ms | C SQLite ms |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 1.9340763576001898 | 0.008156 | 0.004216999999999999 |
| 100 rows / delete 5 rows | 3.5374891020052313 | 0.008114999999999999 | 0.002294 |
| 1000 rows / update 100 rows | 0.8525365907340157 | 0.030988 | 0.036348 |
| 1000 rows / delete 50 rows | 2.0608777429467087 | 0.032871 | 0.015950000000000002 |
| 10000 rows / update 1000 rows | 0.7404322078840375 | 0.268072 | 0.36204800000000004 |
| 10000 rows / delete 500 rows | 1.863703041482778 | 0.296025 | 0.158837 |

The focused run is a small keep signal: the section geomean and the largest
DELETE absolute FSQLite median both improve relative to
`codex-dml-frontier-refresh-20260511T020200Z`.
