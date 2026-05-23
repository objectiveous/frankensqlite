use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::de::DeserializeOwned;

use fsqlite_harness::e2e_traceability::TraceabilityMatrix;
use fsqlite_harness::parity_evidence_matrix::{
    BEAD_ID, ParityEvidenceReport, build_parity_evidence_report,
    generate_workspace_parity_evidence_report, load_parity_closure_bead_ids,
    render_violation_diagnostics,
};
use fsqlite_harness::unit_matrix::{UnitMatrix, build_canonical_matrix};
use fsqlite_harness::verification_contract_enforcement::{
    classify_parity_evidence_report, enforce_gate_decision, render_contract_enforcement_logs,
};

#[derive(Debug)]
struct CliConfig {
    workspace_root: PathBuf,
    output_path: Option<PathBuf>,
    unit_matrix_override_path: Option<PathBuf>,
    traceability_override_path: Option<PathBuf>,
}

fn print_help() {
    let help = "\
parity_evidence_matrix_gate — parity evidence contract validator (bd-1dp9.7.5)

USAGE:
    cargo run -p fsqlite-harness --bin parity_evidence_matrix_gate -- [OPTIONS]

OPTIONS:
    --workspace-root <PATH>   Workspace root containing .beads/issues.jsonl (default: current dir)
    --unit-matrix-override <PATH>
                              Optional JSON override for UnitMatrix (relative to workspace root when not absolute)
    --traceability-override <PATH>
                              Optional JSON override for TraceabilityMatrix (relative to workspace root when not absolute)
    --output <PATH>           Write JSON report to path (stdout when omitted)
    -h, --help                Show this help
";
    println!("{help}");
}

fn parse_args(args: &[String]) -> Result<CliConfig, String> {
    let mut workspace_root = PathBuf::from(".");
    let mut output_path: Option<PathBuf> = None;
    let mut unit_matrix_override_path: Option<PathBuf> = None;
    let mut traceability_override_path: Option<PathBuf> = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace-root" => {
                index += 1;
                if index >= args.len() {
                    return Err("--workspace-root requires a value".to_owned());
                }
                workspace_root = PathBuf::from(&args[index]);
            }
            "--output" => {
                index += 1;
                if index >= args.len() {
                    return Err("--output requires a value".to_owned());
                }
                output_path = Some(PathBuf::from(&args[index]));
            }
            "--unit-matrix-override" => {
                index += 1;
                if index >= args.len() {
                    return Err("--unit-matrix-override requires a value".to_owned());
                }
                unit_matrix_override_path = Some(PathBuf::from(&args[index]));
            }
            "--traceability-override" => {
                index += 1;
                if index >= args.len() {
                    return Err("--traceability-override requires a value".to_owned());
                }
                traceability_override_path = Some(PathBuf::from(&args[index]));
            }
            "-h" | "--help" => {
                print_help();
                return Err(String::new());
            }
            unknown => {
                return Err(format!("unknown option: {unknown}"));
            }
        }
        index += 1;
    }

    Ok(CliConfig {
        workspace_root,
        output_path,
        unit_matrix_override_path,
        traceability_override_path,
    })
}

fn load_json_override<T>(
    workspace_root: &Path,
    override_path: &Path,
    label: &str,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let resolved_path = if override_path.is_absolute() {
        override_path.to_path_buf()
    } else {
        workspace_root.join(override_path)
    };

    let payload = std::fs::read_to_string(&resolved_path).map_err(|error| {
        format!(
            "{label}_read_failed path={} error={error}",
            resolved_path.display()
        )
    })?;

    serde_json::from_str(&payload).map_err(|error| {
        format!(
            "{label}_parse_failed path={} error={error}",
            resolved_path.display()
        )
    })
}

fn build_report(config: &CliConfig) -> Result<ParityEvidenceReport, String> {
    if config.unit_matrix_override_path.is_none() && config.traceability_override_path.is_none() {
        return generate_workspace_parity_evidence_report(&config.workspace_root);
    }

    let issues_path = config.workspace_root.join(".beads/issues.jsonl");
    let required_bead_ids = load_parity_closure_bead_ids(&issues_path)?;
    let unit_matrix = match &config.unit_matrix_override_path {
        Some(override_path) => load_json_override::<UnitMatrix>(
            &config.workspace_root,
            override_path,
            "unit_matrix_override",
        )?,
        None => build_canonical_matrix(),
    };
    let traceability = match &config.traceability_override_path {
        Some(override_path) => load_json_override::<TraceabilityMatrix>(
            &config.workspace_root,
            override_path,
            "traceability_override",
        )?,
        None => fsqlite_harness::e2e_traceability::build_canonical_inventory(),
    };

    Ok(build_parity_evidence_report(
        &config.workspace_root,
        &required_bead_ids,
        &unit_matrix,
        &traceability,
    ))
}

fn run(args: &[String]) -> Result<i32, String> {
    let config = parse_args(args)?;
    let report = build_report(&config)?;

    let payload = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("report_serialize_failed: {error}"))?;

    if let Some(output_path) = &config.output_path {
        std::fs::write(output_path, payload).map_err(|error| {
            format!(
                "report_write_failed path={} error={error}",
                output_path.display()
            )
        })?;
    } else {
        println!("{payload}");
    }

    let contract_report = classify_parity_evidence_report(&report);
    let enforcement = enforce_gate_decision(true, &contract_report);
    for line in render_contract_enforcement_logs(&enforcement) {
        eprintln!("INFO {line}");
    }

    if enforcement.final_gate_passed {
        return Ok(0);
    }

    let diagnostics = render_violation_diagnostics(&report);
    if let Some(first_diagnostic) = diagnostics.first() {
        eprintln!("ERROR bead_id={BEAD_ID} event=parity_evidence_first_failure {first_diagnostic}");
    }
    for line in diagnostics {
        eprintln!("WARN bead_id={BEAD_ID} {line}");
    }
    Ok(1)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(1) => ExitCode::from(1),
        Ok(_) => ExitCode::from(2),
        Err(error) if error.is_empty() => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ERROR bead_id={BEAD_ID} parity_evidence_matrix_gate failed: {error}");
            ExitCode::from(2)
        }
    }
}
