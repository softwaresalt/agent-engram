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

const ENGRAM_FIXTURE_SOURCE: &str = r#"
use std::{
    env, fs,
    path::PathBuf,
    process::{self, Command, Stdio},
    thread,
    time::Duration,
};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env::var_os(name).unwrap_or_else(|| panic!("missing fixture variable {name}")))
}

fn write_fixture_file(name: &str, contents: impl AsRef<[u8]>) {
    fs::write(fixture_path(name), contents).expect("write fixture marker");
}

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.iter().any(|argument| argument == "--fixture-descendant") {
        write_fixture_file(
            "ENGRAM_FIXTURE_DESCENDANT_PID",
            process::id().to_string(),
        );
        loop {
            if fixture_path("ENGRAM_FIXTURE_RELEASE_DESCENDANT").is_file() {
                write_fixture_file("ENGRAM_FIXTURE_DESCENDANT_SURVIVED", "survived");
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
    }

    let executable = env::current_exe().expect("resolve fixture executable");
    let invocation = format!(
        "{}\n{}\n",
        executable.display(),
        arguments.join("\n")
    );
    write_fixture_file("ENGRAM_FIXTURE_INVOCATION", invocation);
    write_fixture_file(
        "ENGRAM_FIXTURE_OWNER_PID",
        process::id().to_string(),
    );

    match env::var("ENGRAM_FIXTURE_MODE").as_deref() {
        Ok("succeeded") => {
            write_fixture_file("ENGRAM_FIXTURE_RESULT", "Succeeded");
            println!("Succeeded");
        }
        Ok("failed") => {
            write_fixture_file("ENGRAM_FIXTURE_RESULT", "Failed:HealthVerified");
            println!("Failed at HealthVerified");
            process::exit(23);
        }
        Ok("cleanup") => {
            drop(
                Command::new(executable)
                    .arg("--fixture-descendant")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("spawn controlled unowned descendant"),
            );

            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
        mode => {
            eprintln!("unexpected fixture mode: {mode:?}");
            process::exit(24);
        }
    }
}
"#;

const PATH_ENGRAM_FIXTURE: &str = r#"
$mode = $env:ENGRAM_FIXTURE_MODE
[IO.File]::WriteAllText($env:ENGRAM_PATH_FALLBACK_MARKER, "PATH fallback was invoked")
if ($mode -eq "failed") {
  [IO.File]::WriteAllText($env:ENGRAM_FIXTURE_RESULT, "Failed:HealthVerified")
  Write-Output '{"state":"Failed","stage":"HealthVerified"}'
  exit 23
}
if ($mode -eq "succeeded") {
  [IO.File]::WriteAllText($env:ENGRAM_FIXTURE_RESULT, "Succeeded")
  Write-Output '{"state":"Succeeded"}'
  exit 0
}
if ($mode -eq "cleanup") {
  [IO.File]::WriteAllText($env:ENGRAM_FIXTURE_OWNER_PID, $PID.ToString())
  Start-Process `
    -FilePath $env:ENGRAM_FIXTURE_DESCENDANT_EXE `
    -ArgumentList "--fixture-descendant" `
    -RedirectStandardOutput $env:ENGRAM_FIXTURE_DESCENDANT_STDOUT `
    -RedirectStandardError $env:ENGRAM_FIXTURE_DESCENDANT_STDERR

  while ($true) {
    Start-Sleep -Seconds 1
  }
}
exit 24
"#;

const COPILOT_FIXTURE: &str = r#"
$state = if (Test-Path -LiteralPath $env:ENGRAM_FIXTURE_RESULT) {
  Get-Content -LiteralPath $env:ENGRAM_FIXTURE_RESULT -Raw
} else {
  "<missing preflight result>"
}
$forwardedArguments = @($args) -join "`n"
[IO.File]::WriteAllText(
  $env:COPILOT_MARKER,
  "$state`n$forwardedArguments"
)
"#;

const PROCESS_IS_OWNED_FIXTURE: &str = r#"
$process = Get-CimInstance Win32_Process `
  -Filter "ProcessId = $env:F51_FIXTURE_PID" `
  -ErrorAction SilentlyContinue
if (
  $null -ne $process -and
  $process.ExecutablePath -and
  [string]::Equals(
    [IO.Path]::GetFullPath($process.ExecutablePath),
    [IO.Path]::GetFullPath($env:F51_FIXTURE_EXE),
    [StringComparison]::OrdinalIgnoreCase
  )
) {
  exit 0
}
exit 1
"#;

const STOP_OWNED_FIXTURE: &str = r#"
$process = Get-CimInstance Win32_Process `
  -Filter "ProcessId = $env:F51_FIXTURE_PID" `
  -ErrorAction SilentlyContinue
if (
  $null -ne $process -and
  $process.ExecutablePath -and
  [string]::Equals(
    [IO.Path]::GetFullPath($process.ExecutablePath),
    [IO.Path]::GetFullPath($env:F51_FIXTURE_EXE),
    [StringComparison]::OrdinalIgnoreCase
  )
) {
  Stop-Process -Id ([int]$env:F51_FIXTURE_PID) -Force -ErrorAction SilentlyContinue
}
"#;

struct LauncherFixture {
    _temporary_directory: TempDir,
    workspace: PathBuf,
    path_shim_directory: PathBuf,
    route: EngramRoute,
    launcher: PathBuf,
    engram_executable: PathBuf,
    copilot_script: PathBuf,
    copilot_marker: PathBuf,
    fallback_marker: PathBuf,
    invocation_marker: PathBuf,
    owner_pid_marker: PathBuf,
    descendant_pid_marker: PathBuf,
    descendant_release_marker: PathBuf,
    descendant_survived_marker: PathBuf,
    descendant_stdout: PathBuf,
    descendant_stderr: PathBuf,
    launcher_stdout: PathBuf,
    launcher_stderr: PathBuf,
    result_marker: PathBuf,
    powershell: PathBuf,
    mode: &'static str,
}

#[derive(Clone, Copy, Debug)]
enum EngramRoute {
    PathShim,
    WorkspaceExecutable,
}

impl LauncherFixture {
    fn new(mode: &'static str, install_engram: bool) -> Self {
        Self::new_with_route(mode, install_engram, EngramRoute::PathShim)
    }

    fn new_with_route(mode: &'static str, install_engram: bool, route: EngramRoute) -> Self {
        let temporary_directory = tempfile::Builder::new()
            .prefix("f51-start-launcher-")
            .tempdir_in(env!("CARGO_MANIFEST_DIR"))
            .expect("create task-specific launcher fixture");
        let workspace = temporary_directory
            .path()
            .join("launcher fixture workspace with spaces");
        let path_shim_directory = temporary_directory.path().join("path-shim");
        let engram_directory = workspace.join("target").join("debug");
        let copilot_directory = workspace.join("GitHub Copilot");
        fs::create_dir_all(&path_shim_directory).expect("create controlled PATH shim");
        fs::create_dir_all(&engram_directory).expect("create workspace target directory");
        fs::create_dir_all(&copilot_directory).expect("create spaced Copilot fixture directory");

        let launcher = workspace.join("start.ps1");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("start.ps1"),
            &launcher,
        )
        .expect("copy start.ps1 into the isolated workspace");

        let copilot_script = copilot_directory.join("copilot.ps1");
        fs::write(&copilot_script, COPILOT_FIXTURE).expect("write Copilot process fixture");
        fs::write(path_shim_directory.join("engram.ps1"), PATH_ENGRAM_FIXTURE)
            .expect("write controlled PATH-only engram fallback");

        let engram_executable = engram_directory.join("engram.exe");
        if install_engram {
            compile_engram_fixture(&workspace, &engram_executable);
        }

        let powershell = find_executable("pwsh");
        Self {
            copilot_marker: workspace.join("copilot invocation.txt"),
            fallback_marker: workspace.join("PATH fallback invocation.txt"),
            invocation_marker: workspace.join("workspace engram invocation.txt"),
            owner_pid_marker: workspace.join("owned child pid.txt"),
            descendant_pid_marker: workspace.join("unowned descendant pid.txt"),
            descendant_release_marker: workspace.join("release descendant.txt"),
            descendant_survived_marker: workspace.join("descendant survived.txt"),
            descendant_stdout: workspace.join("descendant stdout.txt"),
            descendant_stderr: workspace.join("descendant stderr.txt"),
            launcher_stdout: workspace.join("launcher stdout.txt"),
            launcher_stderr: workspace.join("launcher stderr.txt"),
            result_marker: workspace.join("preflight result.txt"),
            _temporary_directory: temporary_directory,
            workspace,
            path_shim_directory,
            route,
            launcher,
            engram_executable,
            copilot_script,
            powershell,
            mode,
        }
    }

    fn run(&self, arguments: &[&str]) -> Output {
        let stdout =
            fs::File::create(&self.launcher_stdout).expect("create copied launcher stdout capture");
        let stderr =
            fs::File::create(&self.launcher_stderr).expect("create copied launcher stderr capture");
        let mut process = Command::new(&self.powershell)
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
            .env("GITHUB_TOKEN", "launcher-contract-fixture")
            .env("COPILOT_EXE_PATH", &self.copilot_script)
            .env("COPILOT_MARKER", &self.copilot_marker)
            .env("ENGRAM_FIXTURE_MODE", self.mode)
            .env("ENGRAM_FIXTURE_RESULT", &self.result_marker)
            .env("ENGRAM_FIXTURE_INVOCATION", &self.invocation_marker)
            .env("ENGRAM_FIXTURE_OWNER_PID", &self.owner_pid_marker)
            .env("ENGRAM_FIXTURE_DESCENDANT_PID", &self.descendant_pid_marker)
            .env("ENGRAM_FIXTURE_DESCENDANT_EXE", &self.engram_executable)
            .env("ENGRAM_FIXTURE_DESCENDANT_STDOUT", &self.descendant_stdout)
            .env("ENGRAM_FIXTURE_DESCENDANT_STDERR", &self.descendant_stderr)
            .env(
                "ENGRAM_FIXTURE_RELEASE_DESCENDANT",
                &self.descendant_release_marker,
            )
            .env(
                "ENGRAM_FIXTURE_DESCENDANT_SURVIVED",
                &self.descendant_survived_marker,
            )
            .env("ENGRAM_PATH_FALLBACK_MARKER", &self.fallback_marker)
            .env(
                "ENGRAM_PREWARM_TIMEOUT_MS",
                if self.mode == "cleanup" {
                    "2500"
                } else {
                    "750"
                },
            )
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .expect("start copied PowerShell launcher");

        let deadline = Instant::now() + Duration::from_secs(22);
        loop {
            if process
                .try_wait()
                .expect("poll copied launcher process")
                .is_some()
            {
                break;
            }
            if Instant::now() >= deadline {
                let _ = process.kill();
                let _ = process.wait();
                panic!("RED F51-LAUNCHER-TIMEOUT: copied start.ps1 did not finish");
            }
            thread::sleep(Duration::from_millis(40));
        }

        let status = process.wait().expect("collect copied launcher exit status");
        Output {
            status,
            stdout: fs::read(&self.launcher_stdout).expect("read copied launcher stdout capture"),
            stderr: fs::read(&self.launcher_stderr).expect("read copied launcher stderr capture"),
        }
    }

    fn controlled_path(&self) -> OsString {
        let mut paths = match self.route {
            EngramRoute::PathShim => vec![self.path_shim_directory.clone()],
            EngramRoute::WorkspaceExecutable => vec![
                self.engram_executable
                    .parent()
                    .expect("workspace engram executable has a parent directory")
                    .to_path_buf(),
            ],
        };
        if let Some(powershell_directory) = self.powershell.parent() {
            paths.push(powershell_directory.to_path_buf());
        }
        env::join_paths(paths).expect("compose isolated launcher PATH")
    }

    fn owner_executable(&self) -> &Path {
        match self.route {
            EngramRoute::PathShim => &self.powershell,
            EngramRoute::WorkspaceExecutable => &self.engram_executable,
        }
    }

    fn assert_workspace_engram_was_invoked(&self) {
        assert!(
            self.invocation_marker.is_file(),
            "RED F51-WORKSPACE-ENGRAM: launcher did not invoke workspace target\\debug\\engram.exe"
        );
        let invocation =
            fs::read_to_string(&self.invocation_marker).expect("read engram invocation marker");
        let actual_executable = invocation
            .lines()
            .next()
            .map(Path::new)
            .expect("fixture invocation must record its executable");
        assert_eq!(
            normalized_path(actual_executable),
            normalized_path(&self.engram_executable),
            "RED F51-EXACT-ENGRAM-PATH: launcher used a binary other than workspace target\\debug\\engram.exe"
        );
        assert!(
            !self.fallback_marker.exists(),
            "RED F51-NO-PATH-FALLBACK: launcher used the controlled PATH engram shim"
        );
    }

    fn fixture_pid(marker: &Path) -> u32 {
        assert!(
            wait_for_file(marker, Duration::from_secs(3)),
            "RED F51-CHILD-IDENTITY: expected test-created process marker at {}",
            marker.display()
        );
        fs::read_to_string(marker)
            .expect("read test-created process id")
            .trim()
            .parse()
            .expect("parse test-created process id")
    }

    fn process_is_running(&self, process_id: u32, executable: &Path) -> bool {
        Command::new(&self.powershell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                PROCESS_IS_OWNED_FIXTURE,
            ])
            .env("F51_FIXTURE_PID", process_id.to_string())
            .env("F51_FIXTURE_EXE", executable)
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
            .env("F51_FIXTURE_PID", process_id.to_string())
            .env("F51_FIXTURE_EXE", executable)
            .status();
    }
}

