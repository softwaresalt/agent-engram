//! Unit tests for branch detection and workspace hashing (TASK-009.07).
//!
//! Covers [`engram::db::workspace::resolve_git_branch`],
//! [`engram::db::workspace::workspace_hash`], and the slash-sanitisation
//! behaviour of `sanitize_branch_for_path` exercised indirectly through
//! `resolve_git_branch`.
//!
//! All implementations already exist in `src/db/workspace.rs`; these tests
//! add the coverage mandated by the feature acceptance criteria.
//!
//! Tests: S075–S080.

use std::fs;
use std::path::Path;

use engram::db::workspace::{resolve_git_branch, workspace_hash};
use tempfile::TempDir;

// ── S075: Standard branch ref ─────────────────────────────────────────────────

/// S075: `resolve_git_branch` correctly parses a standard branch ref and
/// returns the branch name without the `ref: refs/heads/` prefix.
#[test]
fn s075_resolve_git_branch_standard_ref() {
    // GIVEN a workspace whose .git/HEAD holds a standard branch ref
    let ws = TempDir::new().expect("tempdir");
    let git_dir = ws.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    // WHEN resolve_git_branch is called
    let branch = resolve_git_branch(ws.path()).expect("must resolve standard ref");

    // THEN the bare branch name is returned
    assert_eq!(branch, "main");
}

// ── S076: Nested branch name (sanitize_branch_for_path via resolve) ───────────

/// S076: `resolve_git_branch` sanitises nested branch names by replacing
/// every `/` with `__`, producing a path-safe directory name.
///
/// This indirectly exercises `sanitize_branch_for_path`.
#[test]
fn s076_resolve_git_branch_nested_name_sanitised() {
    // GIVEN a workspace with a nested branch name containing slashes
    let ws = TempDir::new().expect("tempdir");
    let git_dir = ws.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(
        git_dir.join("HEAD"),
        "ref: refs/heads/feature/nested-name\n",
    )
    .expect("write HEAD");

    // WHEN resolve_git_branch is called
    let branch = resolve_git_branch(ws.path()).expect("must resolve nested ref");

    // THEN every slash is replaced with double-underscore
    assert_eq!(branch, "feature__nested-name");
    assert!(
        !branch.contains('/'),
        "sanitised branch must contain no forward slashes; got: {branch}"
    );
}

// ── S077: Detached HEAD ───────────────────────────────────────────────────────

/// S077: When `.git/HEAD` contains a raw commit SHA (detached HEAD state),
/// `resolve_git_branch` returns the first 12 characters of the SHA.
#[test]
fn s077_resolve_git_branch_detached_head_uses_sha_prefix() {
    // GIVEN a workspace whose .git/HEAD holds a raw 40-char commit SHA
    let ws = TempDir::new().expect("tempdir");
    let git_dir = ws.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    let full_sha = "abc123def456789012345678901234567890abcd";
    fs::write(git_dir.join("HEAD"), full_sha).expect("write detached HEAD");

    // WHEN resolve_git_branch is called
    let branch = resolve_git_branch(ws.path()).expect("must resolve detached HEAD");

    // THEN the first 12 characters of the SHA are returned
    assert_eq!(branch.len(), 12, "detached HEAD branch must be 12 chars");
    assert_eq!(&branch, &full_sha[..12]);
}

// ── S078: Missing .git/HEAD returns error ─────────────────────────────────────

/// S078: `resolve_git_branch` returns an error when `.git/HEAD` is absent.
#[test]
fn s078_resolve_git_branch_missing_head_returns_error() {
    // GIVEN a workspace with a .git dir but no HEAD file
    let ws = TempDir::new().expect("tempdir");
    fs::create_dir_all(ws.path().join(".git")).expect("create .git");
    // HEAD is intentionally not written

    // WHEN resolve_git_branch is called
    let result = resolve_git_branch(ws.path());

    // THEN an error is returned (WorkspaceError::NotGitRoot)
    assert!(result.is_err(), "missing HEAD must yield an error");
}

// ── S079: No .git directory returns error ────────────────────────────────────

/// S079: `resolve_git_branch` returns an error when the workspace has no
/// `.git` directory at all.
#[test]
fn s079_resolve_git_branch_no_git_dir_returns_error() {
    // GIVEN a bare temporary directory with no .git subdirectory
    let ws = TempDir::new().expect("tempdir");

    // WHEN resolve_git_branch is called
    let result = resolve_git_branch(ws.path());

    // THEN an error is returned
    assert!(result.is_err(), "absent .git directory must yield an error");
}

// ── S080: workspace_hash determinism ─────────────────────────────────────────

/// S080: `workspace_hash` produces the same 64-char hex digest for identical
/// `(path, branch)` inputs on every call.
#[test]
fn s080_workspace_hash_is_deterministic() {
    // GIVEN a fixed path and branch
    let path = Path::new("/home/user/project");
    let branch = "main";

    // WHEN workspace_hash is called twice with the same inputs
    let first = workspace_hash(path, branch);
    let second = workspace_hash(path, branch);

    // THEN both results are identical 64-character lowercase hex strings
    assert_eq!(first, second, "workspace_hash must be deterministic");
    assert_eq!(first.len(), 64, "SHA-256 hex must be 64 chars");
    assert!(
        first.chars().all(|c| c.is_ascii_hexdigit()),
        "result must be valid lowercase hex; got: {first}"
    );
}
