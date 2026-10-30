#![cfg(windows)]
//! Session launch must not depend on index maintenance.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

#[test]
fn launcher_starts_without_engram_and_preserves_arguments_and_workspace() {
    let temp = TempDir::new().expect("fixture");
    let workspace = temp.path().join("work space");
    fs::create_dir(&workspace).expect("workspace");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("start.ps1"),
        workspace.join("start.ps1"),
    )
    .expect("copy launcher");
    fs::write(
        workspace.join("backlogit.cmd"),
        "@echo off\r\nexit /b 0\r\n",
    )
    .expect("backlog stub");
    let copilot = workspace.join("copilot.ps1");
    fs::write(
        &copilot,
        r"[IO.File]::WriteAllLines($env:LAUNCH_RECORD, @($PWD.Path, $env:COPILOT_HOME) + $args)
exit 0
",
    )
    .expect("Copilot stub");
    let path = std::env::join_paths(std::iter::once(workspace.clone()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .expect("PATH");
    let record = workspace.join("launch.txt");
    let output = Command::new("pwsh")
        .args(["-NoProfile", "-NonInteractive", "-File"])
        .arg(workspace.join("start.ps1"))
        .args(["--probe", "argument with spaces"])
        .current_dir(temp.path())
        .env("PATH", path)
        .env("COPILOT_EXE_PATH", copilot)
        .env("LAUNCH_RECORD", &record)
        .env("ENGRAM_EXE_PATH", workspace.join("missing engram.exe"))
        .env("ENGRAM_PREWARM_TIMEOUT_MS", "obsolete-not-a-budget")
        .output()
        .expect("launch");
    assert!(output.status.success(), "{output:?}");
    let recorded = fs::read_to_string(record).expect("Copilot launched");
    let lines: Vec<_> = recorded.lines().collect();
    assert_eq!(
        Path::new(lines[0])
            .canonicalize()
            .expect("recorded workspace"),
        workspace.canonicalize().expect("fixture workspace")
    );
    assert!(!lines[1].is_empty(), "workspace-local Copilot state");
    assert_eq!(&lines[2..], ["--probe", "argument with spaces"]);
    assert!(!workspace.join(".engram").exists());
}

#[test]
fn launcher_has_no_indexing_or_preflight_entrypoint() {
    let script = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("start.ps1"))
        .expect("launcher");
    assert!(!script.contains("Invoke-EngramPreflight"));
    assert!(!script.contains("ENGRAM_PREWARM_TIMEOUT_MS"));
    assert!(!script.contains("ProcessStartInfo"));
}
