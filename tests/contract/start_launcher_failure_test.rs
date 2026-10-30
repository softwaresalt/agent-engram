#![cfg(windows)]
//! Copilot errors are independent of index preparation.

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn launcher_returns_copilot_exit_status_without_running_engram() {
    let temp = tempfile::tempdir().expect("fixture");
    let workspace = temp.path();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("start.ps1"),
        workspace.join("start.ps1"),
    )
    .expect("copy launcher");
    let copilot = workspace.join("copilot.ps1");
    fs::write(&copilot, "exit 17\n").expect("Copilot stub");
    fs::write(
        workspace.join("backlogit.cmd"),
        "@echo off\r\nexit /b 0\r\n",
    )
    .expect("backlog stub");
    let path = std::env::join_paths(std::iter::once(workspace.to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .expect("PATH");
    let output = Command::new("pwsh")
        .args(["-NoProfile", "-NonInteractive", "-File"])
        .arg(workspace.join("start.ps1"))
        .env("PATH", path)
        .env("COPILOT_EXE_PATH", copilot)
        .env("GITHUB_TOKEN", "fixture-token")
        .output()
        .expect("launch");
    assert_eq!(output.status.code(), Some(17), "{output:?}");
    assert!(!workspace.join(".engram").exists());
}
