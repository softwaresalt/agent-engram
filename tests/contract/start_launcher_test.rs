#![cfg(windows)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use tempfile::TempDir;

/// Fixture `engram.exe`: records its argv, prints a verdict per `FIX_MODE`, and
/// (hang mode) spawns an unowned descendant that outlives a launcher kill.
const FIXTURE_SRC: &str = r#"
use std::{env, fs, process::{Command, Stdio}, thread, time::Duration};
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mode = env::var("FIX_MODE").unwrap_or_default();
    if mode == "child" {
        thread::sleep(Duration::from_millis(6000));
        let _ = fs::write(env::var("FIX_MARKER").unwrap(), "survived");
        return;
    }
    if let Ok(p) = env::var("FIX_ARGS") {
        let mut f = fs::OpenOptions::new().create(true).append(true).open(p).unwrap();
        std::io::Write::write_all(&mut f, format!("{}\n", args.join("|")).as_bytes()).unwrap();
    }
    match mode.as_str() {
        "ok" => println!("{{\"state\":\"Succeeded\"}}"),
        "fail" => {
            println!("{{\"state\":\"Failed\",\"stage\":\"Seal\"}}");
            eprintln!("boom-stderr");
            std::process::exit(1);
        }
        "silent" => {}
        "lie" => println!("{{\"state\":\"Failed\",\"stage\":\"Build\"}}"),
        _ => {
            // The descendant inherits this process' pipes and holds them open.
            println!("partial-out");
            eprintln!("partial-err");
            fs::write(env::var("FIX_PID").unwrap(), std::process::id().to_string()).unwrap();
            Command::new(env::current_exe().unwrap()).env("FIX_MODE", "child")
                .stdin(Stdio::null()).stdout(Stdio::inherit()).stderr(Stdio::inherit())
                .spawn().unwrap();
            thread::sleep(Duration::from_secs(30));
        }
    }
}
"#;

struct Run {
    dir: PathBuf,
    success: bool,
    output: String,
    elapsed: Duration,
    _guard: TempDir,
}

impl Run {
    fn copilot_marker(&self) -> PathBuf {
        self.dir.join("copilot-invoked.txt")
    }
    fn descendant_marker(&self) -> PathBuf {
        self.dir.join("descendant-survived.txt")
    }
}

/// Runs a copy of `start.ps1` from a directory whose name contains spaces.
/// `mode` of `None` installs no `target\debug\engram.exe` (missing executable).
fn run_launcher(mode: Option<&str>, outer_ms: &str) -> Run {
    let guard = TempDir::new().expect("launcher fixture");
    let dir = guard.path().join("work space");
    fs::create_dir_all(dir.join("target").join("debug")).expect("fixture dirs");
    let launcher = dir.join("start.ps1");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("start.ps1"),
        &launcher,
    )
    .expect("copy launcher");
    fs::write(dir.join("backlogit.cmd"), "@echo off\r\nexit /b 0\r\n").expect("backlogit stub");
    fs::write(
        dir.join("copilot.ps1"),
        "[IO.File]::WriteAllText($env:COPILOT_MARKER, 'invoked')\nexit 0\n",
    )
    .expect("copilot stub");
    if mode.is_some() {
        let src = dir.join("fixture.rs");
        fs::write(&src, FIXTURE_SRC).expect("fixture source");
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let built = Command::new(rustc)
            .args(["--edition", "2021", "-C", "debuginfo=0", "-o"])
            .arg(dir.join("target").join("debug").join("engram.exe"))
            .arg(&src)
            .status()
            .expect("compile fixture engram");
        assert!(built.success(), "fixture engram must compile");
    }
    let path = std::env::join_paths(std::iter::once(dir.clone()).chain(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    )))
    .expect("compose fixture PATH");
    let started = Instant::now();
    let out = Command::new("pwsh")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&launcher)
        .env("PATH", path)
        .env("COPILOT_EXE_PATH", dir.join("copilot.ps1"))
        .env("COPILOT_MARKER", dir.join("copilot-invoked.txt"))
        .env("FIX_MODE", mode.unwrap_or(""))
        .env("FIX_ARGS", dir.join("args.txt"))
        .env("FIX_MARKER", dir.join("descendant-survived.txt"))
        .env("FIX_PID", dir.join("child.pid"))
        .env("ENGRAM_PREWARM_TIMEOUT_MS", outer_ms)
        .output()
        .expect("run launcher contract");
    let output = format!(
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Run {
        dir,
        success: out.status.success(),
        output,
        elapsed: started.elapsed(),
        _guard: guard,
    }
}

