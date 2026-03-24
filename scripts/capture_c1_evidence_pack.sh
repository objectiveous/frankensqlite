#!/usr/bin/env bash
# Capture a reproducible c1 evidence + scorecard pack for:
#   - bd-db300.1.7.1 authoritative low-concurrency evidence refresh
#   - bd-db300.4.5.9 operator-grade c1 e2e comparison scripts and logs
#
# The pack keeps the original raw benchmark and hot-profile artifacts, then adds:
#   - structured lifecycle events
#   - machine-readable command ledger
#   - explicit provenance bundle
#   - scorecard JSON answering which c1 cells are still below target
#   - human-readable Markdown summary

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PRIMARY_BEAD_ID="${PRIMARY_BEAD_ID:-bd-db300.4.5.9}"
COVERED_BEADS="${COVERED_BEADS:-bd-db300.1.7.1,bd-db300.4.5.9}"
SCENARIO_ID="${SCENARIO_ID:-C1-E2E-COMPARISON}"
SEED="${SEED:-459}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${PRIMARY_BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
OUTPUT_DIR="${1:-${PROJECT_ROOT}/artifacts/${PRIMARY_BEAD_ID}/${RUN_ID}}"
EVENTS_JSONL="${OUTPUT_DIR}/events.jsonl"
COMMANDS_JSONL="${OUTPUT_DIR}/commands.jsonl"
MANIFEST_JSON="${OUTPUT_DIR}/c1_pack_manifest.json"
SCORECARD_JSON="${OUTPUT_DIR}/c1_scorecard.json"
SUMMARY_MD="${OUTPUT_DIR}/summary.md"
BUILD_METADATA_JSON="${OUTPUT_DIR}/build_metadata.json"
HASHES_TXT="${OUTPUT_DIR}/artifact_hashes.txt"

mkdir -p "$OUTPUT_DIR"

WORKLOADS="${WORKLOADS:-commutative_inserts_disjoint_keys,hot_page_contention,mixed_read_write}"
HOT_PROFILE_WORKLOAD="${HOT_PROFILE_WORKLOAD:-commutative_inserts_disjoint_keys}"
if [[ "${HOT_PROFILE_WORKLOAD}" == "commutative_inserts_disjoint_keys" ]]; then
  HOT_PROFILE_WORKLOAD_TAG="commutative"
else
  HOT_PROFILE_WORKLOAD_TAG="$(printf '%s' "${HOT_PROFILE_WORKLOAD}" | tr -c '[:alnum:]_' '_')"
fi
CONCURRENCY="${CONCURRENCY:-1}"
REPEAT="${REPEAT:-3}"
DB_FIXTURES="${DB_FIXTURES:-frankensqlite,frankentui,frankensearch}"
HEALTHY_MARGIN_MIN="${HEALTHY_MARGIN_MIN:-1.10}"
SKIP_RUN="${SKIP_RUN:-0}"
RENDER_ONLY="${RENDER_ONLY:-0}"

if [[ "${RENDER_ONLY}" == "1" ]]; then
  touch "$EVENTS_JSONL" "$COMMANDS_JSONL"
else
  : > "$EVENTS_JSONL"
  : > "$COMMANDS_JSONL"
fi

IFS=',' read -ra FIXTURE_ARRAY <<< "$DB_FIXTURES"
IFS=',' read -ra WORKLOAD_ARRAY <<< "$WORKLOADS"
IFS=',' read -ra COVERED_BEAD_ARRAY <<< "$COVERED_BEADS"

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local elapsed_ms="$4"
  local message="$5"
  local fixture_id="${6:-all}"
  local mode_id="${7:-all}"
  local artifact_relpath="${8:-none}"
  local command_line="${9:-none}"

  python3 - "${EVENTS_JSONL}" \
    "${TRACE_ID}" "${SCENARIO_ID}" "${PRIMARY_BEAD_ID}" "${RUN_ID}" "${phase}" \
    "${event_type}" "${outcome}" "${elapsed_ms}" "${message}" \
    "${fixture_id}" "${mode_id}" "${artifact_relpath}" "${command_line}" <<'PY'
import json
import sys
from datetime import datetime, timezone

