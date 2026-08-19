#![cfg(windows)]

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use tempfile::TempDir;

fn write_batch(path: &Path, body: &str) {
    fs::write(path, body.replace('\n', "\r\n")).expect("write batch fixture");
}

#[test]
fn launcher_timeout_cleanup_wait_is_explicitly_bounded() {
    let launcher = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("start.ps1"))
        .expect("read launcher");

    assert!(
        launcher.contains("$process.WaitForExit($cleanupTimeoutMs)"),
        "timeout cleanup must use its explicit bounded wait"
    );
    assert!(
        !launcher.contains("$process.WaitForExit()"),
        "timeout cleanup must not wait indefinitely after killing the exact process"
    );
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
    fs::write(
        fixture.path().join("engram.ps1"),
        "Start-Sleep -Seconds 10\nexit 1\n",
    )
    .expect("write slow Engram fixture");
    fs::write(
        fixture.path().join("copilot.ps1"),
        "[IO.File]::WriteAllText($env:COPILOT_MARKER, 'invoked')\nexit 0\n",
    )
    .expect("write Copilot fixture");

    let marker = fixture.path().join("copilot-invoked.txt");
    let path = std::env::join_paths(std::iter::once(fixture.path().to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
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
        .env("COPILOT_EXE_PATH", fixture.path().join("copilot.ps1"))
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
        elapsed < Duration::from_secs(8),
        "Engram direct/fallback pre-warm must share one wall-clock budget; the 8s limit allows \
         hosted-runner process startup overhead while remaining below the >20s sequential path; \
         elapsed: {elapsed:?}"
    );
}

#[test]
fn launcher_timeout_does_not_terminate_unowned_descendant() {
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
    fs::write(
        fixture.path().join("engram.ps1"),
        "$pwsh = (Get-Command pwsh -ErrorAction Stop).Source\n\
         Start-Process -FilePath $pwsh -ArgumentList @('-NoProfile', '-NonInteractive', \
         '-ExecutionPolicy', 'Bypass', '-File', $env:ENGRAM_DESCENDANT_SCRIPT) \
         -WindowStyle Hidden\n\
         Start-Sleep -Seconds 30\n",
    )
    .expect("write parent Engram fixture");
    let descendant_script = fixture.path().join("engram-descendant.ps1");
    fs::write(
        &descendant_script,
        "Start-Sleep -Milliseconds 4500\n\
         [IO.File]::WriteAllText($env:ENGRAM_DESCENDANT_MARKER, 'survived')\n\
         exit 0\n",
    )
    .expect("write descendant fixture");
    fs::write(fixture.path().join("copilot.ps1"), "exit 0\n").expect("write Copilot fixture");

    let descendant_marker = fixture.path().join("descendant-survived.txt");
    let path = std::env::join_paths(std::iter::once(fixture.path().to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .expect("compose fixture PATH");

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
        .env("COPILOT_EXE_PATH", fixture.path().join("copilot.ps1"))
        .env("ENGRAM_DESCENDANT_SCRIPT", &descendant_script)
        .env("ENGRAM_DESCENDANT_MARKER", &descendant_marker)
        .env("ENGRAM_PREWARM_TIMEOUT_MS", "3000")
        .output()
        .expect("run launcher descendant contract");

    assert!(
        output.status.success(),
        "launcher must fail open after timing out its command process; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let marker_deadline = Instant::now() + Duration::from_secs(5);
    while !descendant_marker.is_file() && Instant::now() < marker_deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        descendant_marker.is_file(),
        "the short-lived descendant must survive command timeout long enough to write its marker"
    );
}
