# Prepared Direct INSERT No-FK Guard Probe

Date: 2026-05-08

Candidate: cache `has_outbound_foreign_keys` in `PreparedDirectSimpleInsert`
and skip the per-row FK pragma/schema lookup when the prepared target table has
no outbound foreign keys.

Source status: rejected and reverted. No source change was kept.

## Commands

Build candidate:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fk-direct-insert-candidate-target \
  CARGO_BUILD_JOBS=16 \
  cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench --bin perf-update-delete
```

Correctness:

```text
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-fk-direct-insert-candidate-target \
  CARGO_BUILD_JOBS=16 \
  cargo test -p fsqlite-core test_prepare_insert_with_foreign_keys_uses_direct_dispatch_and_checks_fk -- --nocapture
```

Focused A/B:

```text
/data/tmp/frankensqlite-current-noprofile-target/release-perf/comprehensive-bench \
  --quick --no-html --filter insert \
  --json-out tests/artifacts/perf/fk-direct-insert-crimsongorge-20260508T030239Z/baseline-insert.json

/data/tmp/frankensqlite-fk-direct-insert-candidate-target/release-perf/comprehensive-bench \
  --quick --no-html --filter insert \
  --json-out tests/artifacts/perf/fk-direct-insert-crimsongorge-20260508T030239Z/candidate-insert.json

/data/tmp/frankensqlite-current-noprofile-target/release-perf/comprehensive-bench \
  --quick --no-html --filter update \
  --json-out tests/artifacts/perf/fk-direct-insert-crimsongorge-20260508T030239Z/baseline-update.json

/data/tmp/frankensqlite-fk-direct-insert-candidate-target/release-perf/comprehensive-bench \
  --quick --no-html --filter update \
  --json-out tests/artifacts/perf/fk-direct-insert-crimsongorge-20260508T030239Z/candidate-update.json
```

Full candidate gate:

```text
/data/tmp/frankensqlite-fk-direct-insert-candidate-target/release-perf/comprehensive-bench \
  --quick --no-html \
  --json-out tests/artifacts/perf/fk-direct-insert-crimsongorge-20260508T030239Z/candidate-full.json
```

## Results

Focused INSERT:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | `0.784207453637674` | `0.7884368705666973` |
| Average ratio | `0.812080755704152` | `0.8036204292814446` |
| Geomean ratio | `0.7891635377632253` | `0.7734077986414365` |
| P90 ratio | `1.1049357945425362` | `1.1002142474558114` |
| P99 ratio | `1.1516829824326136` | `1.2355933953670204` |

Focused UPDATE/DELETE:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | `1.1388583327143484` | `1.036956189091621` |
| Average ratio | `1.1504068535098724` | `1.0589401876712967` |
| Geomean ratio | `1.1388583327143484` | `1.036956189091621` |
| P90 ratio | `1.4027366252527436` | `1.5023165658093798` |
| P99 ratio | `1.4027366252527436` | `1.5023165658093798` |

Full quick gate, compared to the clean no-profile baseline
`tests/artifacts/perf/calmthrush-clean-noprofile-20260508T0219Z/full-quick-clean-noprofile.json`:

| Metric | Clean baseline | Candidate |
| --- | ---: | ---: |
| Weighted score | `0.34593878641661835` | `0.34861836969535076` |
| Average ratio | `0.4542606463918878` | `0.4601152352147432` |
| Geomean ratio | `0.2674752493298549` | `0.2697448380388971` |
| P90 ratio | `0.9811588214938469` | `1.0592658202932783` |
| P99 ratio | `1.4015153360781543` | `1.39054631949189` |
| C SQLite faster rows | `8` | `10` |

## Decision

Reject. The UPDATE/DELETE focused geomean improvement was not enough to offset
the INSERT weighted-score and p99 regressions, and the full quick matrix moved
the wrong way on the primary score, average, geomean, p90, and C-faster count.
