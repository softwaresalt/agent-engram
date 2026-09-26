#![cfg(windows)]
#![forbid(unsafe_code)]

use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use tempfile::TempDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreflightStage {
    Build,
    Seal,
    Publish,
    DaemonVerified,
    HealthVerified,
    CliProbeVerified,
    McpProbeVerified,
}

impl PreflightStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Build => "Build",
            Self::Seal => "Seal",
            Self::Publish => "Publish",
            Self::DaemonVerified => "DaemonVerified",
            Self::HealthVerified => "HealthVerified",
            Self::CliProbeVerified => "CliProbeVerified",
            Self::McpProbeVerified => "McpProbeVerified",
        }
    }
}

const PREFLIGHT_STAGES: [PreflightStage; 7] = [
    PreflightStage::Build,
    PreflightStage::Seal,
    PreflightStage::Publish,
    PreflightStage::DaemonVerified,
    PreflightStage::HealthVerified,
    PreflightStage::CliProbeVerified,
    PreflightStage::McpProbeVerified,
];

const ENGRAM_FIXTURE_SOURCE: &str = r##"
use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    process::{self, Command, Stdio},
    thread,
    time::Duration,
};

fn fixture_path(name: &str) -> PathBuf {
    env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("missing F52 fixture variable {name}"))
}

