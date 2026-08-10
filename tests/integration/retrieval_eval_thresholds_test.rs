//! Threshold-gating integration for `run_retrieval_eval` (084.006-T / 14B33F9F).
//!
//! Before this change the eval report never consulted the configured
//! `RetrievalEvalThresholds`, so a run whose metrics breached an operator floor
//! still reported `thresholds_breached=false` — the gate was inert. These
//! scenarios drive the full `run_retrieval_eval` dispatch path and assert the
//! outcome is recorded honestly:
//!   1. metrics within thresholds (or default/permissive config) -> no breach;
//!   2. a breached floor -> `thresholds_breached=true` + a recorded reason;
//!   3. default (unconfigured) thresholds -> no gating (back-compat);
//!   4. disabled or empty run -> no FALSE breach (contract preserved).
//!
//! Isolation: the harness binds a fresh tempdir workspace. Whether or not the
//! shell exports an ambient `ENGRAM_DATA_DIR`, the branch DB lives inside that
//! tempdir, so the corpus and metrics are deterministic.

#[path = "../helpers/mod.rs"]
mod helpers;

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::test;

use engram::models::config::WorkspaceConfig;
use engram::models::retrieval_eval::{RetrievalEvalConfig, RetrievalEvalThresholds};
use engram::server::state::AppState;
use engram::tools;

/// A single-file Rust fixture with one in-file call (`alpha` -> `beta`), so the
/// indexed graph has a resolvable call site and `resolution_recall` is > 0.
const FIXTURE_SRC: &str =
    "pub fn alpha() {\n    beta();\n}\n\npub fn beta() {\n    let _ = 1;\n}\n";

/// Dispatch a tool, retrying the known transient `CozoDB` reopen lock
/// (`SQLITE_BUSY`) that can surface on rapid sequential reopens (Windows).
async fn dispatch_retry(
    state: Arc<AppState>,
    tool: &str,
    args: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut last_err = String::new();
    for _ in 0u32..8 {
        match tools::dispatch(state.clone(), tool, args.clone()).await {
            Ok(value) => return value,
            Err(err) => {
                let lowered = err.to_string().to_ascii_lowercase();
                if lowered.contains("database is locked") || lowered.contains("sqlite_busy") {
                    last_err = err.to_string();
                    tokio::time::sleep(Duration::from_millis(75)).await;
                    continue;
                }
                panic!("dispatch {tool} failed: {err}");
            }
        }
    }
    panic!("dispatch {tool} failed after retries on transient DB lock: {last_err}");
}

/// Bind a temp workspace with the given config; returns state + tempdir.
async fn setup_workspace(config: WorkspaceConfig) -> (Arc<AppState>, tempfile::TempDir) {
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let state = Arc::new(AppState::new(10));
    helpers::bind_isolated_workspace_with_disabled_metrics(
        &state,
        workspace.path(),
        "main",
        config,
    )
    .await;
    (state, workspace)
}

/// Regression: an ambient developer data directory must never redirect this
/// in-process fixture away from its disposable workspace.
#[test]
#[serial_test::serial(metrics_writer)]
async fn setup_workspace_keeps_data_dir_inside_temp_workspace() {
    let (state, workspace) = setup_workspace(WorkspaceConfig::default()).await;
    let snapshot = state
        .snapshot_workspace()
        .await
        .expect("test workspace must be bound");

    assert_eq!(
        snapshot.data_dir,
        workspace.path().join(".engram"),
        "ambient ENGRAM_DATA_DIR must not redirect retrieval-eval test state"
    );
}

/// Write the fixture into the workspace and index it via the dispatch path so
/// the branch DB (inside the tempdir under an isolated env) has a real corpus.
async fn write_and_index_fixture(state: &Arc<AppState>, ws: &tempfile::TempDir) {
    let src_dir = ws.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::write(src_dir.join("a.rs"), FIXTURE_SRC).expect("write fixture");
    dispatch_retry(state.clone(), "index_workspace", Some(json!({}))).await;
}

/// Enabled config carrying explicit thresholds.
fn enabled_config_with_thresholds(thresholds: RetrievalEvalThresholds) -> WorkspaceConfig {
    WorkspaceConfig {
        retrieval_eval: RetrievalEvalConfig {
            enabled: true,
            thresholds,
            ..RetrievalEvalConfig::default()
        },
        ..WorkspaceConfig::default()
    }
}

