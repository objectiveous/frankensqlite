# Concurrent writer gap profile

Date: 2026-05-07
Agent: TanBear
Source head: `4e825e62 docs(perf): publish retained dml cursor handoff`

## Context

I profiled the current small concurrent-writer C-faster rows while the higher-EV
direct-DML source files were reserved by another agent. This was measurement
only; no source candidate was applied.

The benchmark binary was built before the docs-only `4e825e62` handoff commit,
so it predates Git HEAD but is source-equivalent for engine code. The benchmark
itself emits that warning in stdout.

Command shape:

```bash
/data/tmp/frankensqlite-current-write-profile-target/release-perf/comprehensive-bench \
  --quick \
  --filter concurrent \
  --json-out tests/artifacts/perf/concurrent-writer-gap-tanbear-20260507T2013Z/current-concurrent.json \
  --no-html
```

I repeated the same command twice more as `current-concurrent-2.json` and
`current-concurrent-3.json`.

## Result

| Run | Avg ratio | Geomean ratio | F faster / comparable / C faster |
| --- | ---: | ---: | --- |
| `current-concurrent.json` | `0.8544427484492759` | `0.762285617655407` | `1 / 1 / 1` |
| `current-concurrent-2.json` | `0.8571983135314326` | `0.7833988736652135` | `1 / 1 / 1` |
| `current-concurrent-3.json` | `0.9320491017864114` | `0.9000422821106795` | `1 / 1 / 1` |

The only consistent C-faster row is `2 writers x 1000 rows`:

| Run | Ratio | C SQLite ms | FrankenSQLite ms | C CV% | F CV% |
| --- | ---: | ---: | ---: | ---: | ---: |
| `current-concurrent.json` | `1.171752413396975` | `12.110316` | `14.190292` | `20.083883782603444` | `7.927809283122309` |
| `current-concurrent-2.json` | `1.1586494128788152` | `11.780447` | `13.649408` | `5.1489505780260485` | `18.084639943594137` |
| `current-concurrent-3.json` | `1.1463411911906078` | `12.185086` | `13.968266` | `27.68108740119919` | `7.102507695633703` |

The `4 writers x 1000 rows` row is effectively noise/comparable:

- `1.0215157325170623`
- `0.9965637799721149`
- `1.035708889656877`

The `8 writers x 1000 rows` row is strongly FrankenSQLite-faster, but the
FrankenSQLite CV is high in repeats:

- `0.37006009943379065`
- `0.41638174774336784`
- `0.6140972245117495`

## Decision

Do not spend a source slice on this section right now. The row carries only the
`concurrent_writers` category weight, the overall section already favors
FrankenSQLite, and the consistent C-faster gap is small compared with the
`UPDATE/DELETEThroughput` and small INSERT rows.

The more defensible next source change remains the retained direct-DML cursor
kernel once `connection.rs` / `cursor.rs` are available. If this concurrent lane
is revisited later, start with a perf/profile run focused only on the
FrankenSQLite `2 writers x 1000 rows` arm and verify whether the extra ~1.7-2.1
ms is engine work, retry/backoff time, thread start/barrier overhead, or harness
setup. Do not optimize the benchmark harness unless the same overhead exists in
the user-facing execution path.