fn write_fixture_file(name: &str, contents: impl AsRef<[u8]>) {
    fs::write(fixture_path(name), contents).expect("write F52 fixture marker");
}

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.iter().any(|argument| argument == "--f52-unowned-descendant") {
        write_fixture_file(
            "F52_FIXTURE_DESCENDANT_PID",
            process::id().to_string(),
        );
        while !fixture_path("F52_FIXTURE_RELEASE_DESCENDANT").is_file() {
            thread::sleep(Duration::from_millis(25));
        }
        write_fixture_file("F52_FIXTURE_DESCENDANT_SURVIVED", "survived");
        return;
    }

    write_fixture_file("F52_FIXTURE_INVOCATION", arguments.join("\n"));
    match env::var("F52_FIXTURE_MODE").as_deref() {
        Ok("failed") => {
            let stage = env::var("F52_FIXTURE_STAGE")
                .unwrap_or_else(|_| panic!("missing F52 fixture stage"));
            let evidence = format!(r#"{{"state":"Failed","stage":"{stage}"}}"#);
            write_fixture_file("F52_FIXTURE_RESULT", &evidence);
            writeln!(io::stdout().lock(), "{evidence}").expect("write typed failure to stdout");
            process::exit(23);
        }
        Ok("succeeded") => {
            let evidence = r#"{"state":"Succeeded"}"#;
            write_fixture_file("F52_FIXTURE_RESULT", evidence);
            writeln!(io::stdout().lock(), "{evidence}").expect("write success to stdout");
        }
        Ok("cleanup") => {
            write_fixture_file("F52_FIXTURE_OWNER_PID", process::id().to_string());
            let executable = env::current_exe().expect("resolve F52 fixture executable");
            drop(
                Command::new(executable)
                    .arg("--f52-unowned-descendant")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("spawn controlled F52 unowned descendant"),
            );
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
        mode => {
            eprintln!("unexpected F52 fixture mode: {mode:?}");
            process::exit(24);
        }
    }
}
"##;

const COPILOT_FIXTURE: &str = r#"
$path = [IO.Path]::GetFullPath($MyInvocation.MyCommand.Path)
$arguments = @($args) -join "`n"
[IO.File]::WriteAllText($env:F52_COPILOT_MARKER, "$path`n$arguments")
"#;

const PROCESS_IS_OWNED_FIXTURE: &str = r#"
$process = Get-CimInstance Win32_Process `
  -Filter "ProcessId = $env:F52_FIXTURE_PID" `
  -ErrorAction SilentlyContinue
if (
  $null -ne $process -and
  $process.ExecutablePath -and
  [string]::Equals(
    [IO.Path]::GetFullPath($process.ExecutablePath),
    [IO.Path]::GetFullPath($env:F52_FIXTURE_EXE),
    [StringComparison]::OrdinalIgnoreCase
  )
) {
  exit 0
}
exit 1
"#;

const STOP_OWNED_FIXTURE: &str = r#"
$process = Get-CimInstance Win32_Process `
  -Filter "ProcessId = $env:F52_FIXTURE_PID" `
  -ErrorAction SilentlyContinue
if (
  $null -ne $process -and
  $process.ExecutablePath -and
  [string]::Equals(
    [IO.Path]::GetFullPath($process.ExecutablePath),
    [IO.Path]::GetFullPath($env:F52_FIXTURE_EXE),
    [StringComparison]::OrdinalIgnoreCase
  )
) {
  Stop-Process -Id ([int]$env:F52_FIXTURE_PID) -Force -ErrorAction SilentlyContinue
}
"#;

#[derive(Debug)]
struct FailureObservation {
    copilot_started: bool,
    fixture_invoked: bool,
    failed_stage: Option<PreflightStage>,
    stage_evidence: Option<String>,
    transcript: String,
}

#[derive(Debug)]
struct CleanupObservation {
    owned_child: u32,
    terminated_processes: Vec<u32>,
    unowned_descendant_survived: bool,
    unowned_descendant_released: bool,
    transcript: String,
}

struct LauncherFixture {
    _temporary_directory: TempDir,
    workspace: PathBuf,
    launcher: PathBuf,
    engram_executable: PathBuf,
    copilot_script: PathBuf,
    copilot_marker: PathBuf,
    invocation_marker: PathBuf,
    result_marker: PathBuf,
    launcher_stdout: PathBuf,
    launcher_stderr: PathBuf,
    owner_pid_marker: PathBuf,
    descendant_pid_marker: PathBuf,
    descendant_release_marker: PathBuf,
    descendant_survived_marker: PathBuf,
    powershell: PathBuf,
}

impl LauncherFixture {
    fn new() -> Self {
        let temporary_directory = tempfile::Builder::new()
            .prefix("f52-start-launcher-")
            .tempdir_in(env!("CARGO_MANIFEST_DIR"))
            .expect("create task-local F52 launcher fixture directory");
        let workspace = temporary_directory
            .path()
            .join("launcher failure fixture workspace with spaces");
        let engram_directory = workspace.join("target").join("debug");
        let copilot_directory = workspace.join("Program Files").join("GitHub Copilot");
        fs::create_dir_all(&engram_directory).expect("create controlled engram directory");
        fs::create_dir_all(&copilot_directory).expect("create spaced Copilot fixture directory");

        let launcher = workspace.join("start.ps1");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("start.ps1"),
            &launcher,
        )
        .expect("copy start.ps1 into the isolated workspace");

        let copilot_script = copilot_directory.join("copilot.ps1");
        fs::write(&copilot_script, COPILOT_FIXTURE).expect("write controlled Copilot fixture");
        let engram_executable = engram_directory.join("engram.exe");
        compile_engram_fixture(&workspace, &engram_executable);

        Self {
            copilot_marker: workspace.join("Copilot invocation marker.txt"),
            invocation_marker: workspace.join("engram invocation marker.txt"),
            result_marker: workspace.join("typed stage result marker.json"),
            launcher_stdout: workspace.join("launcher stdout.txt"),
            launcher_stderr: workspace.join("launcher stderr.txt"),
            owner_pid_marker: workspace.join("owned child pid marker.txt"),
            descendant_pid_marker: workspace.join("unowned descendant pid marker.txt"),
            descendant_release_marker: workspace.join("release descendant marker.txt"),
            descendant_survived_marker: workspace.join("descendant survived marker.txt"),
            _temporary_directory: temporary_directory,
            workspace,
            launcher,
            engram_executable,
            copilot_script,
            powershell: find_executable("pwsh"),
        }
    }

    fn run(&self, mode: &str, stage: Option<PreflightStage>, arguments: &[&str]) -> Output {
        let stdout =
            fs::File::create(&self.launcher_stdout).expect("create F52 launcher stdout capture");
        let stderr =
            fs::File::create(&self.launcher_stderr).expect("create F52 launcher stderr capture");
        for marker in [
            &self.copilot_marker,
            &self.invocation_marker,
            &self.result_marker,
            &self.owner_pid_marker,
            &self.descendant_pid_marker,
            &self.descendant_release_marker,
            &self.descendant_survived_marker,
        ] {
            remove_marker(marker);
        }

        let status = Command::new(&self.powershell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(&self.launcher)
            .args(arguments)
            .current_dir(&self.workspace)
            .env("PATH", self.controlled_path())
            .env("GITHUB_TOKEN", "f52-launcher-contract-fixture")
            .env("COPILOT_HOME", self.workspace.join(".copilot"))
            .env("ENGRAM_DATA_DIR", self.workspace.join(".engram"))
            .env("COPILOT_EXE_PATH", &self.copilot_script)
            .env("F52_COPILOT_MARKER", &self.copilot_marker)
            .env("F52_FIXTURE_MODE", mode)
            .env(
                "F52_FIXTURE_STAGE",
                stage.map_or("", PreflightStage::as_str),
            )
            .env("F52_FIXTURE_INVOCATION", &self.invocation_marker)
            .env("F52_FIXTURE_RESULT", &self.result_marker)
            .env("F52_FIXTURE_OWNER_PID", &self.owner_pid_marker)
            .env("F52_FIXTURE_DESCENDANT_PID", &self.descendant_pid_marker)
            .env(
                "F52_FIXTURE_RELEASE_DESCENDANT",
                &self.descendant_release_marker,
            )
            .env(
                "F52_FIXTURE_DESCENDANT_SURVIVED",
                &self.descendant_survived_marker,
            )
            .env("ENGRAM_PREWARM_TIMEOUT_MS", "1250")
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .status()
            .expect("run the copied PowerShell launcher against F52 fixtures");
        Output {
            status,
            stdout: fs::read(&self.launcher_stdout).expect("read F52 launcher stdout capture"),
            stderr: fs::read(&self.launcher_stderr).expect("read F52 launcher stderr capture"),
        }
    }

    fn controlled_path(&self) -> OsString {
        let mut paths = vec![
            self.engram_executable
                .parent()
                .expect("controlled engram executable has a parent")
                .to_path_buf(),
        ];
        if let Some(powershell_directory) = self.powershell.parent() {
            paths.push(powershell_directory.to_path_buf());
        }
        env::join_paths(paths).expect("compose isolated F52 fixture PATH")
    }

    fn fixture_pid(marker: &Path) -> u32 {
        assert!(
            wait_for_file(marker, Duration::from_secs(3)),
            "RED F52-FIXTURE-PROCESS: expected a test-created process marker at {}",
            marker.display()
        );
        fs::read_to_string(marker)
            .expect("read F52 test-created process id")
            .trim()
            .parse()
            .expect("parse F52 test-created process id")
    }

    fn process_is_running(&self, process_id: u32, executable: &Path) -> bool {
        Command::new(&self.powershell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                PROCESS_IS_OWNED_FIXTURE,
            ])
            .env("F52_FIXTURE_PID", process_id.to_string())
            .env("F52_FIXTURE_EXE", executable)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn stop_test_created_process(&self, process_id: u32, executable: &Path) {
        let _ = Command::new(&self.powershell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                STOP_OWNED_FIXTURE,
            ])
            .env("F52_FIXTURE_PID", process_id.to_string())
            .env("F52_FIXTURE_EXE", executable)
            .status();
    }
}