#[test]
fn launcher_timeout_cleanup_wait_is_explicitly_bounded() {
    let launcher = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("start.ps1"))
        .expect("read launcher");
    assert!(launcher.contains("$process.WaitForExit($cleanupTimeoutMs)"));
    assert!(!launcher.contains("$process.WaitForExit()"));
    assert!(launcher.contains("ArgumentList"));
    assert!(
        !launcher.contains("Start-Process"),
        "Start-Process -ArgumentList is forbidden"
    );
}

#[test]
fn launcher_launches_copilot_after_succeeded_preflight() {
    let run = run_launcher(Some("ok"), "15000");
    assert!(
        run.success,
        "Succeeded preflight must launch; {}",
        run.output
    );
    assert!(
        run.copilot_marker().is_file(),
        "Copilot must launch; {}",
        run.output
    );
    let args = fs::read_to_string(run.dir.join("args.txt")).expect("preflight argv recorded");
    assert_eq!(
        args.lines().count(),
        1,
        "exactly one preflight call, no sync/bind: {args:?}"
    );
    let lines: Vec<&str> = args.trim_end().split('|').collect();
    assert_eq!(lines.len(), 4, "one argv element per argument: {args:?}");
    assert_eq!(lines[0], "preflight");
    assert!(
        lines[1].ends_with("work space"),
        "path with a space stays one argument: {args:?}"
    );
    assert_eq!(lines[2], "--timeout-ms");
    assert_eq!(
        lines[3], "12000",
        "T = outer budget - (2000 grace + 1000 margin)"
    );
}

#[test]
fn launcher_fails_closed_and_reports_typed_stage_on_failed_preflight() {
    let run = run_launcher(Some("fail"), "15000");
    assert!(!run.success, "failed preflight must block; {}", run.output);
    assert!(
        !run.copilot_marker().exists(),
        "Copilot must not launch; {}",
        run.output
    );
    assert!(
        run.output.contains(r#""stage":"Seal""#),
        "typed stage reported; {}",
        run.output
    );
    assert!(
        run.output.contains("boom-stderr"),
        "stderr reported; {}",
        run.output
    );
}

#[test]
fn launcher_blocks_on_non_succeeded_verdict_despite_zero_exit() {
    let run = run_launcher(Some("lie"), "15000");
    assert!(
        !run.success,
        "non-Succeeded verdict must block; {}",
        run.output
    );
    assert!(!run.copilot_marker().exists(), "{}", run.output);
    assert!(run.output.contains("reported state"), "{}", run.output);
}

#[test]
fn launcher_blocks_when_exit_zero_without_succeeded_verdict() {
    let run = run_launcher(Some("silent"), "15000");
    assert!(!run.success, "silent exit 0 must block; {}", run.output);
    assert!(!run.copilot_marker().exists(), "{}", run.output);
    assert!(
        run.output.contains("without a Succeeded verdict"),
        "{}",
        run.output
    );
}

#[test]
fn launcher_blocks_when_preflight_executable_is_missing() {
    let run = run_launcher(None, "15000");
    assert!(
        !run.success,
        "missing executable must block; {}",
        run.output
    );
    assert!(!run.copilot_marker().exists(), "{}", run.output);
}

#[test]
fn launcher_timeout_blocks_and_kills_only_its_own_child() {
    let run = run_launcher(Some("hang"), "4000");
    assert!(!run.success, "timeout must block; {}", run.output);
    assert!(!run.copilot_marker().exists(), "{}", run.output);
    assert!(
        run.elapsed < Duration::from_secs(25),
        "bounded wait; elapsed {:?}",
        run.elapsed
    );
    assert!(
        run.output.contains("partial-err") && run.output.contains("partial-out"),
        "partial diagnostics survive an inherited open pipe; {}",
        run.output
    );
    let pid = fs::read_to_string(run.dir.join("child.pid")).expect("child pid recorded");
    let alive = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid.trim()), "/NH"])
        .output()
        .expect("tasklist");
    assert!(
        !String::from_utf8_lossy(&alive.stdout).contains("engram.exe"),
        "the launcher's direct child must be terminated; pid {pid}"
    );
    let deadline = Instant::now() + Duration::from_secs(15);
    while !run.descendant_marker().is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        run.descendant_marker().is_file(),
        "unowned descendant must survive cleanup"
    );
}
