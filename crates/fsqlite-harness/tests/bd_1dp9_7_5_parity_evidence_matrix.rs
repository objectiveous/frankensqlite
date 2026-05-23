use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fsqlite_harness::e2e_traceability::{
    ConcurrencyMode, InvocationContract, ScriptEntry, ScriptKind, StorageMode, TraceabilityMatrix,
};
use fsqlite_harness::parity_evidence_matrix::{EvidenceViolationKind, ParityEvidenceReport};
use fsqlite_harness::parity_taxonomy::FeatureCategory;
use fsqlite_harness::unit_matrix::{BucketCoverage, FailureDiagnostics, UnitMatrix, UnitTestEntry};

fn write_minimal_issues_jsonl(workspace_root: &Path) -> PathBuf {
    let beads_dir = workspace_root.join(".beads");
    fs::create_dir_all(&beads_dir).expect("create .beads directory");

    let issues_path = beads_dir.join("issues.jsonl");
    let payload = r#"{"id":"bd-1dp9.7.5","status":"closed","issue_type":"task"}"#;
    fs::write(&issues_path, payload).expect("write issues.jsonl");
    issues_path
}

fn write_track_g_issues_jsonl(workspace_root: &Path, bead_id: &str) -> PathBuf {
    let beads_dir = workspace_root.join(".beads");
    fs::create_dir_all(&beads_dir).expect("create .beads directory");

    let issues_path = beads_dir.join("issues.jsonl");
    let payload = format!(
        r#"{{"id":"{bead_id}","status":"in_progress","issue_type":"task","labels":["parity-cert","track-g"]}}"#
    );
    fs::write(&issues_path, payload).expect("write issues.jsonl");
    issues_path
}

fn write_unit_matrix_override(workspace_root: &Path, bead_id: &str) -> PathBuf {
    let unit_matrix = UnitMatrix {
        schema_version: "1.0.0".to_owned(),
        bead_id: "bd-2yqp6.7.7".to_owned(),
        root_seed: 20_260_523,
        tests: vec![UnitTestEntry {
            test_id: "UT-QUALITY-CONTRACT-001".to_owned(),
            category: FeatureCategory::ApiCli,
            crate_name: "fsqlite-harness".to_owned(),
            module_path: "bd_1dp9_7_5_parity_evidence_matrix".to_owned(),
            description: "synthetic quality-contract unit/property proof".to_owned(),
            invariants: vec!["all_evidence_classes_present".to_owned()],
            seed: 7_077,
            property_based: true,
            failure_diagnostics: FailureDiagnostics {
                dump_targets: vec!["parity_evidence_report".to_owned()],
                log_spans: vec!["parity.evidence.contract".to_owned()],
                related_beads: vec![bead_id.to_owned()],
            },
        }],
        coverage: vec![BucketCoverage {
            category: FeatureCategory::ApiCli,
            test_count: 1,
            invariant_count: 1,
            property_test_count: 1,
            contributing_crates: vec!["fsqlite-harness".to_owned()],
            missing_coverage: Vec::new(),
            fill_pct: 1.0,
        }],
    };

    let override_path = workspace_root.join("unit_matrix_override.json");
    let payload = serde_json::to_string_pretty(&unit_matrix).expect("serialize unit matrix");
    fs::write(&override_path, payload).expect("write unit matrix override");
    override_path
}

fn write_complete_traceability_override(workspace_root: &Path, bead_id: &str) -> PathBuf {
    let scripts_dir = workspace_root.join("scripts");
    fs::create_dir_all(&scripts_dir).expect("create scripts directory");
    fs::write(
        scripts_dir.join("verify_quality_contract.sh"),
        "#!/usr/bin/env bash\nexit 0\n",
    )
    .expect("write synthetic script");

    let traceability = TraceabilityMatrix {
        schema_version: "1.0.0".to_owned(),
        bead_id: "bd-2yqp6.7.7".to_owned(),
        scripts: vec![ScriptEntry {
            path: "scripts/verify_quality_contract.sh".to_owned(),
            kind: ScriptKind::ShellUtility,
            bead_id: Some(bead_id.to_owned()),
            description: "synthetic quality-contract deterministic replay".to_owned(),
            invocation: InvocationContract {
                command: "bash scripts/verify_quality_contract.sh --json".to_owned(),
                env_vars: Vec::new(),
                json_output: true,
                timeout_secs: Some(30),
            },
            scenario_ids: vec!["INFRA-7077".to_owned()],
            storage_modes: vec![StorageMode::InMemory],
            concurrency_modes: vec![ConcurrencyMode::Sequential],
            artifact_paths: vec!["artifacts/bd-2yqp6.7.7/manifest.json".to_owned()],
            log_schema_version: Some("1.0.0".to_owned()),
        }],
        gaps: Vec::new(),
    };

    let override_path = workspace_root.join("traceability_override.json");
    let payload = serde_json::to_string_pretty(&traceability).expect("serialize traceability");
    fs::write(&override_path, payload).expect("write traceability override");
    override_path
}

