//! U2 — platform reparse/symlink adversarial harness (plan 568B257C).
//!
//! Substitutes validated directories inside the common git dir with directory
//! junctions (Windows) or symlinks (Unix) and asserts explicit rejection — not
//! merely "does not crash". A mandatory cross-platform no-regression case (plan
//! finding S3) proves the tightened policy is scoped to the validated git chain
//! and does not reject a legitimate reparse point *above* the workspace root.
//!
//! All scenarios are deterministic. When a link genuinely cannot be created in
//! the environment an explicit `SKIPPED:` line is printed and the strongest
//! available assertion is still made.

use std::fs;
use std::path::Path;
use std::process::Command;

use engram::db::workspace::{canonicalize_workspace, resolve_git_branch};
use tempfile::TempDir;

fn run_git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("run git fixture command");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn initialize_primary(primary: &Path) {
    fs::create_dir_all(primary).expect("create primary checkout");
    run_git(primary, &["init", "--initial-branch=main"]);
    run_git(primary, &["config", "user.name", "Engram Test"]);
    run_git(
        primary,
        &["config", "user.email", "engram-test@example.invalid"],
    );
    fs::write(primary.join("README.md"), "# fixture\n").expect("write tracked fixture");
    run_git(primary, &["add", "README.md"]);
    run_git(primary, &["commit", "-m", "fixture"]);
}

fn add_linked_worktree(primary: &Path, worktree: &Path, branch: &str) {
    let output = Command::new("git")
        .args(["worktree", "add", "-b", branch])
        .arg(worktree)
        .current_dir(primary)
        .output()
        .expect("create linked worktree");
    assert!(
        output.status.success(),
        "git worktree add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Create a directory link (Unix symlink / Windows directory junction) at `at`
/// pointing to `target`. Junctions require no elevation.
#[cfg(unix)]
fn make_dir_link(at: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, at)
}

#[cfg(windows)]
fn make_dir_link(at: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt as _;

    // `mklink` is a `cmd.exe` builtin; passing the link and target as separate
    // escaped arguments makes the builtin misparse the path. A single raw
    // command line quotes the operands correctly and needs no elevation for /J.
    let status = Command::new("cmd")
        .arg("/c")
        .raw_arg(format!(
            "mklink /J \"{}\" \"{}\"",
            at.display(),
            target.display()
        ))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("mklink /J failed"))
    }
}

/// Build a linked-worktree fixture, replace `subdir` inside the common `.git`
/// directory with a directory link to its own moved-aside content, and assert
/// that admission of the worktree is explicitly rejected.
fn assert_common_subdir_link_rejected(subdir: &str) {
    let fixture = TempDir::new().expect("fixture tempdir");
    let primary = fixture.path().join("primary");
    let worktree = fixture.path().join("worktree");
    initialize_primary(&primary);
    add_linked_worktree(&primary, &worktree, "feature/reparse");

    let target = primary.join(".git").join(subdir);
    assert!(
        target.is_dir(),
        "the fixture must contain a real {subdir} directory to substitute"
    );
    let aside = target.with_file_name(format!("{subdir}__aside"));
    fs::rename(&target, &aside).expect("move real subdir aside");
    if let Err(error) = make_dir_link(&target, &aside) {
        fs::rename(&aside, &target).expect("restore real subdir after link failure");
        // Deliberately NOT a skip. Directory links need no elevation on either
        // supported platform (`mklink /J` on Windows, `symlink` on Unix), so a
        // failure here means the fixture — not the environment — is broken. The
        // plan mandate treats a silently-passing security test as a failing one,
        // and this scenario is the security-critical substitution case.
        panic!(
            "assert_common_subdir_link_rejected({subdir}): could not create the directory link \
             the scenario depends on: {error}. Directory links require no elevation on any \
             supported platform, so this is a fixture failure, not an environment skip."
        );
    }

    let workspace = worktree.to_str().expect("worktree path is valid UTF-8");
    let result = canonicalize_workspace(workspace);
    assert!(
        result.is_err(),
        "substituting {subdir} with a directory link must be rejected; got {result:?}"
    );
}

// ── Windows: junction substitution of worktrees / objects / refs ─────────────

#[cfg(windows)]
#[test]
fn windows_worktrees_junction_is_rejected() {
    assert_common_subdir_link_rejected("worktrees");
}

#[cfg(windows)]
#[test]
fn windows_objects_junction_is_rejected() {
    assert_common_subdir_link_rejected("objects");
}

#[cfg(windows)]
#[test]
fn windows_refs_junction_is_rejected() {
    assert_common_subdir_link_rejected("refs");
}

// ── Unix: symlink substitution of worktrees / objects ────────────────────────

#[cfg(unix)]
#[test]
fn unix_worktrees_symlink_is_rejected() {
    assert_common_subdir_link_rejected("worktrees");
}

#[cfg(unix)]
#[test]
fn unix_objects_symlink_is_rejected() {
    assert_common_subdir_link_rejected("objects");
}

// ── Cross-platform: reparse point above the workspace root (S3) ──────────────

/// A workspace root placed *under* a link/junction ancestor — i.e. the reparse
/// point is ABOVE the workspace root and outside the validated git chain — MUST
/// still be admitted. This confirms the tightened policy is scoped to the
/// validated chain, not to unrelated ancestors.
#[test]
fn legitimate_reparse_point_above_workspace_root_is_still_admitted() {
    let fixture = TempDir::new().expect("fixture tempdir");
    let real_base = fixture.path().join("real_base");
    fs::create_dir_all(&real_base).expect("create real base");
    let project = real_base.join("project");
    initialize_primary(&project);

    let link_base = fixture.path().join("link_base");
    if let Err(error) = make_dir_link(&link_base, &real_base) {
        println!(
            "SKIPPED: legitimate_reparse_point_above_workspace_root_is_still_admitted — cannot \
             create a directory link in this environment: {error}"
        );
        let direct = project.to_str().expect("project path is valid UTF-8");
        assert!(
            canonicalize_workspace(direct).is_ok(),
            "the underlying primary checkout must still be admitted"
        );
        return;
    }

    let via_link = link_base.join("project");
    let workspace = via_link
        .to_str()
        .expect("linked project path is valid UTF-8");
    let result = canonicalize_workspace(workspace);
    assert!(
        result.is_ok(),
        "a reparse point above the workspace root, outside the validated git chain, must remain \
         admitted; got {result:?}"
    );
    assert_eq!(
        resolve_git_branch(&via_link).expect("resolve branch through the reparse ancestor"),
        "main",
        "branch resolution must still succeed through a legitimate ancestor reparse point"
    );
}
