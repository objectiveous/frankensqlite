# Lazy VersionStore perf artifact

Candidate tested: lazy `Connection::version_store` allocation plus `VdbeProgram::requires_version_store`.

## Baseline

Published current artifact:
`tests/artifacts/perf/current-head-after-autocommit-preserialize-purpleotter-20260507T0615Z/report-full.json`

- Primary weighted score: `0.39072807938472587`
- C SQLite faster rows: `20`
- write_single avg ratio: `1.269708588240268`
- write_bulk avg ratio: `1.0131414327060808`

## Candidate

Artifact:
`tests/artifacts/perf/lazy-versionstore-crimsongorge-20260507T0642Z/report-full.json`

- Primary weighted score: `0.3791032082351591`
- C SQLite faster rows: `17`
- write_single avg ratio: `1.2413884652122598`
- write_bulk avg ratio: `0.9871724786676688`

Focused profile command:

```bash
perf record -F 999 -g \
  -o tests/artifacts/perf/lazy-versionstore-crimsongorge-20260507T0642Z/perf-delete-100-fsqlite.data \
  -- /data/tmp/frankensqlite-crimsongorge-versionstore-perf-target/release-perf/perf-update-delete \
  100 5000 delete fsqlite standard
```

Focused profile result:

- Before: `total=1686ms populate=433ms delete=127ms per-row-delete=5084ns`
- Candidate: `total=919ms populate=179ms delete=71ms per-row-delete=2879ns`

Correctness and gates:

- `cargo test -p fsqlite-core --test conformance_oracle_ext time_travel -- --nocapture`
- `cargo test -p fsqlite-core test_version_store_publish_and_resolve_visibility -- --nocapture`
- `cargo fmt --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
