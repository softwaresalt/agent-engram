//! Contract tests for the retrieval-eval MCP tools (081.002-T).
//!
//! Validates the empty-state contract for `run_retrieval_eval` and
//! `get_retrieval_eval_report`: both tools are discoverable, return a
//! well-formed empty [`RetrievalEvalReport`] when disabled or before any run,
//! and tolerate unknown params. Compute is out of scope for this task.
//!
//! Scenarios:
//! 1. Both tool schemas present in the static catalog.
//! 2. `run_retrieval_eval` while disabled → empty report with `enabled:false`.
//! 3. `get_retrieval_eval_report` before any run → empty report.
//! 4. Unknown params are tolerated.

use std::fs;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::test;

use engram::models::config::WorkspaceConfig;
use engram::server::state::AppState;
use engram::tools;

/// Bind a temp workspace with the given config and return the state handle.
async fn setup_workspace(config: WorkspaceConfig) -> (Arc<AppState>, tempfile::TempDir) {
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    state.set_workspace_config(Some(config)).await;
    (state, workspace)
}

fn assert_empty_report(report: &Value) {
    assert!(report.get("branch").is_some(), "must have branch");
    assert!(
        report.get("evaluated_at").is_some(),
        "must have evaluated_at"
    );
    let semantic = report.get("semantic").expect("must have semantic");
    assert_eq!(
        semantic["queries"],
        json!(0),
        "empty semantic has 0 queries"
    );
    assert_eq!(semantic["mrr"], json!(0.0), "empty semantic has 0 mrr");
    let graph = report.get("graph").expect("must have graph");
    assert_eq!(
        graph["call_sites"],
        json!(0),
        "empty graph has 0 call sites"
    );
    assert_eq!(graph["resolved"], json!(0), "empty graph has 0 resolved");
}

// ── Scenario 1: both schemas present in the catalog ──────────────────────────

#[test]
async fn both_tools_present_in_catalog() {
    let tools = engram::shim::tools_catalog::all_tools();
    let names: std::collections::HashSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(
        names.contains("run_retrieval_eval"),
        "run_retrieval_eval must be discoverable in the catalog"
    );
    assert!(
        names.contains("get_retrieval_eval_report"),
        "get_retrieval_eval_report must be discoverable in the catalog"
    );
}

// ── Scenario 2: run while disabled → empty report + enabled:false ─────────────

#[test]
async fn run_while_disabled_returns_empty_report() {
    // Default config → retrieval_eval disabled.
    let (state, _ws) = setup_workspace(WorkspaceConfig::default()).await;

    let result = tools::dispatch(state.clone(), "run_retrieval_eval", Some(json!({})))
        .await
        .expect("run_retrieval_eval must succeed");

    assert_eq!(
        result["enabled"],
        json!(false),
        "disabled config must report enabled:false"
    );
    assert_empty_report(&result);
}

// ── Scenario 3: get before any run → empty report ────────────────────────────

#[test]
async fn get_before_run_returns_empty_report() {
    let (state, _ws) = setup_workspace(WorkspaceConfig::default()).await;

    let result = tools::dispatch(state.clone(), "get_retrieval_eval_report", Some(json!({})))
        .await
        .expect("get_retrieval_eval_report must succeed");

    assert_empty_report(&result);
}

// ── Scenario 4: unknown params tolerated ─────────────────────────────────────

#[test]
async fn unknown_params_tolerated() {
    let (state, _ws) = setup_workspace(WorkspaceConfig::default()).await;

    let result = tools::dispatch(
        state.clone(),
        "run_retrieval_eval",
        Some(json!({ "unexpected": "value", "k": 3 })),
    )
    .await
    .expect("unknown params must be tolerated");

    assert_empty_report(&result);
}
