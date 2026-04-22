use std::fs;
use std::sync::Arc;

use engram::db::workspace::load_or_create_workspace_id;
use engram::errors::{EngramError, WorkspaceError};
use engram::server::state::AppState;
use engram::tools;
use serde_json::json;
use uuid::Uuid;

#[test]
fn two_canonical_paths_bind_single_daemon() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let typed_error = WorkspaceError::AmbiguousBind {
        expected_id: Uuid::new_v4(),
        found_id: Uuid::new_v4(),
        path: workspace.path().to_path_buf(),
    };

    assert!(
        typed_error
            .to_string()
            .contains("Workspace bind is ambiguous"),
        "typed error should remain available to the future implementation"
    );

    let canonical = workspace
        .path()
        .canonicalize()
        .expect("canonical workspace path");
    let canonical_id = load_or_create_workspace_id(&canonical)
        .expect("workspace-id should load for canonical path");
    let original_id = load_or_create_workspace_id(workspace.path())
        .expect("workspace-id should load for original path");

    assert_eq!(canonical_id, original_id);
}

#[tokio::test]
async fn conflicting_workspace_id_returns_ambiguous_bind() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(1));
    let path = workspace.path().to_string_lossy().to_string();
    let initial = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path.clone() })),
    )
    .await
    .expect("initial workspace bind should succeed");
    let _ = initial;
    let expected_id = load_or_create_workspace_id(workspace.path())
        .expect("workspace-id should exist after initial bind");

    let conflicting_id = Uuid::new_v4();
    assert_ne!(expected_id, conflicting_id);
    fs::write(
        workspace.path().join(".engram").join(".workspace-id"),
        format!("{conflicting_id}\n"),
    )
    .expect("overwrite workspace-id");

    let err = tools::dispatch(state, "set_workspace", Some(json!({ "path": path })))
        .await
        .expect_err("conflicting workspace-id should be rejected");

    assert!(matches!(
        err,
        EngramError::Workspace(WorkspaceError::AmbiguousBind {
            expected_id: expected,
            found_id: found,
            ..
        }) if expected == expected_id && found == conflicting_id
    ));
    assert!(
        err.to_string()
            .contains("Remove stale runtime state and retry"),
        "ambiguous bind should carry a remediation hint"
    );
}
