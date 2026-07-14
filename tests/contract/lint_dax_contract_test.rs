//! Contract tests for the `lint_dax` MCP tool (P6, `085.006-T`).
//!
//! Locks the agent-native surface: the tool is registered in the catalog in
//! lockstep with `TOOL_COUNT`, returns the `{ conformant, findings[] }` schema
//! over the bound workspace's indexed Power BI model(s), honours the optional
//! `model_path` selector, and returns a `WorkspaceNotFound` error for a path
//! that names no indexed model.
//!
//! Tests: S-LINTDAX-01 through S-LINTDAX-06.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::errors::codes::WORKSPACE_NOT_FOUND;
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::shim::tools_catalog;
use engram::tools;

/// Build a workspace on disk with a `powerbi` registry source and a two-file
/// TMDL model that contains one broken column reference, then return the temp
/// dir and a snapshot bound to it.
fn bound_workspace() -> (tempfile::TempDir, WorkspaceSnapshot) {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();

    let engram_dir = workspace.join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");
    std::fs::write(
        engram_dir.join("registry.yaml"),
        "sources:\n  - type: powerbi\n    path: models\n",
    )
    .expect("write registry.yaml");

    let tables = workspace
        .join("models")
        .join("Sales.SemanticModel")
        .join("definition")
        .join("tables");
    std::fs::create_dir_all(&tables).expect("create tmdl dirs");
    std::fs::write(
        tables.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure Total = SUM(Sales[Amount])\n\
         \x20\x20measure Broken = SUM(Sales[Nonexistent])\n",
    )
    .expect("write Sales.tmdl");
    std::fs::write(
        tables.join("Date.tmdl"),
        "table Date\n\
         \x20\x20column Year\n\
         \x20\x20\x20\x20dataType: int64\n",
    )
    .expect("write Date.tmdl");

    let snapshot = WorkspaceSnapshot {
        workspace_id: "lint-dax".to_string(),
        workspace_uuid: "uuid-lint-dax".to_string(),
        branch: "lint-dax".to_string(),
        data_dir: root.path().join("data"),
        path: workspace.to_string_lossy().to_string(),
        last_flush: None,
        stale_files: false,
        connection_count: 1,
        file_mtimes: HashMap::new(),
    };
    (root, snapshot)
}

/// S-LINTDAX-01: `TOOL_COUNT` is 21 and equals the catalog length.
#[test]
async fn tool_count_is_twenty_one_and_matches_catalog() {
    assert_eq!(
        tools_catalog::TOOL_COUNT,
        21,
        "TOOL_COUNT must be bumped to 21"
    );
    assert_eq!(
        tools_catalog::all_tools().len(),
        tools_catalog::TOOL_COUNT,
        "catalog length must equal TOOL_COUNT"
    );
}

/// S-LINTDAX-02: `lint_dax` is present in the tool catalog.
#[test]
async fn lint_dax_is_registered_in_catalog() {
    let present = tools_catalog::all_tools()
        .iter()
        .any(|tool| tool.name.as_ref() == "lint_dax");
    assert!(present, "lint_dax must appear in the tool catalog");
}

/// S-LINTDAX-03: `lint_dax` requires a bound workspace.
#[test]
async fn lint_dax_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let err = tools::dispatch(state, "lint_dax", None)
        .await
        .expect_err("expected workspace-not-set error");
    // WorkspaceNotSet is the expected class; assert it is a workspace error.
    assert_eq!(err.to_response().error.name, "WorkspaceNotSet");
}

/// S-LINTDAX-04: with a bound workspace, `lint_dax` returns the
/// `{ conformant, findings[] }` schema and each finding carries the expected
/// fields.
#[test]
async fn lint_dax_returns_conformant_findings_schema() {
    let (_root, snapshot) = bound_workspace();
    let state = Arc::new(AppState::new(10));
    state.set_workspace(snapshot).await.expect("set workspace");

    let value = tools::dispatch(state, "lint_dax", None)
        .await
        .expect("lint_dax should succeed");

    let conformant = value
        .get("conformant")
        .and_then(serde_json::Value::as_bool)
        .expect("response must carry a boolean `conformant`");
    assert!(!conformant, "the broken-ref fixture must be non-conformant");

    let findings = value
        .get("findings")
        .and_then(serde_json::Value::as_array)
        .expect("response must carry a `findings` array");
    assert!(
        !findings.is_empty(),
        "the broken-ref fixture must produce findings"
    );

    let broken = findings
        .iter()
        .find(|f| {
            f.get("rule").and_then(serde_json::Value::as_str) == Some("dax.broken_column_ref")
        })
        .expect("expected a dax.broken_column_ref finding");
    assert!(
        broken
            .get("message")
            .and_then(serde_json::Value::as_str)
            .is_some()
    );
    assert!(
        broken
            .get("severity")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "each finding must carry a severity"
    );
    // `line` is present as an optional field (null for adapter-sourced findings).
    assert!(
        broken.get("line").is_some(),
        "each finding must carry a `line` key"
    );
}

/// S-LINTDAX-05: the optional `model_path` selector filters to one model scope.
#[test]
async fn lint_dax_model_path_selector_targets_one_model() {
    let (_root, snapshot) = bound_workspace();
    let state = Arc::new(AppState::new(10));
    state.set_workspace(snapshot).await.expect("set workspace");

    let params = json!({
        "model_path": "models/Sales.SemanticModel/definition/tables/Sales.tmdl"
    });
    let value = tools::dispatch(state, "lint_dax", Some(params))
        .await
        .expect("lint_dax with model_path should succeed");

    let findings = value
        .get("findings")
        .and_then(serde_json::Value::as_array)
        .expect("findings array");
    assert!(
        findings
            .iter()
            .any(|f| f.get("rule").and_then(serde_json::Value::as_str)
                == Some("dax.broken_column_ref")),
        "the selected Sales model still reports its broken ref"
    );
}

/// S-LINTDAX-06: a `model_path` that is not an indexed model is a
/// `WorkspaceNotFound` error result.
#[test]
async fn lint_dax_unindexed_model_path_is_error() {
    let (_root, snapshot) = bound_workspace();
    let state = Arc::new(AppState::new(10));
    state.set_workspace(snapshot).await.expect("set workspace");

    let params = json!({
        "model_path": "models/Ghost.SemanticModel/definition/tables/Ghost.tmdl"
    });
    let err = tools::dispatch(state, "lint_dax", Some(params))
        .await
        .expect_err("an unindexed model_path must error");
    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_FOUND);
}