path = sys.argv[1]
(
    trace_id,
    scenario_id,
    bead_id,
    run_id,
    phase,
    event_type,
    outcome,
    elapsed_ms,
    message,
    fixture_id,
    mode_id,
    artifact_relpath,
    command_line,
) = sys.argv[2:15]

event = {
    "artifact_manifest_key": "c1_evidence_pack",
    "bead_id": bead_id,
    "command_line": None if command_line == "none" else command_line,
    "elapsed_ms": int(elapsed_ms),
    "event_type": event_type,
    "fixture_id": fixture_id,
    "message": message,
    "mode_id": mode_id,
    "outcome": outcome,
    "phase": phase,
    "artifact_relpath": None if artifact_relpath == "none" else artifact_relpath,
    "run_id": run_id,
    "scenario_id": scenario_id,
    "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "trace_id": trace_id,
}
with open(path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(event, sort_keys=True) + "\n")
PY
}

record_command() {
  local stage="$1"
  local fixture_id="$2"
  local mode_id="$3"
  local command_line="$4"

  python3 - "${COMMANDS_JSONL}" \
    "${TRACE_ID}" "${SCENARIO_ID}" "${PRIMARY_BEAD_ID}" "${RUN_ID}" \
    "${stage}" "${fixture_id}" "${mode_id}" "${command_line}" <<'PY'
import json
import sys
from datetime import datetime, timezone

path = sys.argv[1]
(
    trace_id,
    scenario_id,
    bead_id,
    run_id,
    stage,
    fixture_id,
    mode_id,
    command_line,
) = sys.argv[2:10]

record = {
    "bead_id": bead_id,
    "command_line": command_line,
    "fixture_id": fixture_id,
    "mode_id": mode_id,
    "run_id": run_id,
    "scenario_id": scenario_id,
    "stage": stage,
    "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "trace_id": trace_id,
}
with open(path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(record, sort_keys=True) + "\n")
PY
}

