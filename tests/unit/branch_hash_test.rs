//! Unit tests for branch-aware workspace hashing (TASK-009.04).
//!
//! Asserts that [`engram::db::workspace::workspace_hash`] includes the branch
//! in its SHA-256 digest so `workspace_id` uniquely identifies `(path, branch)`
//! pairs, not just `path`.
//!
//! **Red phase**: S081 and S084 currently fail because the stub implementation
//! accepts `branch` but does not include it in the digest.  They will pass
//! once the worker embeds `branch` into the SHA-256 computation.
//!
//! Tests: S081–S084.

use std::path::Path;

use engram::db::workspace::workspace_hash;

// ── S081: Same path, different branches → different hashes ───────────────────

/// S081: `workspace_hash` produces different hashes for the same path on
/// two different branches.
///
/// **Red phase** — fails until branch is embedded in the digest.
#[test]
fn s081_same_path_different_branch_produces_different_hash() {
    // GIVEN the same workspace path
    let path = Path::new("/home/user/project");

    // WHEN workspace_hash is called with two distinct branch names
    let hash_main = workspace_hash(path, "main");
    let hash_dev = workspace_hash(path, "dev");

    // THEN the digests differ
    assert_ne!(
        hash_main, hash_dev,
        "workspace_id must differ across branches for the same path \
         (worker: include branch in SHA-256 — see workspace_hash doc)"
    );
}

// ── S082: Same path and branch → identical hash ───────────────────────────────

/// S082: `workspace_hash` is idempotent: identical `(path, branch)` inputs
/// produce the same digest on repeated calls.
#[test]
fn s082_same_path_same_branch_is_idempotent() {
    // GIVEN a fixed path and branch
    let path = Path::new("/home/user/project");
    let branch = "feature__my-feature";

    // WHEN workspace_hash is called twice
    let first = workspace_hash(path, branch);
    let second = workspace_hash(path, branch);

    // THEN both results are identical
    assert_eq!(first, second, "workspace_hash must be idempotent");
}

// ── S083: Different paths, same branch → different hashes ────────────────────

/// S083: `workspace_hash` produces different hashes for distinct workspace
/// paths even when the branch is the same.
#[test]
fn s083_different_paths_produce_different_hashes() {
    // GIVEN two different workspace paths on the same branch
    let path_a = Path::new("/home/user/project-alpha");
    let path_b = Path::new("/home/user/project-beta");
    let branch = "main";

    // WHEN hashed
    let hash_a = workspace_hash(path_a, branch);
    let hash_b = workspace_hash(path_b, branch);

    // THEN the digests differ
    assert_ne!(
        hash_a, hash_b,
        "distinct paths must yield distinct workspace IDs"
    );
}

// ── S084: Detached-HEAD branch string treated as opaque identifier ─────────────

/// S084: A 12-char commit-SHA string used as a branch (detached HEAD) is
/// treated the same as any other branch string — the same `(path, sha_branch)`
/// pair is deterministic, and it differs from a named branch on the same path.
///
/// **Red phase** — fails until branch is embedded in the digest.
#[test]
fn s084_detached_head_branch_is_distinct_from_named_branch() {
    // GIVEN the same path with a detached-HEAD SHA branch and a named branch
    let path = Path::new("/repos/my-repo");
    let sha_branch = "abc123def456"; // 12-char detached HEAD

    // WHEN hashed
    let h_sha = workspace_hash(path, sha_branch);
    let h_main = workspace_hash(path, "main");

    // THEN the two digests differ (branch is part of the identity)
    assert_ne!(
        h_sha, h_main,
        "detached-HEAD SHA branch must differ from named branch hash \
         (worker: include branch in SHA-256 — see workspace_hash doc)"
    );

    // AND the SHA-branch hash is stable across repeated calls
    let h_sha2 = workspace_hash(path, sha_branch);
    assert_eq!(
        h_sha, h_sha2,
        "detached-HEAD branch hash must be deterministic"
    );
}