impl Drop for LauncherFixture {
    fn drop(&mut self) {
        // Stop only marker-recorded fixture processes, and verify each exact
        // image path first; never use a recursive tree-kill.
        for (marker, executable) in [
            (&self.owner_pid_marker, self.owner_executable()),
            (
                &self.descendant_pid_marker,
                self.engram_executable.as_path(),
            ),
        ] {
            if let Ok(contents) = fs::read_to_string(marker) {
                if let Ok(process_id) = contents.trim().parse() {
                    self.stop_test_created_process(process_id, executable);
                }
            }
        }
    }
}

fn compile_engram_fixture(workspace: &Path, executable: &Path) {
    let source = workspace.join("controlled engram fixture.rs");
    fs::write(&source, ENGRAM_FIXTURE_SOURCE).expect("write engram process fixture source");
    let output = Command::new(find_executable("rustc"))
        .args([
            "--edition=2024",
            "--crate-type",
            "bin",
            "--crate-name",
            "f51_engram_fixture",
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
        .expect("canonicalize fixture executable path")
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
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

fn launcher_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn copilot_starts_only_after_succeeded_preflight_with_spaced_paths() {
    let fixture = LauncherFixture::new("succeeded", true);
    assert!(
        fixture.workspace.to_string_lossy().contains(' ')
            && fixture.copilot_script.to_string_lossy().contains(' '),
        "the launcher and Copilot fixture paths must contain spaces"
    );

    let output = fixture.run(&["--session", "session name with spaces"]);

    assert!(
        output.status.success(),
        "RED F51-SUCCEEDED-PREFLIGHT: launcher should complete after a Succeeded preflight; {}",
        launcher_output(&output)
    );
    fixture.assert_workspace_engram_was_invoked();
    assert!(
        fixture.copilot_marker.is_file(),
        "RED F51-COPILOT-AFTER-SUCCEEDED: Copilot fixture was not launched"
    );
    let copilot_invocation =
        fs::read_to_string(&fixture.copilot_marker).expect("read Copilot invocation marker");
    assert!(
        copilot_invocation.starts_with("Succeeded"),
        "RED F51-COPILOT-AFTER-SUCCEEDED: Copilot did not observe the successful preflight result: {copilot_invocation}"
    );
    assert!(
        copilot_invocation
            .lines()
            .any(|argument| argument == "session name with spaces"),
        "RED F51-SPACED-ARGUMENT: launcher split the caller argument containing spaces: {copilot_invocation}"
    );
}

#[test]
fn typed_preflight_failure_is_reported_without_starting_copilot() {
    let fixture = LauncherFixture::new("failed", true);

    let output = fixture.run(&[]);
    let transcript = launcher_output(&output);

    assert!(
        !output.status.success(),
        "RED F51-FAIL-CLOSED: failed preflight must return a failure status; {transcript}"
    );
    fixture.assert_workspace_engram_was_invoked();
    assert!(
        transcript.contains("Failed") && transcript.contains("HealthVerified"),
        "RED F51-TYPED-STAGE: launcher must report the typed HealthVerified failure; {transcript}"
    );
    assert!(
        !fixture.copilot_marker.exists(),
        "RED F51-NO-COPILOT-ON-FAILURE: Copilot ran after the HealthVerified preflight failure"
    );
}

#[test]
fn cleanup_terminates_only_its_child_and_preserves_an_unowned_descendant() {
    for route in [EngramRoute::PathShim, EngramRoute::WorkspaceExecutable] {
        let fixture = LauncherFixture::new_with_route("cleanup", true, route);
        let output = fixture.run(&[]);
        let owned_child = LauncherFixture::fixture_pid(&fixture.owner_pid_marker);
        let unowned_descendant = LauncherFixture::fixture_pid(&fixture.descendant_pid_marker);

        assert!(
            !fixture.process_is_running(owned_child, fixture.owner_executable()),
            "RED F51-EXACT-CHILD-CLEANUP: launcher left its {:?} child running; {}",
            route,
            launcher_output(&output)
        );
        assert!(
            fixture.process_is_running(unowned_descendant, &fixture.engram_executable),
            "RED F51-UNOWNED-DESCENDANT-SURVIVES: launcher terminated a descendant on the {route:?} route"
        );

        fs::write(&fixture.descendant_release_marker, "release")
            .expect("release test-created unowned descendant");
        assert!(
            wait_for_file(&fixture.descendant_survived_marker, Duration::from_secs(3)),
            "RED F51-UNOWNED-DESCENDANT-SURVIVES: test-created descendant on the {route:?} route did not finish after launcher cleanup"
        );
        if matches!(route, EngramRoute::PathShim) {
            assert!(
                fixture.fallback_marker.is_file(),
                "RED F51-PATH-SHIM-CLEANUP: cleanup scenario did not run the PATH shim"
            );
        }
    }
}

#[test]
fn missing_workspace_binary_fails_visibly_without_using_path_fallback() {
    let fixture = LauncherFixture::new("succeeded", false);
    assert!(
        !fixture.engram_executable.exists(),
        "missing-binary fixture must not create target\\debug\\engram.exe"
    );
    let launcher_source =
        fs::read_to_string(&fixture.launcher).expect("read copied launcher for fallback guard");
    let normalized_source = launcher_source.to_ascii_lowercase().replace('/', "\\");
    assert!(
        !normalized_source.contains(r"c:\tools"),
        "RED F51-NO-TOOLS-FALLBACK: launcher must not select a C:\\Tools fallback"
    );

    let output = fixture.run(&[]);
    let transcript = launcher_output(&output);

    assert!(
        !output.status.success(),
        "RED F51-MISSING-WORKSPACE-BINARY: launcher must fail when target\\debug\\engram.exe is absent; {transcript}"
    );
    assert!(
        transcript
            .to_ascii_lowercase()
            .contains(r"target\debug\engram.exe"),
        "RED F51-MISSING-BINARY-DIAGNOSTIC: missing exact workspace binary must be reported visibly; {transcript}"
    );
    assert!(
        !fixture.fallback_marker.exists(),
        "RED F51-NO-PATH-FALLBACK: launcher ran the PATH-only engram fixture"
    );
    assert!(
        !fixture.copilot_marker.exists(),
        "RED F51-NO-COPILOT-WITHOUT-BINARY: Copilot ran without the required workspace engram binary"
    );
}
