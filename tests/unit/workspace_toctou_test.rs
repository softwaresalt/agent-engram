//! U1 — adversarial TOCTOU harness for workspace admission (plan 568B257C).
//!
//! These black-box scenarios drive [`engram::db::workspace::canonicalize_workspace`]
//! against fixtures whose git admin chain is attacked with static link and
//! directory substitutions. The genuinely temporal check/use race (a swap
//! *between* validation and use) is exercised deterministically by the colocated
//! `toctou_tests` module inside `src/db/workspace.rs`; here we cover the
//! substitutions that can be staged before resolution begins.
//!
//! Every scenario is deterministic — no sleeps, no thread racing. A scenario
//! that genuinely needs privileges unavailable in the environment prints an
//! explicit `SKIPPED:` line and still asserts everything it can, never a silent
//! pass.

use std::fs;
use std::path::{Path, PathBuf};
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

fn admin_dir_of(worktree: &Path) -> PathBuf {
    let gitfile = fs::read_to_string(worktree.join(".git")).expect("read linked gitfile");
    let pointer = gitfile
        .trim()
        .strip_prefix("gitdir: ")
        .expect("native linked gitfile directive")
        .to_owned();
    PathBuf::from(pointer)
}

/// Create a directory link (Unix symlink / Windows directory junction) at `at`
/// pointing to `target`. Junctions need no elevation; a failure is surfaced so
/// the caller can decide whether to skip.
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

/// Create a file link to attacker-controlled content. On Unix this is a symlink;
/// on Windows a file symlink, which may require privilege (Developer Mode or
/// `SeCreateSymbolicLinkPrivilege`).
#[cfg(unix)]
fn make_file_link(at: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, at)
}

#[cfg(windows)]
fn make_file_link(at: &Path, target: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, at)
}

// ── Scenario 1: ancestor substitution in the admin chain ─────────────────────

/// The `worktrees` directory inside the common `.git` dir — an ancestor of the
/// linked-worktree admin directory — is replaced by a link pointing at a forged
/// sibling `worktrees` tree that contains an internally consistent same-named
/// admin directory. Admission MUST be rejected.
#[test]
fn ancestor_substitution_in_admin_chain_is_rejected() {
    let fixture = TempDir::new().expect("fixture tempdir");
    let primary = fixture.path().join("primary");
    let worktree = fixture.path().join("worktree");
    initialize_primary(&primary);
    add_linked_worktree(&primary, &worktree, "feature/ancestor");

    let admin_dir = admin_dir_of(&worktree);
    let worktrees_dir = admin_dir
        .parent()
        .expect("admin dir has a worktrees parent")
        .to_path_buf();
    let admin_name = admin_dir.file_name().expect("admin dir has a name");

    // Forge an internally consistent sibling worktrees tree: the same-named
    // admin dir carries the real relative commondir and the real backlink, so
    // only the ancestor link — not inconsistent metadata — should trip rejection.
    let commondir = fs::read_to_string(admin_dir.join("commondir")).expect("read commondir");
    let backlink = fs::read_to_string(admin_dir.join("gitdir")).expect("read admin backlink");
    let forged = fixture.path().join("forged_worktrees");
    let forged_admin = forged.join(admin_name);
    fs::create_dir_all(&forged_admin).expect("create forged admin dir");
    fs::write(forged_admin.join("commondir"), &commondir).expect("write forged commondir");
    fs::write(forged_admin.join("gitdir"), &backlink).expect("write forged backlink");
    fs::write(forged_admin.join("HEAD"), "ref: refs/heads/hijacked\n").expect("write forged HEAD");

    let aside = worktrees_dir.with_file_name("worktrees__aside");
    fs::rename(&worktrees_dir, &aside).expect("move real worktrees aside");
    if let Err(error) = make_dir_link(&worktrees_dir, &forged) {
        fs::rename(&aside, &worktrees_dir).expect("restore real worktrees after link failure");
        // Deliberately NOT a skip. Directory links need no elevation on either
        // supported platform, so a failure here means the fixture is broken, not
        // that the environment lacks a capability. A silently-passing security
        // test is treated as a failing one.
        panic!(
            "ancestor_substitution_in_admin_chain_is_rejected: could not create the directory \
             link the scenario depends on: {error}. Directory links require no elevation on any \
             supported platform, so this is a fixture failure, not an environment skip."
        );
    }

    let workspace = worktree.to_str().expect("worktree path is valid UTF-8");
    let result = canonicalize_workspace(workspace);
    assert!(
        result.is_err(),
        "a linked ancestor in the admin chain must not be admitted; got {result:?}"
    );
    // Discriminate on provenance too: the forged tree names `hijacked`, so the
    // rejection must not merely be an unrelated fixture failure.
    assert!(
        !format!("{result:?}").contains("hijacked"),
        "attacker-controlled content must never reach the admitted result; got {result:?}"
    );
}

