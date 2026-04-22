use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fsqlite_harness::fts5_baseline::{
    BEAD_ID, DEFAULT_SEED, Fts5BaselineConfig, canonical_scenarios, run_all_baselines,
};
use serde_json::Value;

fn unique_artifact_root(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_nanos();
    std::env::temp_dir().join(format!(
        "frankensqlite_fts5_baseline_{label}_{}_{}",
        std::process::id(),
        stamp
    ))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("path={} error={error}", path.display()))?;
    serde_json::from_str(&raw).map_err(|error| format!("path={} error={error}", path.display()))
}

#[test]
fn test_bd_2nzo8_1_3_canonical_scenarios_cover_s1_contract() {
    let scenarios = canonical_scenarios();

    for required in [
        "FTS5-S1-STORED-CREATE-MATCH-001",
        "FTS5-S1-REOPEN-SHADOW-LAYOUT-002",
        "FTS5-S1-CONTENTLESS-COMMAND-003",
        "FTS5-S1-EXTERNAL-CONTENT-004",
        "FTS5-S1-DML-MAINTENANCE-005",
    ] {
        assert!(
            scenarios.iter().any(|scenario| scenario == required),
            "bead_id={BEAD_ID} missing required scenario {required}"
        );
    }
}

#[test]
fn test_bd_2nzo8_1_3_writes_replayable_artifact_bundle() -> Result<(), String> {
    let artifact_root = unique_artifact_root("bundle");
    let config = Fts5BaselineConfig {
        run_id: "bd-2nzo8-1-3-test-run".to_owned(),
        trace_id: "bd-2nzo8-1-3-test-trace".to_owned(),
        seed: DEFAULT_SEED,
        artifact_root: artifact_root.clone(),
        perf_iterations: 1,
        scenario_id: Some("FTS5-S1-STORED-CREATE-MATCH-001".to_owned()),
    };

    let report = run_all_baselines(&config)?;
    assert_eq!(report.bead_id, BEAD_ID);
    assert_eq!(report.scenarios.len(), 1);
    assert!(
        !report.benchmark_rows.is_empty(),
        "bead_id={BEAD_ID} expected benchmark rows"
    );

    let bundle_dir = artifact_root.join(BEAD_ID).join(&config.run_id);
    for name in [
        "events.jsonl",
        "manifest.json",
        "summary.json",
        "artifact_hashes.txt",
        "replay.env",
        "diff_report.json",
        "benchmark_summary.json",
        "memory_io_summary.json",
    ] {
        let path = bundle_dir.join(name);
        assert!(
            path.is_file(),
            "bead_id={BEAD_ID} expected artifact {}",
            path.display()
        );
    }

    let manifest = read_json(&bundle_dir.join("manifest.json"))?;
    assert_eq!(manifest["bead_id"], BEAD_ID);
    assert_eq!(manifest["run_id"], config.run_id);
    assert_eq!(manifest["seed"], DEFAULT_SEED);
    assert_eq!(manifest["oracle_backend"], "stock-sqlite-fts5");
    assert_eq!(
        manifest["rootpage_mode"],
        "stock_zero_vs_current_materialized"
    );

    let events = fs::read_to_string(bundle_dir.join("events.jsonl"))
        .map_err(|error| format!("events read failed: {error}"))?;
    for marker in [
        "\"event_type\":\"start\"",
        "\"event_type\":\"artifact_generated\"",
        "\"event_type\":\"pass\"",
        "\"phase\":\"setup\"",
        "\"phase\":\"execute\"",
        "\"phase\":\"validate\"",
        "\"phase\":\"report\"",
    ] {
        assert!(
            events.contains(marker),
            "bead_id={BEAD_ID} events missing marker {marker}"
        );
    }

    Ok(())
}

#[test]
fn test_bd_2nzo8_1_3_records_current_storage_model_divergence() -> Result<(), String> {
    let artifact_root = unique_artifact_root("divergence");
    let config = Fts5BaselineConfig {
        run_id: "bd-2nzo8-1-3-divergence-run".to_owned(),
        trace_id: "bd-2nzo8-1-3-divergence-trace".to_owned(),
        seed: DEFAULT_SEED,
        artifact_root,
        perf_iterations: 1,
        scenario_id: Some("FTS5-S1-STORED-CREATE-MATCH-001".to_owned()),
    };

    let report = run_all_baselines(&config)?;
    assert!(
        report.first_divergence.is_some(),
        "bead_id={BEAD_ID} expected current materialized path to diverge from stock shadow-table catalog"
    );
    assert!(
        report.divergent_scenario_count() >= 1,
        "bead_id={BEAD_ID} expected at least one divergent scenario"
    );
    Ok(())
}

#[test]
fn test_bd_2nzo8_1_3_env_replay_entrypoint() -> Result<(), String> {
    if std::env::var("FSQLITE_FTS5_BASELINE_E2E").as_deref() != Ok("1") {
        return Ok(());
    }

    let config = Fts5BaselineConfig::from_env();
    let report = run_all_baselines(&config)?;
    println!(
        "bead_id={BEAD_ID} run_id={} trace_id={} artifact_path={} divergent_scenario_count={}",
        report.run_id,
        report.trace_id,
        report.artifacts.bundle_dir,
        report.divergent_scenario_count()
    );
    Ok(())
}