impl Drop for LauncherFixture {
    fn drop(&mut self) {
        // Give the unowned test descendant a normal exit first. If it remains,
        // stop only marker-recorded fixture PIDs after verifying their image.
        let _ = fs::write(&self.descendant_release_marker, "release");
        let _ = wait_for_file(&self.descendant_survived_marker, Duration::from_secs(2));
        for marker in [&self.owner_pid_marker, &self.descendant_pid_marker] {
            if let Ok(contents) = fs::read_to_string(marker) {
                if let Ok(process_id) = contents.trim().parse() {
                    self.stop_test_created_process(process_id, &self.engram_executable);
                }
            }
        }
    }
}

fn run_launcher_after_failure(
    fixture: &LauncherFixture,
    stage: PreflightStage,
) -> FailureObservation {
    let output = fixture.run("failed", Some(stage), &[]);
    let transcript = launcher_output(&output);
    let stage_evidence = fs::read_to_string(&fixture.result_marker).ok();

    FailureObservation {
        copilot_started: fixture.copilot_marker.is_file(),
        fixture_invoked: fixture.invocation_marker.is_file(),
        failed_stage: reported_stage(&transcript),
        stage_evidence,
        transcript,
    }
}

fn cleanup_launcher_child(fixture: &LauncherFixture) -> CleanupObservation {
    let output = fixture.run("cleanup", None, &[]);
    let owned_child = LauncherFixture::fixture_pid(&fixture.owner_pid_marker);
    let unowned_descendant = LauncherFixture::fixture_pid(&fixture.descendant_pid_marker);
    let owned_child_stopped = !fixture.process_is_running(owned_child, &fixture.engram_executable);
    let unowned_descendant_survived =
        fixture.process_is_running(unowned_descendant, &fixture.engram_executable);

    fs::write(&fixture.descendant_release_marker, "release")
        .expect("release test-created unowned descendant");
    let unowned_descendant_released =
        wait_for_file(&fixture.descendant_survived_marker, Duration::from_secs(3));

    CleanupObservation {
        owned_child,
        terminated_processes: if owned_child_stopped {
            vec![owned_child]
        } else {
            Vec::new()
        },
        unowned_descendant_survived,
        unowned_descendant_released,
        transcript: launcher_output(&output),
    }
}