// ── Verification (2): a breached floor is recorded (the RED scenario) ─────────

#[test]
#[serial_test::serial(metrics_writer)]
async fn breached_threshold_is_recorded_in_report() {
    // An impossible floor: `resolution_recall` is clamped to [0, 1], so a floor
    // of 2.0 can never be met by any non-empty run.
    let thresholds = RetrievalEvalThresholds {
        min_resolution_recall: 2.0,
        ..RetrievalEvalThresholds::default()
    };
    let (state, ws) = setup_workspace(enabled_config_with_thresholds(thresholds)).await;
    write_and_index_fixture(&state, &ws).await;

    let report = dispatch_retry(state.clone(), "run_retrieval_eval", Some(json!({}))).await;

    assert_eq!(
        report["thresholds_breached"],
        json!(true),
        "a breached floor must be recorded as thresholds_breached=true; report: {report}"
    );
    let breaches = report["threshold_breaches"]
        .as_array()
        .expect("threshold_breaches must be an array");
    assert!(
        !breaches.is_empty(),
        "a breach must record a human-readable reason; report: {report}"
    );
    assert!(
        breaches
            .iter()
            .any(|b| b.as_str().is_some_and(|s| s.contains("resolution_recall"))),
        "the recorded breach must name resolution_recall; got {breaches:?}"
    );
}

// ── Verification (1)+(3): met / default thresholds do not gate ───────────────

#[test]
#[serial_test::serial(metrics_writer)]
async fn default_thresholds_do_not_produce_a_breach() {
    // Default thresholds are permissive (floors 0.0, ceiling 1.0), modelling an
    // unconfigured workspace: a real run must not be gated (back-compat).
    let (state, ws) = setup_workspace(enabled_config_with_thresholds(
        RetrievalEvalThresholds::default(),
    ))
    .await;
    write_and_index_fixture(&state, &ws).await;

    let report = dispatch_retry(state.clone(), "run_retrieval_eval", Some(json!({}))).await;

    assert_eq!(
        report["thresholds_breached"],
        json!(false),
        "permissive/default thresholds must not gate a healthy run; report: {report}"
    );
    assert_eq!(
        report["threshold_breaches"],
        json!([]),
        "no breach reasons when nothing is breached; report: {report}"
    );
}

// ── Verification (4a): a disabled run never breaches ─────────────────────────

#[test]
#[serial_test::serial(metrics_writer)]
async fn disabled_run_records_no_breach() {
    // Disabled config with an otherwise-impossible floor: the disabled path must
    // short-circuit before any gating.
    let config = WorkspaceConfig {
        retrieval_eval: RetrievalEvalConfig {
            enabled: false,
            thresholds: RetrievalEvalThresholds {
                min_resolution_recall: 2.0,
                ..RetrievalEvalThresholds::default()
            },
            ..RetrievalEvalConfig::default()
        },
        ..WorkspaceConfig::default()
    };
    let (state, _ws) = setup_workspace(config).await;

    let report = dispatch_retry(state.clone(), "run_retrieval_eval", Some(json!({}))).await;

    assert_eq!(report["enabled"], json!(false), "run must report disabled");
    assert_eq!(
        report["thresholds_breached"],
        json!(false),
        "a disabled run must never breach; report: {report}"
    );
}

// ── Verification (4b): an empty enabled run does not FALSE-breach ─────────────

#[test]
#[serial_test::serial(metrics_writer)]
async fn empty_enabled_run_does_not_false_breach() {
    // Enabled with an impossible floor, but NOTHING indexed: there is no metric
    // to measure against the floor, so gating must be skipped rather than firing
    // a false breach on an unmeasured corpus.
    let thresholds = RetrievalEvalThresholds {
        min_resolution_recall: 2.0,
        min_precision_at_k: 2.0,
        ..RetrievalEvalThresholds::default()
    };
    let (state, _ws) = setup_workspace(enabled_config_with_thresholds(thresholds)).await;

    let report = dispatch_retry(state.clone(), "run_retrieval_eval", Some(json!({}))).await;

    assert_eq!(report["enabled"], json!(true), "run must report enabled");
    assert_eq!(
        report["sample_size"],
        json!(0),
        "an un-indexed workspace evaluates nothing; report: {report}"
    );
    assert_eq!(
        report["thresholds_breached"],
        json!(false),
        "an empty run must not false-breach an unmeasured floor; report: {report}"
    );
}
