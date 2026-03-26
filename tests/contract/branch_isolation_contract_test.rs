//! Contract tests for branch-aware workspace binding (TASK-009.08).
//!
//! Validates the MCP tool contracts for `set_workspace` and
//! `get_workspace_status` with respect to branch awareness:
//!
//! * Both responses include a `branch` field reflecting `.git/HEAD`.
//! * The `workspace_id` in `set_workspace` will differ across branches once
//!   TASK-009.04 is complete (C009-03 is the red-phase gate for that task).
//!
//! Tests: C009-01 through C009-03.

use std::fs;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::test;

use engram::server::state::AppState;
use engram::tools;

// ── C009-01: set_workspace response schema ────────────────────────────────────

/// C009-01: `set_workspace` returns a response that includes a non-empty,
/// 64-character `workspace_id` hex string.
///
/// Engram analysis: `set_workspace` → `workspace_hash(&canonical, &branch)` →
/// 64-char SHA-256 hex.  Only 1 call site in `lifecycle.rs:66`.
#[test]
async fn c009_01_set_workspace_returns_valid_workspace_id() {
    // GIVEN a workspace with a git repo root
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/feature-branch\n").expect("write HEAD");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    // WHEN set_workspace is called
    let result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    // THEN workspace_id is a 64-char hex string
    let workspace_id = result
        .get("workspace_id")
        .and_then(Value::as_str)
        .expect("workspace_id must be present");
    assert_eq!(
        workspace_id.len(),
        64,
        "workspace_id must be a 64-char SHA-256 hex digest"
    );
    assert!(
        workspace_id.chars().all(|c| c.is_ascii_hexdigit()),
        "workspace_id must be valid hex; got: {workspace_id}"
    );
}

// ── C009-02: get_workspace_status.branch matches .git/HEAD ───────────────────

/// C009-02: `get_workspace_status` returns a `branch` field that matches the
/// branch name recorded in `.git/HEAD` at workspace binding time.
///
/// Engram analysis: `WorkspaceSnapshot.branch` is populated by
/// `resolve_git_branch` in `lifecycle.rs:67`; `get_workspace_status` reads
/// it from the snapshot and serialises it in the `WorkspaceStatus` struct.
#[test]
async fn c009_02_get_workspace_status_branch_matches_git_head() {
    // GIVEN a workspace with a specific branch in .git/HEAD
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/my-branch\n").expect("write HEAD");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    // WHEN get_workspace_status is called
    let status = tools::dispatch(state.clone(), "get_workspace_status", None)
        .await
        .expect("get_workspace_status must succeed");

    // THEN branch reflects the .git/HEAD content
    let branch = status
        .get("branch")
        .and_then(Value::as_str)
        .expect("branch field must be present in get_workspace_status response");
    assert_eq!(branch, "my-branch", "branch must match .git/HEAD ref name");
}

// ── C009-03: workspace_id differs across branches (gate for TASK-009.04) ──────

/// C009-03: Binding the same workspace path with two different `.git/HEAD`
/// branches produces distinct `workspace_id` values.
///
/// **Red phase** — fails until `workspace_hash` includes `branch` in the
/// SHA-256 digest (TASK-009.04).  This test is the acceptance gate for that task.
///
/// Engram analysis: `workspace_hash` currently ignores the `branch` parameter
/// (stub in place); after TASK-009.04 the digest will cover `path + ":" + branch`.
#[test]
async fn c009_03_workspace_id_differs_across_branches() {
    // GIVEN the same workspace path
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    let path = workspace.path().to_string_lossy().to_string();

    // WHEN bound on branch "alpha"
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/alpha\n").expect("write HEAD alpha");
    let state_a = Arc::new(AppState::new(10));
    let result_a = tools::dispatch(
        state_a,
        "set_workspace",
        Some(json!({ "path": path.clone() })),
    )
    .await
    .expect("set_workspace alpha");
    let id_a = result_a
        .get("workspace_id")
        .and_then(Value::as_str)
        .expect("workspace_id alpha")
        .to_string();

    // AND bound on branch "beta"
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/beta\n").expect("write HEAD beta");
    let state_b = Arc::new(AppState::new(10));
    let result_b = tools::dispatch(state_b, "set_workspace", Some(json!({ "path": path })))
        .await
        .expect("set_workspace beta");
    let id_b = result_b
        .get("workspace_id")
        .and_then(Value::as_str)
        .expect("workspace_id beta")
        .to_string();

    // THEN workspace_id is distinct for each branch
    assert_ne!(
        id_a, id_b,
        "workspace_id must differ across branches \
         (worker: include branch in SHA-256 digest in workspace_hash)"
    );
}
