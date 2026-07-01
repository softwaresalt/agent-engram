//! Contract test for the `engram verify` exit-code + stderr diagnostics
//! contract (064.002-T).
//!
//! Pins the autoharness `pre_task_completion` gate semantics by running the
//! built binary as a subprocess and asserting the real process exit codes:
//! - `0` conformant (and non-markdown targets, which have nothing to validate);
//! - `1` non-conformant, with findings emitted to stderr for agent context;
//! - `2` I/O or usage error (missing / unreadable file).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

fn engram_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_engram"))
}

/// Run `engram verify <arg>` with cwd set to `workspace`.
/// Returns `(exit_code, stdout, stderr)`.
fn verify_cmd(workspace: &Path, arg: &str) -> (i32, String, String) {
    let output = Command::new(engram_bin())
        .arg("verify")
        .arg(arg)
        .current_dir(workspace)
        .output()
        .unwrap_or_else(|e| panic!("failed to run engram verify: {e}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (code, stdout, stderr)
}

/// C-VF-01: a conformant markdown file exits 0.
#[test]
fn conformant_markdown_exits_zero() {
    let tmp = TempDir::new().expect("tempdir");
    fs::write(
        tmp.path().join("good.md"),
        "---\nid: 001-T\ntitle: Good\n---\n\n# Heading\n\nBody text.\n",
    )
    .expect("write fixture");

    let (code, _stdout, stderr) = verify_cmd(tmp.path(), "good.md");
    assert_eq!(code, 0, "conformant markdown must exit 0; stderr: {stderr}");
}

/// C-VF-02: malformed frontmatter exits non-zero and writes findings to stderr.
#[test]
fn malformed_frontmatter_exits_one_with_stderr_findings() {
    let tmp = TempDir::new().expect("tempdir");
    fs::write(
        tmp.path().join("bad.md"),
        "---\n: invalid: yaml: {\n---\n\n# Body\n",
    )
    .expect("write fixture");

    let (code, _stdout, stderr) = verify_cmd(tmp.path(), "bad.md");
    assert_eq!(code, 1, "malformed frontmatter must exit 1");
    assert!(
        stderr.contains("frontmatter.malformed"),
        "findings must be written to stderr for agent context, got: {stderr}"
    );
}

/// C-VF-03: a missing file is an I/O error and exits 2.
#[test]
fn missing_file_exits_two() {
    let tmp = TempDir::new().expect("tempdir");
    let (code, _stdout, _stderr) = verify_cmd(tmp.path(), "does-not-exist.md");
    assert_eq!(code, 2, "missing file is an I/O error -> exit 2");
}

/// C-VF-04: a non-markdown target exits 0 (nothing to validate in Phase 1a).
#[test]
fn non_markdown_target_exits_zero() {
    let tmp = TempDir::new().expect("tempdir");
    fs::write(tmp.path().join("notes.txt"), "plain text, not markdown").expect("write fixture");

    let (code, _stdout, _stderr) = verify_cmd(tmp.path(), "notes.txt");
    assert_eq!(code, 0, "non-markdown target must exit 0 in Phase 1a");
}
