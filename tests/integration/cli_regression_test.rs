//! CLI regression tests for the `engram` binary.
//!
//! These tests run the compiled binary as a subprocess and verify baseline CLI
//! behaviour that must never regress:
//!
//! * `--help` exits 0
//! * `--version` exits 0
//! * `sync --direct` on an empty git workspace exits 0
//! * `sync --direct` on a workspace containing Rust source reports `files_parsed >= 1`
//! * `sync --direct` on a workspace with an oversized file still exits 0 (resilience)
//!
//! **Windows note**: daemon-backed tests (those using IPC) are ignored on
//! Windows due to a `CozoDB` initialisation issue. The `--direct` tests here do
//! not start a daemon and run on all platforms.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

// ── Helpers ────────────────────────────────────────────────────────────────────

fn engram_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_engram"))
}

/// Initialise a minimal git workspace (creates `.git/HEAD`).
fn init_git(dir: &Path) {
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
}

/// Write a file relative to `workspace`, creating parent directories as needed.
fn write_file(workspace: &Path, relative_path: &str, content: &str) {
    let full_path = workspace.join(relative_path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(&full_path, content).expect("write file");
}

/// Run `engram <args>` in `workspace` with an isolated data directory.
///
/// Returns `(exit_code, stdout, stderr)`.
fn run_direct(workspace: &Path, args: &[&str]) -> (i32, String, String) {
    let bin = engram_bin();
    let data_dir = workspace.join(".engram-regression-data");
    let output = Command::new(&bin)
        .args(args)
        .current_dir(workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env("ENGRAM_DATA_DIR", &data_dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (code, stdout, stderr)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

/// `engram --help` must exit 0.
#[test]
fn cli_regression_help_flag_exits_zero() {
    let tmp = TempDir::new().expect("tempdir");
    let (code, _stdout, stderr) = run_direct(tmp.path(), &["--help"]);
    assert_eq!(code, 0, "`engram --help` should exit 0; stderr: {stderr}");
}

/// `engram --version` must exit 0.
#[test]
fn cli_regression_version_flag_exits_zero() {
    let tmp = TempDir::new().expect("tempdir");
    let (code, _stdout, stderr) = run_direct(tmp.path(), &["--version"]);
    assert_eq!(
        code, 0,
        "`engram --version` should exit 0; stderr: {stderr}"
    );
}

/// `sync --direct` on an empty git workspace must exit 0 and return a
/// valid JSON-RPC 2.0 envelope containing a `result` field.
#[test]
fn cli_regression_direct_sync_empty_workspace_exits_zero() {
    let tmp = TempDir::new().expect("tempdir");
    init_git(tmp.path());

    let (code, stdout, stderr) = run_direct(tmp.path(), &["sync", "--direct"]);

    assert_eq!(
        code, 0,
        "`sync --direct` on empty workspace should exit 0; stderr: {stderr}"
    );

    // Must produce a JSON-RPC 2.0 result envelope.
    assert!(
        stdout.contains("\"jsonrpc\"") && stdout.contains("\"result\""),
        "stdout must be a JSON-RPC 2.0 result envelope; stdout: {stdout}"
    );
}

/// `sync --direct` on a workspace containing a Rust source file must report
/// `files_parsed >= 1` in the result payload.
#[test]
fn cli_regression_direct_sync_indexes_rust_source_file() {
    let tmp = TempDir::new().expect("tempdir");
    init_git(tmp.path());

    // Write a small Rust source file.
    write_file(
        tmp.path(),
        "src/lib.rs",
        "pub fn hello() -> &'static str { \"hi\" }\n",
    );

    let (code, stdout, stderr) = run_direct(tmp.path(), &["sync", "--direct"]);

    assert_eq!(
        code, 0,
        "`sync --direct` with a source file should exit 0; stderr: {stderr}"
    );

    // Parse the JSON-RPC envelope and inspect files_parsed.
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    let files_parsed = value
        .pointer("/result/files_parsed")
        .or_else(|| value.pointer("/result/result/files_parsed"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    assert!(
        files_parsed >= 1,
        "expected at least 1 parsed file; stdout: {stdout}"
    );
}

/// `sync --direct` on a workspace containing an oversized file alongside a
/// normal file must still exit 0.  The oversized file is a policy skip, not a
/// fatal error.
#[test]
fn cli_regression_direct_sync_with_oversized_file_resilience() {
    let tmp = TempDir::new().expect("tempdir");
    init_git(tmp.path());

    // Write a normal, parseable Rust file.
    write_file(tmp.path(), "src/lib.rs", "pub fn small() {}\n");

    // Write a file large enough to exceed the default 1 MiB limit.
    // We use 1 MiB + 1 byte of content (repeated ASCII so it is valid UTF-8).
    let oversized_content = "x".repeat(1_048_577); // 1 MiB + 1 byte
    write_file(tmp.path(), "src/big_generated.rs", &oversized_content);

    let (code, stdout, stderr) = run_direct(tmp.path(), &["sync", "--direct"]);

    assert_eq!(
        code, 0,
        "`sync --direct` must exit 0 even when an oversized file is present; stderr: {stderr}"
    );

    // The result envelope must still be valid JSON.
    assert!(
        stdout.contains("\"result\""),
        "stdout must contain a result field; stdout: {stdout}"
    );
}