// ── Scenario 2: metadata leaf check/read substitution ────────────────────────

/// A metadata leaf (`commondir`) inside the admin directory is replaced by a
/// link to attacker-controlled content. The check-then-read pattern must not
/// source metadata through a link; admission MUST be rejected.
#[test]
fn metadata_leaf_substitution_between_check_and_read_is_rejected() {
    let fixture = TempDir::new().expect("fixture tempdir");
    let primary = fixture.path().join("primary");
    let worktree = fixture.path().join("worktree");
    initialize_primary(&primary);
    add_linked_worktree(&primary, &worktree, "feature/leaf");

    let admin_dir = admin_dir_of(&worktree);

    // Attacker-controlled content the substituted leaf resolves to.
    let attacker_common = fixture.path().join("attacker_common");
    fs::create_dir_all(attacker_common.join("worktrees")).expect("create attacker common tree");
    fs::create_dir_all(attacker_common.join("objects")).expect("create attacker objects");
    fs::create_dir_all(attacker_common.join("refs")).expect("create attacker refs");
    fs::write(attacker_common.join("HEAD"), "ref: refs/heads/hijacked\n")
        .expect("write attacker common HEAD");
    let attacker_commondir = fixture.path().join("attacker_commondir");
    fs::write(
        &attacker_commondir,
        format!("{}\n", attacker_common.display()),
    )
    .expect("write attacker commondir content");

    let leaf = admin_dir.join("commondir");
    fs::remove_file(&leaf).expect("remove real commondir leaf");
    if let Err(error) = make_file_link(&leaf, &attacker_commondir) {
        if error.kind() == std::io::ErrorKind::PermissionDenied {
            println!(
                "SKIPPED: metadata_leaf_substitution_between_check_and_read_is_rejected — file \
                 symlink creation requires privilege in this environment: {error}"
            );
            // Still assert what we can: with the leaf missing, admission fails.
            let workspace = worktree.to_str().expect("worktree path is valid UTF-8");
            assert!(
                canonicalize_workspace(workspace).is_err(),
                "a linked worktree whose commondir leaf is absent must not be admitted"
            );
            return;
        }
        panic!("unexpected error creating metadata leaf link: {error}");
    }

    let workspace = worktree.to_str().expect("worktree path is valid UTF-8");
    let result = canonicalize_workspace(workspace);
    assert!(
        result.is_err(),
        "a metadata leaf sourced through a link must not be admitted; got {result:?}"
    );
}

// ── Scenario 3: no regression for legitimate roots ───────────────────────────

/// A legitimate primary checkout and a legitimate native `git worktree` are both
/// still admitted, and each resolves to its expected branch.
#[test]
fn no_regression_primary_checkout_and_native_worktree_are_admitted() {
    let fixture = TempDir::new().expect("fixture tempdir");
    let primary = fixture.path().join("primary");
    let worktree = fixture.path().join("worktree");
    initialize_primary(&primary);
    add_linked_worktree(&primary, &worktree, "feature/native");

    let primary_text = primary.to_str().expect("primary path is valid UTF-8");
    let primary_canonical =
        canonicalize_workspace(primary_text).expect("a primary checkout must be admitted");
    assert_eq!(
        primary_canonical
            .canonicalize()
            .expect("re-canonicalize admitted primary"),
        primary.canonicalize().expect("canonical primary"),
        "the admitted primary root must resolve to the same object as the checkout"
    );
    assert_eq!(
        resolve_git_branch(&primary).expect("resolve primary branch"),
        "main",
        "the primary checkout must report its active branch"
    );

    let worktree_text = worktree.to_str().expect("worktree path is valid UTF-8");
    let worktree_canonical =
        canonicalize_workspace(worktree_text).expect("a native worktree must be admitted");
    assert_eq!(
        worktree_canonical
            .canonicalize()
            .expect("re-canonicalize admitted worktree"),
        worktree.canonicalize().expect("canonical worktree"),
        "the admitted worktree root must resolve to the same object as the checkout"
    );
    assert_eq!(
        resolve_git_branch(&worktree).expect("resolve worktree branch"),
        "feature__native",
        "the native worktree must report its own active branch"
    );
}
