//! RED harness for the direct-sync daemon-mode boundary.
//!
//! Plan unit F11 (142.016-T) requires `sync --direct` to proceed in managed
//! mode and return a stable refusal in read-server mode.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use engram::errors::codes::DIRECT_SYNC_REFUSED;
use engram::models::config::DaemonMode;
use tempfile::TempDir;

fn engram_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_engram"))
}

fn init_git(dir: &Path) {
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
}

fn write_mode_config(workspace: &Path, mode: DaemonMode) {
    let engram_dir = workspace.join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");
    fs::write(
        engram_dir.join("config.toml"),
        format!("mode = \"{}\"\n", mode.as_str()),
    )
    .expect("write config.toml");
}

fn run_direct_sync(workspace: &Path) -> (i32, String, String) {
    let data_dir = workspace.join(".engram-test-data");
    let output = Command::new(engram_bin())
        .args(["sync", "--direct", "--json"])
        .current_dir(workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env("ENGRAM_DATA_DIR", &data_dir)
        .output()
        .expect("run engram sync --direct");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (code, stdout, stderr)
}

#[test]
fn managed_mode_direct_sync_does_not_return_the_refusal_error() {
    let tempdir = TempDir::new().expect("tempdir");
    let workspace = tempdir
        .path()
        .canonicalize()
        .expect("canonicalize workspace");
    init_git(&workspace);
    write_mode_config(&workspace, DaemonMode::Managed);

    let (code, stdout, stderr) = run_direct_sync(&workspace);
    assert_eq!(
        code, 0,
        "managed mode must keep direct sync available; stdout: {stdout}; stderr: {stderr}"
    );
}

#[test]
fn read_server_mode_direct_sync_returns_the_stable_refusal_error() {
    let tempdir = TempDir::new().expect("tempdir");
    let workspace = tempdir
        .path()
        .canonicalize()
        .expect("canonicalize workspace");
    init_git(&workspace);
    write_mode_config(&workspace, DaemonMode::ReadServer);

    let (code, stdout, stderr) = run_direct_sync(&workspace);
    assert_eq!(
        code, 1,
        "read-server mode must refuse direct sync; stdout: {stdout}; stderr: {stderr}"
    );

    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("direct-sync refusal must be JSON");
    assert_eq!(payload["error"]["code"], DIRECT_SYNC_REFUSED);
    assert_eq!(payload["error"]["name"], "DirectSyncRefused");
    assert_eq!(payload["error"]["data"]["operation"], "sync_workspace");
}