fn reported_stage(transcript: &str) -> Option<PreflightStage> {
    PREFLIGHT_STAGES.into_iter().find(|stage| {
        transcript.contains(&format!(
            r#"{{"state":"Failed","stage":"{}"}}"#,
            stage.as_str()
        ))
    })
}

fn compile_engram_fixture(workspace: &Path, executable: &Path) {
    let source = workspace.join("controlled engram fixture.rs");
    fs::write(&source, ENGRAM_FIXTURE_SOURCE).expect("write controlled engram fixture source");
    let output = Command::new(find_executable("rustc"))
        .args([
            "--edition=2024",
            "--crate-type",
            "bin",
            "--crate-name",
            "f52_engram_fixture",
        ])
        .arg(source)
        .arg("-o")
        .arg(executable)
        .output()
        .expect("compile controlled engram executable fixture");
    assert!(
        output.status.success(),
        "compile controlled engram fixture: stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn find_executable(name: &str) -> PathBuf {
    let executable_name = format!("{name}.exe");
    let path = env::var_os("PATH").unwrap_or_default();
    env::split_paths(&path)
        .map(|directory| directory.join(&executable_name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("required test tool {executable_name} was not found on PATH"))
}

fn normalized_path(path: &Path) -> String {
    path.canonicalize()
        .expect("canonicalize F52 fixture executable path")
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn launcher_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn remove_marker(path: &Path) {
    if path.exists() {
        fs::remove_file(path).expect("remove stale F52 fixture marker");
    }
}

fn wait_for_file(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.is_file() {
            return true;
        }
        thread::sleep(Duration::from_millis(25));
    }
    path.is_file()
}

#[test]
fn each_preflight_stage_failure_prevents_copilot_launch() {
    let fixture = LauncherFixture::new();
    assert!(
        fixture.copilot_script.to_string_lossy().contains(' '),
        "F52 failure fixture must configure a Copilot executable path with spaces"
    );

    let observations = PREFLIGHT_STAGES
        .into_iter()
        .map(|stage| (stage, run_launcher_after_failure(&fixture, stage)))
        .collect::<Vec<_>>();

    let fixture_misses = observations
        .iter()
        .filter(|(_, observation)| !observation.fixture_invoked)
        .map(|(stage, observation)| format!("{stage:?}: {}", observation.transcript))
        .collect::<Vec<_>>();
    assert!(
        fixture_misses.is_empty(),
        "RED F52-STAGE-INJECTION: controlled preflight fixtures were not invoked: \
         {fixture_misses:?}"
    );

    let evidence_mismatches = observations
        .iter()
        .filter_map(|(stage, observation)| {
            let expected = format!(r#"{{"state":"Failed","stage":"{}"}}"#, stage.as_str());
            (observation.stage_evidence.as_deref() != Some(expected.as_str()))
                .then(|| format!("{stage:?}: {:?}", observation.stage_evidence))
        })
        .collect::<Vec<_>>();
    assert!(
        evidence_mismatches.is_empty(),
        "RED F52-STAGE-EVIDENCE: controlled fixture did not retain exact stage failures: \
         {evidence_mismatches:?}"
    );

    let copilot_started_stages = observations
        .iter()
        .filter(|(_, observation)| observation.copilot_started)
        .map(|(stage, _)| *stage)
        .collect::<Vec<_>>();
    assert!(
        copilot_started_stages.is_empty(),
        "RED F52-FAIL-CLOSED: unchanged start.ps1 launched Copilot after preflight failures at \
         {copilot_started_stages:?}"
    );

    let typed_stage_mismatches = observations
        .iter()
        .filter(|(stage, observation)| observation.failed_stage != Some(*stage))
        .map(|(stage, observation)| {
            format!(
                "{stage:?}: observed {:?}; {}",
                observation.failed_stage, observation.transcript
            )
        })
        .collect::<Vec<_>>();
    assert!(
        typed_stage_mismatches.is_empty(),
        "RED F52-TYPED-STAGE: launcher did not report exact failing stages: \
         {typed_stage_mismatches:?}"
    );
}

#[test]
fn launch_path_with_spaces_is_preserved() {
    let fixture = LauncherFixture::new();
    assert!(
        fixture.workspace.to_string_lossy().contains(' ')
            && fixture.copilot_script.to_string_lossy().contains(' '),
        "F52 launcher workspace and Copilot executable must contain spaces"
    );

    let output = fixture.run(
        "succeeded",
        None,
        &["--session", "session name with spaces"],
    );
    let transcript = launcher_output(&output);
    assert!(
        output.status.success(),
        "RED F52-SPACED-LAUNCH: copied launcher failed with the spaced Copilot path; {transcript}"
    );
    assert!(
        fixture.copilot_marker.is_file(),
        "RED F52-SPACED-LAUNCH: Copilot fixture was not launched; {transcript}"
    );

    let invocation =
        fs::read_to_string(&fixture.copilot_marker).expect("read spaced Copilot invocation");
    let mut lines = invocation.lines();
    let actual_executable = lines.next().unwrap_or_default();
    assert_eq!(
        normalized_path(Path::new(actual_executable)),
        normalized_path(&fixture.copilot_script),
        "RED F52-SPACED-EXECUTABLE: Copilot executable path was not preserved; {invocation}"
    );
    assert!(
        lines.any(|argument| argument == "session name with spaces"),
        "RED F52-SPACED-ARGUMENT: launcher split the argument containing spaces; {invocation}"
    );
}

#[test]
fn cleanup_with_a_spaced_path_never_terminates_an_unowned_descendant() {
    let fixture = LauncherFixture::new();
    assert!(
        fixture.engram_executable.to_string_lossy().contains(' '),
        "F52 cleanup fixture executable path must contain spaces"
    );

    let observation = cleanup_launcher_child(&fixture);

    assert_eq!(
        observation.terminated_processes,
        [observation.owned_child],
        "RED F52-EXACT-CHILD-CLEANUP: launcher must stop only its marker-recorded child; {}",
        observation.transcript
    );
    assert!(
        observation.unowned_descendant_survived,
        "RED F52-UNOWNED-DESCENDANT: launcher terminated a marker-recorded descendant \
         that it did not own; {}",
        observation.transcript
    );
    assert!(
        observation.unowned_descendant_released,
        "RED F52-UNOWNED-DESCENDANT: controlled descendant did not exit after release"
    );
}
