#![cfg(windows)]

use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use tempfile::TempDir;

fn write_batch(path: &Path, body: &str) {
    fs::write(path, body.replace('\n', "\r\n")).expect("write batch fixture");
}

#[test]
fn launcher_fails_open_to_copilot_within_one_prewarm_budget() {
    let fixture = TempDir::new().expect("launcher fixture");
    let launcher = fixture.path().join("start.ps1");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("start.ps1"),
        &launcher,
    )
    .expect("copy launcher");

    write_batch(
        &fixture.path().join("backlogit.cmd"),
        "@echo off\nexit /b 0\n",
    );
    write_batch(
        &fixture.path().join("engram.cmd"),
        concat!(
            "@echo off\n",
            "pwsh -NoProfile -NonInteractive -Command ",
            "\"Start-Sleep -Seconds 2\"\n",
            "exit /b 1\n",
        ),
    );
    write_batch(
        &fixture.path().join("copilot.cmd"),
        concat!(
            "@echo off\n",
            "pwsh -NoProfile -NonInteractive -Command ",
            "\"[IO.File]::WriteAllText($env:COPILOT_MARKER, 'invoked')\"\n",
            "exit /b 0\n",
        ),
    );

    let marker = fixture.path().join("copilot-invoked.txt");
    let path = std::env::join_paths(std::iter::once(fixture.path().to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_else(OsString::new)),
    ))
    .expect("compose fixture PATH");

    let started = Instant::now();
    let output = Command::new("pwsh")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&launcher)
        .env("PATH", path)
        .env("COPILOT_EXE_PATH", fixture.path().join("copilot.cmd"))
        .env("COPILOT_MARKER", &marker)
        .env("ENGRAM_PREWARM_TIMEOUT_MS", "500")
        .output()
        .expect("run launcher contract");
    let elapsed = started.elapsed();

    assert!(
        output.status.success(),
        "launcher must fail open to Copilot; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        marker.is_file(),
        "Copilot fixture must be invoked after bounded pre-warm"
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "Engram direct/fallback pre-warm must share one wall-clock budget; elapsed: {elapsed:?}"
    );
}
