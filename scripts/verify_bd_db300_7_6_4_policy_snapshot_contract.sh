#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT_DIR}/target-policy-snapshot}"
ARTIFACT_DIR="${POLICY_SNAPSHOT_ARTIFACT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/fsqlite-policy-snapshot.XXXXXX")}"

cd "${ROOT_DIR}"

export CARGO_TARGET_DIR="${TARGET_DIR}"

cargo test -p fsqlite-core test_runtime_snapshot_serializes_contract_fields -- --nocapture
cargo test -p fsqlite-harness --test bd_db300_7_6_4_policy_snapshot_contract -- --nocapture
cargo run -p fsqlite-e2e --bin realdb-e2e -- verify-suite \
  --suite-id bd-db300.7.6.4.operator_surface \
  --execution-context local \
  --mode fsqlite_mvcc \
  --verification-depth quick \
  --activation-regime hostile_or_unclassified \
  --shadow-mode forced \
  --shadow-verdict diverged \
  --kill-switch-state tripped \
  --divergence-class observability_gap \
  --first-failure-diagnostics "policy snapshot contract smoke" \
  --output-dir "${ARTIFACT_DIR}/verify_suite" \
  --pretty

printf 'policy_snapshot_artifacts=%s\n' "${ARTIFACT_DIR}"
