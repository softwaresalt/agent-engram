#![cfg(windows)]
//! F52 launcher failure and ownership matrix (142.056-T): every typed preflight
//! stage fails closed with a stage diagnostic, paths with spaces stay intact,
//! and cleanup never kills processes the launcher does not own.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tempfile::TempDir;

const STAGES: [&str; 7] = [
    "Build",
    "Seal",
    "Publish",
    "DaemonVerified",
    "HealthVerified",
    "CliProbeVerified",
    "McpProbeVerified",
];

/// Fixture `engram.exe`: records argv, prints a verdict per `FIX_MODE`, and
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
            println!("{{\"state\":\"Failed\",\"stage\":\"{}\"}}", env::var("FIX_STAGE").unwrap_or_default());
            eprintln!("boom-stderr");
            std::process::exit(1);
        }
        _ => {
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

/// Compiles the fixture once per test binary; the directory is leaked on purpose.
fn fixture_exe() -> &'static Path {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        let dir = TempDir::new().expect("fixture dir").keep();
        let src = dir.join("fixture.rs");
        fs::write(&src, FIXTURE_SRC).expect("fixture source");
        let exe = dir.join("engram.exe");
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let built = Command::new(rustc)
            .args(["--edition", "2021", "-C", "debuginfo=0", "-o"])
            .arg(&exe)
            .arg(&src)
            .status()
            .expect("compile fixture engram");
        assert!(built.success(), "fixture engram must compile");
        exe
    })
}

struct Run {
    dir: PathBuf,
    success: bool,
    output: String,
    elapsed: Duration,
    _guard: TempDir,
}

impl Run {
    fn copilot_launched(&self) -> bool {
        self.dir.join("copilot-invoked.txt").exists()
    }
}

/// Runs a copy of `start.ps1` from a `work space` directory against the fixture.
fn run_launcher(mode: &str, stage: &str, outer_ms: &str) -> Run {
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
    fs::copy(
        fixture_exe(),
        dir.join("target").join("debug").join("engram.exe"),
    )
    .expect("install fixture engram");
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
        .env("FIX_MODE", mode)
        .env("FIX_STAGE", stage)
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

fn wait_for(path: &Path, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while !path.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    path.is_file()
}

#[test]
fn every_failed_stage_blocks_copilot_and_reports_stage() {
    for stage in STAGES {
        let run = run_launcher("fail", stage, "15000");
        assert!(!run.success, "{stage}: must exit non-zero; {}", run.output);
        assert!(!run.copilot_launched(), "{stage}: Copilot launched");
        assert!(
            run.output.contains(&format!(r#""stage":"{stage}""#)),
            "{stage}: fixture verdict missing; {}",
            run.output
        );
        assert!(
            run.output
                .contains(&format!("engram preflight failed at stage {stage}.")),
            "{stage}: launcher stage diagnostic missing; {}",
            run.output
        );
    }
}

#[test]
fn spaced_launch_path_passes_one_argument_per_element() {
    let run = run_launcher("ok", "", "15000");
    assert!(run.success && run.copilot_launched(), "{}", run.output);
    let args = fs::read_to_string(run.dir.join("args.txt")).expect("argv recorded");
    let parts: Vec<&str> = args.trim_end().split('|').collect();
    assert_eq!(parts.len(), 4, "argv elements: {args:?}");
    assert!(parts[1].ends_with("work space"), "{args:?}");
}

#[test]
fn spaced_cleanup_path_terminates_the_direct_child() {
    let run = run_launcher("hang", "", "4000");
    assert!(!run.success && !run.copilot_launched(), "{}", run.output);
    assert!(run.elapsed < Duration::from_secs(25), "{:?}", run.elapsed);
    let pid = fs::read_to_string(run.dir.join("child.pid")).expect("child pid recorded");
    let alive = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid.trim()), "/NH"])
        .output()
        .expect("tasklist");
    assert!(
        !String::from_utf8_lossy(&alive.stdout).contains("engram.exe"),
        "direct child must be terminated; pid {pid}"
    );
}

#[test]
fn unowned_descendant_survives_cleanup_and_copilot_stays_blocked() {
    let run = run_launcher("hang", "", "4000");
    assert!(!run.copilot_launched(), "{}", run.output);
    assert!(
        wait_for(&run.dir.join("descendant-survived.txt"), 15),
        "launcher must not kill the unowned descendant"
    );
}

#[test]
fn launcher_has_no_worker_placeholder_marker() {
    let launcher = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("start.ps1"))
        .expect("read launcher");
    assert!(
        !launcher.contains(concat!("Worker: ", "142.056-T")),
        "placeholder marker must be replaced by the typed-stage diagnostic"
    );
}
