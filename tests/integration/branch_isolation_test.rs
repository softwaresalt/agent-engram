//! Integration tests for branch-aware database isolation (TASK-009.08).
//!
//! Verifies that binding the same workspace root on two different git branches
//! yields separate SurrealDB directories, that data is not shared across
//! branches, and that `get_workspace_status` surfaces the correct `db_path`.
//!
//! Engram analysis context:
//! * `connect_db(data_dir, branch)` → `{data_dir}/db/{branch}/` — already
//!   branch-isolated at the filesystem level.
//! * `WorkspaceSnapshot.branch` is populated by `resolve_git_branch` in
//!   `lifecycle.rs:67` before `workspace_hash` is called.
//! * `get_workspace_status` reads `snapshot.data_dir` and `snapshot.branch`
//!   to construct `db_path`.
//!
//! Tests: I009-01 through I009-04.

use std::fs;
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::db::connect_db;
use engram::server::state::AppState;
use engram::tools;

// ── I009-01: Different branches → separate DB directories ────────────────────

/// I009-01: Binding the same workspace path with different `.git/HEAD` branch
/// values connects to different `{data_dir}/db/{branch}/` directories.
#[test]
async fn i009_01_different_branches_use_different_db_directories() {
    // GIVEN the same workspace root configured with two branch states in sequence
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    let path = workspace.path().to_string_lossy().to_string();

    // WHEN set_workspace is called with branch "alpha"
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/alpha\n").expect("write HEAD alpha");
    let state_a = Arc::new(AppState::new(10));
    tools::dispatch(state_a.clone(), "set_workspace", Some(json!({ "path": path.clone() })))
        .await
        .expect("set_workspace alpha");
    let snap_a = state_a.snapshot_workspace().await.expect("snapshot alpha");

    // AND set_workspace is called with branch "beta"
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/beta\n").expect("write HEAD beta");
    let state_b = Arc::new(AppState::new(10));
    tools::dispatch(state_b.clone(), "set_workspace", Some(json!({ "path": path })))
        .await
        .expect("set_workspace beta");
    let snap_b = state_b.snapshot_workspace().await.expect("snapshot beta");

    // THEN the branch names in the snapshots are distinct
    assert_ne!(snap_a.branch, snap_b.branch, "snapshots must record different branch names");

    // AND the resolved DB storage directories differ
    let db_a = snap_a.data_dir.join("db").join(&snap_a.branch);
    let db_b = snap_b.data_dir.join("db").join(&snap_b.branch);
    assert_ne!(db_a, db_b, "DB storage directories must differ across branches");
}

// ── I009-02: Branch databases are data-isolated ───────────────────────────────

/// I009-02: A record inserted into the alpha-branch database is not visible
/// from the beta-branch database of the same workspace root.
///
/// This validates that `connect_db(data_dir, branch)` truly isolates SurrealDB
/// namespaces between branches.
#[test]
async fn i009_02_branch_databases_do_not_share_records() {
    // GIVEN two workspace bindings on different branches of the same root
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    let path = workspace.path().to_string_lossy().to_string();

    // Bind "alpha"
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/alpha\n").expect("write HEAD alpha");
    let state_a = Arc::new(AppState::new(10));
    tools::dispatch(state_a.clone(), "set_workspace", Some(json!({ "path": path.clone() })))
        .await
        .expect("set_workspace alpha");
    let snap_a = state_a.snapshot_workspace().await.expect("snapshot alpha");

    // WHEN a record is inserted into the alpha DB
    let db_alpha = connect_db(&snap_a.data_dir, &snap_a.branch)
        .await
        .expect("connect alpha db");
    db_alpha
        .query("CREATE branch_iso_test:alpha_rec SET label = 'alpha-only'")
        .await
        .expect("insert into alpha DB");

    // AND bind "beta" (same root path, different HEAD)
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/beta\n").expect("write HEAD beta");
    let state_b = Arc::new(AppState::new(10));
    tools::dispatch(state_b.clone(), "set_workspace", Some(json!({ "path": path })))
        .await
        .expect("set_workspace beta");
    let snap_b = state_b.snapshot_workspace().await.expect("snapshot beta");

    // THEN the beta DB does not contain the alpha record
    let db_beta = connect_db(&snap_b.data_dir, &snap_b.branch)
        .await
        .expect("connect beta db");
    let rows: Vec<serde_json::Value> = db_beta
        .query("SELECT * FROM branch_iso_test WHERE label = 'alpha-only'")
        .await
        .expect("query beta DB")
        .take(0)
        .expect("take result set");

    assert!(
        rows.is_empty(),
        "beta DB must not contain records from alpha DB; found: {rows:?}"
    );
}

// ── I009-03: WorkspaceSnapshot.branch matches .git/HEAD ──────────────────────

/// I009-03: `WorkspaceSnapshot.branch` is populated from `resolve_git_branch`
/// and reflects the `.git/HEAD` content at the time `set_workspace` was called.
#[test]
async fn i009_03_snapshot_branch_reflects_git_head() {
    // GIVEN a workspace with a specific branch
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/release-1.0\n").expect("write HEAD");
    let path = workspace.path().to_string_lossy().to_string();

    let state = Arc::new(AppState::new(10));

    // WHEN set_workspace is called
    tools::dispatch(state.clone(), "set_workspace", Some(json!({ "path": path })))
        .await
        .expect("set_workspace");

    // THEN the snapshot branch equals the .git/HEAD branch name
    let snap = state.snapshot_workspace().await.expect("snapshot");
    assert_eq!(snap.branch, "release-1.0", "snapshot.branch must match .git/HEAD ref");
}

// ── I009-04: get_workspace_status.db_path encodes the branch ─────────────────

/// I009-04: `get_workspace_status` returns a `db_path` that includes the
/// active branch name as a directory segment, confirming the DB is stored at
/// `{data_dir}/db/{branch}/`.
///
/// Engram analysis: `get_workspace_status` in `lifecycle.rs` builds `db_path`
/// as `snapshot.data_dir.join("db").join(&snapshot.branch)`.
#[test]
async fn i009_04_workspace_status_db_path_encodes_branch() {
    // GIVEN a workspace with a known branch
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/dev-feature\n").expect("write HEAD");
    let path = workspace.path().to_string_lossy().to_string();

    let state = Arc::new(AppState::new(10));
    tools::dispatch(state.clone(), "set_workspace", Some(json!({ "path": path })))
        .await
        .expect("set_workspace");

    // WHEN get_workspace_status is called
    let status = tools::dispatch(state.clone(), "get_workspace_status", None)
        .await
        .expect("get_workspace_status");

    // THEN db_path contains "dev-feature" as a path segment
    let db_path = status
        .get("db_path")
        .and_then(|v| v.as_str())
        .expect("db_path must be present");

    assert!(
        db_path.contains("dev-feature"),
        "db_path must contain branch name 'dev-feature' as a path segment; got: {db_path}"
    );
}