fn write_traceability_override_with_invalid_log_schema(workspace_root: &Path) -> PathBuf {
    let scripts_dir = workspace_root.join("scripts");
    fs::create_dir_all(&scripts_dir).expect("create scripts directory");
    fs::write(
        scripts_dir.join("verify_invalid_reference.sh"),
        "#!/usr/bin/env bash\nexit 0\n",
    )
    .expect("write synthetic script");

    let traceability = TraceabilityMatrix {
        schema_version: "1.0.0".to_owned(),
        bead_id: "bd-mblr.4.5.1".to_owned(),
        scripts: vec![ScriptEntry {
            path: "scripts/verify_invalid_reference.sh".to_owned(),
            kind: ScriptKind::ShellUtility,
            bead_id: Some("bd-1dp9.7.5".to_owned()),
            description: "synthetic invalid-log-reference scenario".to_owned(),
            invocation: InvocationContract {
                command: "bash scripts/verify_invalid_reference.sh".to_owned(),
                env_vars: Vec::new(),
                json_output: true,
                timeout_secs: Some(30),
            },
            scenario_ids: vec!["INFRA-7001".to_owned()],
            storage_modes: vec![StorageMode::InMemory],
            concurrency_modes: vec![ConcurrencyMode::Sequential],
            artifact_paths: vec!["artifacts/invalid-reference.json".to_owned()],
            log_schema_version: Some("1.bad.0".to_owned()),
        }],
        gaps: Vec::new(),
    };

    let override_path = workspace_root.join("traceability_override.json");
    let payload = serde_json::to_string_pretty(&traceability).expect("serialize traceability");
    fs::write(&override_path, payload).expect("write traceability override");
    override_path
}

#[test]
fn test_gate_binary_detects_missing_evidence_for_required_beads() {
    let temp_dir = tempfile::tempdir().expect("create temporary workspace");
    let workspace_root = temp_dir.path();
    let _issues_path = write_minimal_issues_jsonl(workspace_root);

    let output = Command::new(env!("CARGO_BIN_EXE_parity_evidence_matrix_gate"))
        .arg("--workspace-root")
        .arg(workspace_root)
        .output()
        .expect("run parity_evidence_matrix_gate");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected non-zero exit code when evidence is missing"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let report: ParityEvidenceReport =
        serde_json::from_str(&stdout).expect("report should be valid json");

    assert!(!report.summary.overall_pass);
    assert!(report.summary.violation_count > 0);
    assert!(report.violations.iter().any(|violation| violation.kind
        == EvidenceViolationKind::MissingUnitEvidence
        || violation.kind == EvidenceViolationKind::MissingE2eEvidence
        || violation.kind == EvidenceViolationKind::MissingLogEvidence
        || violation.kind == EvidenceViolationKind::MissingArtifactHashEvidence));
}

#[test]
fn test_gate_binary_detects_invalid_log_reference_from_traceability_override() {
    let temp_dir = tempfile::tempdir().expect("create temporary workspace");
    let workspace_root = temp_dir.path();
    let _issues_path = write_minimal_issues_jsonl(workspace_root);
    let override_path = write_traceability_override_with_invalid_log_schema(workspace_root);

    let output = Command::new(env!("CARGO_BIN_EXE_parity_evidence_matrix_gate"))
        .arg("--workspace-root")
        .arg(workspace_root)
        .arg("--traceability-override")
        .arg(&override_path)
        .output()
        .expect("run parity_evidence_matrix_gate with traceability override");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected non-zero exit code for invalid log reference"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let report: ParityEvidenceReport =
        serde_json::from_str(&stdout).expect("report should be valid json");

    assert!(
        report
            .violations
            .iter()
            .any(|violation| violation.kind == EvidenceViolationKind::InvalidLogReference),
        "expected invalid log reference violation in report"
    );
}

#[test]
fn test_gate_binary_accepts_complete_track_g_quality_contract_fixture() {
    let temp_dir = tempfile::tempdir().expect("create temporary workspace");
    let workspace_root = temp_dir.path();
    let bead_id = "bd-2yqp6.7.7";
    let _issues_path = write_track_g_issues_jsonl(workspace_root, bead_id);
    let unit_override = write_unit_matrix_override(workspace_root, bead_id);
    let traceability_override = write_complete_traceability_override(workspace_root, bead_id);

    let output = Command::new(env!("CARGO_BIN_EXE_parity_evidence_matrix_gate"))
        .arg("--workspace-root")
        .arg(workspace_root)
        .arg("--unit-matrix-override")
        .arg(&unit_override)
        .arg("--traceability-override")
        .arg(&traceability_override)
        .output()
        .expect("run parity_evidence_matrix_gate with complete overrides");

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected complete evidence fixture to pass"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let report: ParityEvidenceReport =
        serde_json::from_str(&stdout).expect("report should be valid json");

    assert!(report.summary.overall_pass);
    assert_eq!(report.required_evidence_classes.len(), 4);
    assert_eq!(report.rows.len(), 1);
    assert_eq!(
        report.rows[0].artifact_hash_manifest_refs,
        vec!["scripts/verify_quality_contract.sh#artifacts/bd-2yqp6.7.7/manifest.json".to_owned()]
    );
}