write_build_metadata() {
  local beads_hash="unknown"
  local git_sha="unknown"
  local git_dirty_entries="0"
  local cpu_model="unknown"
  if [[ -f "${PROJECT_ROOT}/.beads/issues.jsonl" ]]; then
    beads_hash="$(sha256sum "${PROJECT_ROOT}/.beads/issues.jsonl" | awk '{print $1}')"
  fi
  if git -C "${PROJECT_ROOT}" rev-parse HEAD >/dev/null 2>&1; then
    git_sha="$(git -C "${PROJECT_ROOT}" rev-parse HEAD)"
    git_dirty_entries="$(git -C "${PROJECT_ROOT}" status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
  fi
  cpu_model="$(awk -F: '/model name/ {gsub(/^[ \t]+/, "", $2); print $2; exit}' /proc/cpuinfo 2>/dev/null || true)"
  if [[ -z "${cpu_model}" ]]; then
    cpu_model="unknown"
  fi

  python3 - "${BUILD_METADATA_JSON}" \
    "${PRIMARY_BEAD_ID}" \
    "${COVERED_BEADS}" \
    "${RUN_ID}" \
    "${TRACE_ID}" \
    "${SCENARIO_ID}" \
    "${HEALTHY_MARGIN_MIN}" \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    "$(hostname)" \
    "$(uname -r)" \
    "$(rustc --version)" \
    "$(cargo --version)" \
    "${CARGO_TARGET_DIR:-default}" \
    "${git_sha}" \
    "${git_dirty_entries}" \
    "${beads_hash}" \
    "${cpu_model}" \
    "$(nproc 2>/dev/null || echo unknown)" <<'PY'
import json
import sys

path = sys.argv[1]
document = {
    "generated_at_utc": sys.argv[8],
    "primary_bead_id": sys.argv[2],
    "covered_beads": [item for item in sys.argv[3].split(",") if item],
    "run_id": sys.argv[4],
    "trace_id": sys.argv[5],
    "scenario_id": sys.argv[6],
    "cargo_profile": "release-perf",
    "healthy_margin_min": float(sys.argv[7]),
    "hostname": sys.argv[9],
    "kernel_release": sys.argv[10],
    "rustc_version": sys.argv[11],
    "cargo_version": sys.argv[12],
    "cargo_target_dir": sys.argv[13],
    "git_sha": sys.argv[14],
    "git_dirty_entries": int(sys.argv[15]),
    "beads_data_hash": sys.argv[16],
    "cpu_model": sys.argv[17],
    "cpu_cores": int(sys.argv[18]),
}
with open(path, "w", encoding="utf-8") as handle:
    json.dump(document, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
}

ensure_binary() {
  BINARY="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}/release-perf/realdb-e2e"
  if [[ -f "${BINARY}" ]]; then
    return
  fi

  local started finished elapsed build_cmd
  build_cmd="cargo build --profile release-perf -p fsqlite-e2e --bin realdb-e2e"
  record_command "build" "all" "build" "${build_cmd}"
  emit_event "build" "start" "running" 0 "building release-perf realdb-e2e" "all" "build" "none" "${build_cmd}"
  started="$(date +%s%3N)"
  if (
    cd "${PROJECT_ROOT}"
    cargo build --profile release-perf -p fsqlite-e2e --bin realdb-e2e
  ) 2>&1 | tee "${OUTPUT_DIR}/build_stdout.log"; then
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event "build" "pass" "pass" "${elapsed}" "built release-perf realdb-e2e" "all" "build" "build_stdout.log" "${build_cmd}"
  else
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event "build" "fail" "fail" "${elapsed}" "failed to build release-perf realdb-e2e" "all" "build" "build_stdout.log" "${build_cmd}"
    return 1
  fi
}

run_bench_mode() {
  local fixture_id="$1"
  local mode_id="$2"
  local mode_label="$3"
  shift 3
  local -a mode_args=("$@")
  local result_relpath="c1_${fixture_id}_${mode_id}.jsonl"
  local log_relpath="c1_${fixture_id}_${mode_id}_stdout.log"
  local started finished elapsed
  local -a cmd=(
    "${BINARY}" bench
    --db "${fixture_id}"
    --preset "${WORKLOADS}"
    --concurrency "${CONCURRENCY}"
    --repeat "${REPEAT}"
    --output-jsonl "${OUTPUT_DIR}/${result_relpath}"
    --pretty
    "${mode_args[@]}"
  )
  local command_line="${cmd[*]}"

  record_command "bench" "${fixture_id}" "${mode_id}" "${command_line}"
  emit_event "bench" "start" "running" 0 "running ${mode_label}" "${fixture_id}" "${mode_id}" "${result_relpath}" "${command_line}"
  started="$(date +%s%3N)"
  if "${cmd[@]}" 2>&1 | tee "${OUTPUT_DIR}/${log_relpath}"; then
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event "bench" "pass" "pass" "${elapsed}" "completed ${mode_label}" "${fixture_id}" "${mode_id}" "${result_relpath}" "${command_line}"
  else
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event "bench" "fail" "fail" "${elapsed}" "failed ${mode_label}" "${fixture_id}" "${mode_id}" "${result_relpath}" "${command_line}"
    return 1
  fi
}

run_hot_profile() {
  local fixture_id="$1"
  local hot_dir_relpath="hotprofile_${fixture_id}_${HOT_PROFILE_WORKLOAD_TAG}_c1"
  local hot_dir="${OUTPUT_DIR}/${hot_dir_relpath}"
  local log_relpath="c1_${fixture_id}_hotprofile_${HOT_PROFILE_WORKLOAD_TAG}.log"
  local started finished elapsed
  local -a cmd=(
    "${BINARY}" hot-profile
    --db "${fixture_id}"
    --preset "${HOT_PROFILE_WORKLOAD}"
    --concurrency "${CONCURRENCY}"
    --mvcc
    --output-dir "${hot_dir}"
    --pretty
  )
  local command_line="${cmd[*]}"

  mkdir -p "${hot_dir}"
  record_command "hot_profile" "${fixture_id}" "fsqlite_mvcc" "${command_line}"
  emit_event "hot_profile" "start" "running" 0 "running c1 hot profile" "${fixture_id}" "fsqlite_mvcc" "${hot_dir_relpath}" "${command_line}"
  started="$(date +%s%3N)"
  if "${cmd[@]}" 2>&1 | tee "${OUTPUT_DIR}/${log_relpath}"; then
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event "hot_profile" "pass" "pass" "${elapsed}" "completed c1 hot profile" "${fixture_id}" "fsqlite_mvcc" "${hot_dir_relpath}" "${command_line}"
  else
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event "hot_profile" "fail" "fail" "${elapsed}" "failed c1 hot profile" "${fixture_id}" "fsqlite_mvcc" "${hot_dir_relpath}" "${command_line}"
    return 1
  fi
}

render_reports() {
  emit_event "render" "start" "running" 0 "rendering c1 scorecard and manifest"
  python3 - \
    "${OUTPUT_DIR}" \
    "${MANIFEST_JSON}" \
    "${SCORECARD_JSON}" \
    "${SUMMARY_MD}" \
    "${BUILD_METADATA_JSON}" \
    "${COMMANDS_JSONL}" \
    "${TRACE_ID}" \
    "${SCENARIO_ID}" \
    "${RUN_ID}" \
    "${PRIMARY_BEAD_ID}" \
    "${HEALTHY_MARGIN_MIN}" \
    "${WORKLOADS}" \
    "${DB_FIXTURES}" \
    "${CONCURRENCY}" \
    "${REPEAT}" \
    "${HOT_PROFILE_WORKLOAD}" \
    "${HOT_PROFILE_WORKLOAD_TAG}" <<'PY'
import json
import math
import os
import sys
from pathlib import Path

output_dir = Path(sys.argv[1])
manifest_path = Path(sys.argv[2])
scorecard_path = Path(sys.argv[3])
summary_path = Path(sys.argv[4])
build_metadata_path = Path(sys.argv[5])
commands_path = Path(sys.argv[6])
trace_id = sys.argv[7]
scenario_id = sys.argv[8]
run_id = sys.argv[9]
bead_id = sys.argv[10]
healthy_margin_min = float(sys.argv[11])
workloads = [w for w in sys.argv[12].split(",") if w]
fixtures = [f for f in sys.argv[13].split(",") if f]
concurrency = int(sys.argv[14])
repeat = int(sys.argv[15])
hot_profile_workload = sys.argv[16]
hot_profile_workload_tag = sys.argv[17]

mode_specs = [
    ("sqlite3", "C SQLite"),
    ("fsqlite_mvcc", "FrankenSQLite MVCC"),
    ("fsqlite_single", "FrankenSQLite Single Writer"),
]

with build_metadata_path.open("r", encoding="utf-8") as handle:
    build_metadata = json.load(handle)

trace_id = build_metadata.get("trace_id", trace_id)
scenario_id = build_metadata.get("scenario_id", scenario_id)
run_id = build_metadata.get("run_id", run_id)
bead_id = build_metadata.get("primary_bead_id", bead_id)
healthy_margin_min = float(build_metadata.get("healthy_margin_min", healthy_margin_min))

commands = []
with commands_path.open("r", encoding="utf-8") as handle:
    for line in handle:
        line = line.strip()
        if line:
            commands.append(json.loads(line))

summaries = {}
artifacts = []
for fixture in fixtures:
    for mode_id, mode_label in mode_specs:
        relpath = f"c1_{fixture}_{mode_id}.jsonl"
        log_relpath = f"c1_{fixture}_{mode_id}_stdout.log"
        path = output_dir / relpath
        if path.exists():
            entries = []
            with path.open("r", encoding="utf-8") as handle:
                for line in handle:
                    line = line.strip()
                    if not line:
                        continue
                    record = json.loads(line)
                    if "benchmark_id" not in record or "throughput" not in record:
                        continue
                    entries.append(record)
            summaries[(fixture, mode_id)] = entries
            artifacts.append(
                {
                    "fixture_id": fixture,
                    "mode_id": mode_id,
                    "mode_label": mode_label,
                    "result_jsonl": relpath,
                    "stdout_log": log_relpath,
                }
            )

hot_profiles = []
for fixture in fixtures:
    candidates = [
        (
            f"hotprofile_{fixture}_{hot_profile_workload_tag}_c1",
            f"c1_{fixture}_hotprofile_{hot_profile_workload_tag}.log",
        ),
        (f"hotprofile_{fixture}_commutative_c1", f"c1_{fixture}_hotprofile_commutative.log"),
    ]
    selected = None
    for relpath, log_relpath in candidates:
        if (output_dir / relpath).exists() or (output_dir / log_relpath).exists():
            selected = (relpath, log_relpath)
            break
    if selected is not None:
        relpath, log_relpath = selected
        hot_profiles.append(
            {
                "fixture_id": fixture,
                "workload": hot_profile_workload,
                "directory": relpath,
                "stdout_log": log_relpath,
            }
        )
hot_profile_dirs = {entry["fixture_id"]: entry["directory"] for entry in hot_profiles}

rows = []
ratio_buckets = {
    "fsqlite_mvcc": {"below_parity": 0, "parity_to_margin": 0, "healthy_margin": 0},
    "fsqlite_single": {"below_parity": 0, "parity_to_margin": 0, "healthy_margin": 0},
}
ratio_values = {"fsqlite_mvcc": [], "fsqlite_single": []}

def classify_ratio(ratio: float) -> str:
    if ratio < 1.0:
        return "below_parity"
    if ratio < healthy_margin_min:
        return "parity_to_margin"
    return "healthy_margin"

for fixture in fixtures:
    sqlite_entries = {
        row["workload"]: row
        for row in summaries.get((fixture, "sqlite3"), [])
    }
    for mode_id, mode_label in mode_specs[1:]:
        for row in summaries.get((fixture, mode_id), []):
            workload = row["workload"]
            baseline = sqlite_entries.get(workload)
            median_ops = row["throughput"]["median_ops_per_sec"]
            median_latency = row["latency"]["median_ms"]
            p95_latency = row["latency"]["p95_ms"]
            retries_total = sum(it["retries"] for it in row.get("iterations", []))
            aborts_total = sum(it["aborts"] for it in row.get("iterations", []))
            if baseline is None:
                ratio = None
                classification = "missing_baseline"
                sqlite_median_ops = None
                sqlite_median_latency = None
            else:
                sqlite_median_ops = baseline["throughput"]["median_ops_per_sec"]
                sqlite_median_latency = baseline["latency"]["median_ms"]
                ratio = (median_ops / sqlite_median_ops) if sqlite_median_ops > 0 else None
                if ratio is None:
                    classification = "missing_baseline"
                else:
                    classification = classify_ratio(ratio)
                    ratio_buckets[mode_id][classification] += 1
                    ratio_values[mode_id].append(ratio)

            rows.append(
                {
                    "row_id": f"{fixture}:{workload}:{mode_id}",
                    "fixture_id": fixture,
                    "workload": workload,
                    "mode_id": mode_id,
                    "mode_label": mode_label,
                    "median_ops_per_sec": median_ops,
                    "median_latency_ms": median_latency,
                    "p95_latency_ms": p95_latency,
                    "sqlite_median_ops_per_sec": sqlite_median_ops,
                    "sqlite_median_latency_ms": sqlite_median_latency,
                    "speedup_vs_sqlite": ratio,
                    "classification": classification,
                    "retries_total": retries_total,
                    "aborts_total": aborts_total,
                    "measurement_count": row["measurement_count"],
                    "total_measurement_ms": row["total_measurement_ms"],
                    "hot_profile_dir": hot_profile_dirs.get(fixture),
                }
            )

def geometric_mean(values):
    positives = [value for value in values if value and value > 0]
    if not positives:
        return None
    return math.exp(sum(math.log(value) for value in positives) / len(positives))

below_rows = [row for row in rows if row["classification"] == "below_parity"]
margin_rows = [row for row in rows if row["classification"] == "parity_to_margin"]
healthy_rows = [row for row in rows if row["classification"] == "healthy_margin"]
missing_baseline_rows = [row for row in rows if row["classification"] == "missing_baseline"]
expected_critical_cell_count = len(fixtures) * len(workloads) * 2
comparable_rows = [row for row in rows if row["speedup_vs_sqlite"] is not None]
if not comparable_rows:
    honest_gate_verdict = "no_data"
elif len(comparable_rows) < expected_critical_cell_count or missing_baseline_rows:
    honest_gate_verdict = "incomplete"
elif below_rows:
    honest_gate_verdict = "fail"
elif margin_rows:
    honest_gate_verdict = "warning"
else:
    honest_gate_verdict = "pass"

mode_rollup = []
for mode_id, mode_label in mode_specs[1:]:
    mode_rollup.append(
        {
            "mode_id": mode_id,
            "mode_label": mode_label,
            "comparable_cell_count": len(ratio_values[mode_id]),
            "geometric_mean_speedup": geometric_mean(ratio_values[mode_id]),
            **ratio_buckets[mode_id],
        }
    )

workload_rollup = []
for mode_id, mode_label in mode_specs[1:]:
    for workload in workloads:
        values = [
            row["speedup_vs_sqlite"]
            for row in rows
            if row["mode_id"] == mode_id and row["workload"] == workload and row["speedup_vs_sqlite"]
        ]
        workload_rollup.append(
            {
                "mode_id": mode_id,
                "mode_label": mode_label,
                "workload": workload,
                "geometric_mean_speedup": geometric_mean(values),
                "comparable_cell_count": len(values),
            }
        )

scorecard = {
    "schema_version": "bd-db300.c1_evidence_pack_scorecard.v1",
    "bead_id": bead_id,
    "covered_beads": build_metadata["covered_beads"],
    "run_id": run_id,
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "pack_role": "honest_gate_scorecard",
    "baseline_comparator": "sqlite3_same_pack",
    "shadow_lineage": "none",
    "critical_scope": "all c1 fixture/workload/mode cells captured by this pack",
    "comparator_contract": {
        "baseline_comparator": "sqlite3_same_pack",
        "comparator_engine": "sqlite3",
        "comparator_scope": "same fixture, same workload, same pack",
        "aggregate_rows_are_secondary": True,
    },
    "causal_attribution_contract": {
        "required_for_claimed_fix": True,
        "required_claim_fields": [
            "code_change_ref",
            "claim_summary",
            "baseline_run_id",
            "baseline_comparator",
            "cells_expected_to_move",
            "cells_expected_not_to_move",
            "negative_findings",
        ],
    },
    "honest_gate_summary": {
        "verdict": honest_gate_verdict,
        "expected_critical_cell_count": expected_critical_cell_count,
        "critical_cell_count": len(rows),
        "comparable_cell_count": len(comparable_rows),
        "missing_baseline_count": len(missing_baseline_rows),
        "below_parity_count": len(below_rows),
        "parity_to_margin_count": len(margin_rows),
        "healthy_margin_count": len(healthy_rows),
        "hard_fail_when_below_parity_present": True,
        "critical_red_cell_ids": [row["row_id"] for row in below_rows],
        "margin_band_cell_ids": [row["row_id"] for row in margin_rows],
        "missing_baseline_row_ids": [row["row_id"] for row in missing_baseline_rows],
    },
    "healthy_margin_min": healthy_margin_min,
    "concurrency": concurrency,
    "repeat": repeat,
    "fixtures": fixtures,
    "workloads": workloads,
    "rows": rows,
    "mode_rollup": mode_rollup,
    "workload_rollup": workload_rollup,
    "below_parity_rows": below_rows,
    "parity_to_margin_rows": margin_rows,
    "healthy_margin_rows": healthy_rows,
    "missing_baseline_rows": missing_baseline_rows,
}
scorecard_path.write_text(json.dumps(scorecard, indent=2, sort_keys=True) + "\n", encoding="utf-8")

manifest = {
    "schema_version": "bd-db300.c1_evidence_pack_manifest.v1",
    "bead_id": bead_id,
    "covered_beads": build_metadata["covered_beads"],
    "run_id": run_id,
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "output_dir": output_dir.name,
    "entrypoint": "scripts/capture_c1_evidence_pack.sh",
    "pack_role": "honest_gate_evidence_pack",
    "baseline_comparator": "sqlite3_same_pack",
    "shadow_lineage": "none",
    "comparator_contract": scorecard["comparator_contract"],
    "causal_attribution_contract": scorecard["causal_attribution_contract"],
    "honest_gate_summary": scorecard["honest_gate_summary"],
    "build_metadata": build_metadata,
    "fixtures": fixtures,
    "workloads": workloads,
    "concurrency": concurrency,
    "repeat": repeat,
    "healthy_margin_min": healthy_margin_min,
    "build_metadata_json": build_metadata_path.name,
    "build_metadata_relpath": build_metadata_path.name,
    "commands_jsonl": commands_path.name,
    "commands_relpath": commands_path.name,
    "events_jsonl": "events.jsonl",
    "events_relpath": "events.jsonl",
    "scorecard_json": scorecard_path.name,
    "scorecard_relpath": scorecard_path.name,
    "summary_md": summary_path.name,
    "summary_relpath": summary_path.name,
    "hashes_relpath": "artifact_hashes.txt",
    "bench_artifacts": artifacts,
    "hot_profiles": hot_profiles,
    "command_count": len(commands),
}
manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

summary_lines = [
    f"# {bead_id} c1 Evidence Pack",
    "",
    f"- run_id: `{run_id}`",
    f"- trace_id: `{trace_id}`",
    f"- scenario_id: `{scenario_id}`",
    f"- fixtures: `{', '.join(fixtures)}`",
    f"- workloads: `{', '.join(workloads)}`",
    f"- concurrency: `{concurrency}`",
    f"- repeat: `{repeat}`",
    f"- healthy_margin_min: `{healthy_margin_min:.2f}x`",
    "",
    "## Honest Gate Summary",
    "",
    f"- verdict: `{honest_gate_verdict}`",
    "- critical_scope: every c1 fixture/workload/mode cell in this pack is a critical gate cell",
    "- baseline_comparator: same-pack `sqlite3` rows for the matching fixture and workload",
    f"- expected_critical_cell_count: `{expected_critical_cell_count}`",
    f"- comparable_cell_count: `{len(comparable_rows)}`",
    f"- missing_baseline_count: `{len(missing_baseline_rows)}`",
    f"- below_parity_count: `{len(below_rows)}`",
    f"- parity_to_margin_count: `{len(margin_rows)}`",
    f"- healthy_margin_count: `{len(healthy_rows)}`",
    "- aggregate rollups are secondary and must not be used to hide a red c1 cell",
    "",
    "## Mode Rollup",
    "",
    "| Mode | Geometric Mean Speedup | Below 1.0x | 1.0x to Margin | Healthy Margin |",
    "|------|------------------------|------------|----------------|----------------|",
]
for row in mode_rollup:
    gm = row["geometric_mean_speedup"]
    gm_str = "n/a" if gm is None else f"{gm:.3f}x"
    summary_lines.append(
        f"| {row['mode_label']} | {gm_str} | {row['below_parity']} | {row['parity_to_margin']} | {row['healthy_margin']} |"
    )

summary_lines.extend(
    [
        "",
        "## Workload Rollup",
        "",
        "| Mode | Workload | Geometric Mean Speedup | Comparable Cells |",
        "|------|----------|------------------------|------------------|",
    ]
)
for row in workload_rollup:
    gm = row["geometric_mean_speedup"]
    gm_str = "n/a" if gm is None else f"{gm:.3f}x"
    summary_lines.append(
        f"| {row['mode_label']} | {row['workload']} | {gm_str} | {row['comparable_cell_count']} |"
    )

summary_lines.extend(
    [
        "",
        "## Cell Scorecard",
        "",
        "| Fixture | Workload | Mode | Median ops/s | SQLite median ops/s | Speedup | Median latency (ms) | P95 latency (ms) | Retries | Aborts | Verdict |",
        "|---------|----------|------|--------------|---------------------|---------|---------------------|------------------|---------|--------|---------|",
    ]
)
for row in rows:
    speedup = row["speedup_vs_sqlite"]
    speedup_str = "n/a" if speedup is None else f"{speedup:.3f}x"
    sqlite_ops = row["sqlite_median_ops_per_sec"]
    sqlite_ops_str = "n/a" if sqlite_ops is None else f"{sqlite_ops:.2f}"
    summary_lines.append(
        f"| {row['fixture_id']} | {row['workload']} | {row['mode_label']} | {row['median_ops_per_sec']:.2f} | {sqlite_ops_str} | {speedup_str} | {row['median_latency_ms']:.3f} | {row['p95_latency_ms']:.3f} | {row['retries_total']} | {row['aborts_total']} | {row['classification']} |"
    )

summary_lines.extend(["", "## Cells Still Below Parity", ""])
if not comparable_rows:
    summary_lines.append("- no comparable c1 cells were captured in this pack")
elif below_rows:
    for row in below_rows:
        hot_profile_note = row["hot_profile_dir"] or "none"
        summary_lines.append(
            f"- `{row['fixture_id']}:{row['workload']}:{row['mode_id']}` at `{row['speedup_vs_sqlite']:.3f}x`; hot-profile bundle: `{hot_profile_note}`"
        )
else:
    summary_lines.append("- none")

summary_lines.extend(["", "## Comparator and Hot-Profile Bundles", ""])
if hot_profiles:
    for entry in hot_profiles:
        summary_lines.append(
            f"- `{entry['fixture_id']}` hot-profile dir: `{entry['directory']}` for workload `{entry['workload']}`"
        )
else:
    summary_lines.append("- no hot-profile bundles were captured in this pack")

summary_path.write_text("\n".join(summary_lines) + "\n", encoding="utf-8")
PY
  emit_event "render" "pass" "pass" 0 "rendered c1 scorecard and manifest"
}

hash_artifacts() {
  (
    cd "${OUTPUT_DIR}"
    find . -type f ! -name "$(basename "${HASHES_TXT}")" -print0 \
      | sort -z \
      | xargs -0 sha256sum > "$(basename "${HASHES_TXT}")"
  )
  emit_event "hash" "pass" "pass" 0 "hashed c1 evidence artifacts"
}

main() {
  local display_bead_id="${PRIMARY_BEAD_ID}"
  if [[ "${RENDER_ONLY}" == "1" && -f "${BUILD_METADATA_JSON}" ]]; then
    display_bead_id="$(python3 - "${BUILD_METADATA_JSON}" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    data = json.load(handle)
print(data.get("primary_bead_id", "unknown"))
PY
)"
  fi

  echo "=== ${display_bead_id}: c1 Evidence Pack ==="
  echo "Output: ${OUTPUT_DIR}"
  echo "Profile: release-perf"
  echo "Fixtures: ${DB_FIXTURES}"
  echo "Workloads: ${WORKLOADS}"
  echo ""

  if [[ "${RENDER_ONLY}" == "1" ]]; then
    if [[ ! -f "${BUILD_METADATA_JSON}" ]]; then
      echo "RENDER_ONLY=1 requires an existing build_metadata.json at ${BUILD_METADATA_JSON}" >&2
      return 1
    fi
    emit_event "render" "note" "pass" 0 "render-only refresh: reusing existing c1 raw artifacts"
  else
    write_build_metadata
    ensure_binary
  fi

  if [[ "${RENDER_ONLY}" == "1" ]]; then
    :
  elif [[ "${SKIP_RUN}" != "1" ]]; then
    for fixture_id in "${FIXTURE_ARRAY[@]}"; do
      echo "====== Fixture: ${fixture_id} ======"
      run_bench_mode "${fixture_id}" "sqlite3" "C SQLite control (c1)" --engine sqlite3
      echo ""
      run_bench_mode "${fixture_id}" "fsqlite_mvcc" "FrankenSQLite MVCC (c1)" --engine fsqlite --mvcc
      echo ""
      run_bench_mode "${fixture_id}" "fsqlite_single" "FrankenSQLite single-writer (c1)" --engine fsqlite --no-mvcc
      echo ""
      run_hot_profile "${fixture_id}"
      echo ""
    done
  else
    emit_event "bench" "skip" "skipped" 0 "skipping benchmark execution because SKIP_RUN=1"
  fi

  render_reports
  hash_artifacts

  printf '%s\n' \
    "=== Evidence pack complete: ${OUTPUT_DIR} ===" \
    "summary: ${SUMMARY_MD}" \
    "manifest: ${MANIFEST_JSON}" \
    "scorecard: ${SCORECARD_JSON}" \
    "events: ${EVENTS_JSONL}" \
    "commands: ${COMMANDS_JSONL}" \
    "hashes: ${HASHES_TXT}"
}

main "$@"
