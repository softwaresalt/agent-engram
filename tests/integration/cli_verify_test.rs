//! Integration tests for `engram verify <path>` cross-platform path handling
//! (064.003-T).
//!
//! Runs the built binary as a subprocess and asserts real process exit codes
//! against on-disk fixtures under `tests/fixtures/verify/`:
//! 1. a conformant fixture exits `0`;
//! 2. a malformed-frontmatter fixture exits non-zero;
//! 3. a Windows-style backslash path is normalized and accepted;
//! 4. a `..` traversal path escaping the workspace root is rejected (exit `2`);
//! 5. a relative `<path>` is resolved against the `--workspace` root, not the
//!    process CWD, when the two differ (Constitution Principle III/IV).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

fn engram_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_engram"))
}

/// The crate root, used as the workspace/cwd for fixture-relative scenarios.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run `engram verify <arg>` with cwd set to `cwd`. Returns `(code, stdout, stderr)`.
fn verify_in(cwd: &Path, arg: &str) -> (i32, String, String) {
    let output = Command::new(engram_bin())
        .arg("verify")
        .arg(arg)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("failed to run engram verify: {e}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (code, stdout, stderr)
}

/// Run `engram verify <arg> --workspace <workspace>` with cwd set to `cwd`, and
/// with `ENGRAM_WORKSPACE` cleared so only the explicit flag governs the root.
/// Returns `(code, stdout, stderr)`.
fn verify_ws(cwd: &Path, workspace: &Path, arg: &str) -> (i32, String, String) {
    let output = Command::new(engram_bin())
        .arg("verify")
        .arg(arg)
        .arg("--workspace")
        .arg(workspace)
        .env_remove("ENGRAM_WORKSPACE")
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("failed to run engram verify: {e}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (code, stdout, stderr)
}

/// I-VF-01: a conformant fixture (forward-slash relative path) exits 0.
#[test]
fn conformant_fixture_exits_zero() {
    let (code, _stdout, stderr) = verify_in(&crate_root(), "tests/fixtures/verify/conformant.md");
    assert_eq!(code, 0, "conformant fixture must exit 0; stderr: {stderr}");
}

/// I-VF-02: a malformed-frontmatter fixture exits non-zero.
#[test]
fn malformed_fixture_exits_nonzero() {
    let (code, _stdout, _stderr) = verify_in(&crate_root(), "tests/fixtures/verify/malformed.md");
    assert_eq!(code, 1, "malformed fixture must exit 1 (non-conformant)");
}

/// I-VF-03: a Windows-style backslash path is normalized and accepted.
#[test]
fn backslash_path_is_normalized_and_accepted() {
    let (code, _stdout, stderr) =
        verify_in(&crate_root(), "tests\\fixtures\\verify\\conformant.md");
    assert_eq!(
        code, 0,
        "backslash path must normalize to forward-slash and exit 0; stderr: {stderr}"
    );
}

/// I-VF-04: a `..` traversal path escaping the workspace root is rejected.
#[test]
fn parent_traversal_path_is_rejected() {
    // Build: <tmp>/outside.md (conformant) and <tmp>/workspace/ as the cwd.
    let tmp = TempDir::new().expect("tempdir");
    fs::write(
        tmp.path().join("outside.md"),
        "---\nid: outside\ntitle: Outside\n---\n\n# Outside\n\nReachable only via traversal.\n",
    )
    .expect("write outside fixture");
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).expect("create workspace dir");

    // From the workspace, `../outside.md` escapes the workspace root and must be
    // rejected with exit 2 rather than read (Constitution Principle III).
    let (code, _stdout, _stderr) = verify_in(&workspace, "../outside.md");
    assert_eq!(
        code, 2,
        "a '..' path escaping the workspace root must be rejected with exit 2"
    );
}

/// I-VF-05: a relative `<path>` resolves against the `--workspace` root, not the
/// process CWD, when the two differ.
///
/// Containment (Constitution Principle III/IV) means a relative target is read
/// under the declared workspace root. A conformant file that exists only under
/// the CWD (outside the workspace) must NOT be read: it resolves under the
/// workspace, is missing there, and exits `2` — never silently reading CWD.
/// Conversely, a file under the workspace root resolves and exits `0` even when
/// the CWD differs from that root.
#[test]
fn relative_path_resolves_against_workspace_not_cwd() {
    let workspace = TempDir::new().expect("workspace tempdir");
    let cwd = TempDir::new().expect("cwd tempdir");

    // A conformant file that exists ONLY under the CWD, not the workspace root.
    fs::write(
        cwd.path().join("only-in-cwd.md"),
        "---\nid: cwd\ntitle: Only In Cwd\n---\n\n# Only In Cwd\n\nReachable only via the CWD.\n",
    )
    .expect("write cwd-only fixture");

    // Resolving `<path>` under the workspace (not the CWD) means this file is
    // missing under the workspace root -> exit 2. It must never be read from the
    // CWD, which would undermine containment.
    let (code, _stdout, _stderr) = verify_ws(cwd.path(), workspace.path(), "only-in-cwd.md");
    assert_eq!(
        code, 2,
        "a relative <path> must resolve under --workspace (missing there -> exit 2), \
         not be read from the CWD"
    );

    // A conformant file under the WORKSPACE root resolves and exits 0 even when
    // the process CWD differs from the workspace root.
    fs::write(
        workspace.path().join("in-workspace.md"),
        "---\nid: ws\ntitle: In Workspace\n---\n\n# In Workspace\n\nResolved under the workspace root.\n",
    )
    .expect("write workspace fixture");
    let (code, _stdout, stderr) = verify_ws(cwd.path(), workspace.path(), "in-workspace.md");
    assert_eq!(
        code, 0,
        "a relative <path> under the workspace root must resolve and exit 0; stderr: {stderr}"
    );
}
