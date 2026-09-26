//! Behavior-level contract tests for the F53 Unix launcher.
#![forbid(unsafe_code)]

use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use tempfile::{Builder, TempDir};

const PREFLIGHT_SUCCEEDED: &str = r#"{"state":"Succeeded"}"#;
const PREFLIGHT_FAILED: &str = r#"{"state":"Failed","stage":"McpProbeVerified"}"#;

const PREFLIGHT_FIXTURE: &str = r#"#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--f53-unowned-descendant" ]]; then
    printf '%s\n' "$$" > "$F53_DESCENDANT_PID_MARKER"
    while [[ ! -f "$F53_RELEASE_DESCENDANT_MARKER" ]]; do
        sleep 0.025
    done
    printf '%s\n' "descendant-released" >> "$F53_EVENTS"
    printf '%s\n' "released" > "$F53_DESCENDANT_EXITED_MARKER"
    exit 0
fi

case "$F53_PREFLIGHT_MODE" in
    succeeded)
        result='{"state":"Succeeded"}'
        printf '%s' "$result" > "$F53_PREFLIGHT_RESULT"
        printf '%s\n' "preflight:succeeded" >> "$F53_EVENTS"
        printf '%s\n' "$result"
        ;;
    failed)
        result='{"state":"Failed","stage":"McpProbeVerified"}'
        printf '%s' "$result" > "$F53_PREFLIGHT_RESULT"
        printf '%s\n' "preflight:failed" >> "$F53_EVENTS"
        printf '%s\n' "$result"
        exit 23
        ;;
    cleanup)
        printf '%s\n' "$$" > "$F53_OWNER_PID_MARKER"
        printf '%s\n' "preflight:cleanup" >> "$F53_EVENTS"
        "$0" --f53-unowned-descendant >/dev/null 2>&1 &
        while [[ ! -f "$F53_RELEASE_OWNER_MARKER" ]]; do
            sleep 0.025
        done
        ;;
    *)
        printf 'unexpected F53 fixture mode: %s\n' "$F53_PREFLIGHT_MODE" >&2
        exit 24
        ;;
esac
"#;

const COPILOT_FIXTURE: &str = r#"#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "copilot" >> "$F53_EVENTS"
printf '%s\0' "$0" "$@" > "$F53_COPILOT_ARGUMENTS"
"#;

const PROCESS_PATH_CHECK: &str = r#"
f53_process_has_fixture_path() {
    local process_id="$1"
    local expected_path="$2"
    local argument
    [[ "$process_id" =~ ^[0-9]+$ ]] || return 1
    [[ -r "/proc/${process_id}/cmdline" ]] || return 1
    while IFS= read -r -d '' argument; do
        if [[ "$argument" == "$expected_path" ]]; then
            return 0
        fi
    done < "/proc/${process_id}/cmdline"
    return 1
}
"#;

struct LauncherFixture {
    _temporary_directory: TempDir,
    bash: PathBuf,
    workspace: PathBuf,
    fixture_bin: PathBuf,
    launcher: PathBuf,
    preflight_executable: PathBuf,
    copilot_executable: PathBuf,
    workspace_argument: PathBuf,
    events: PathBuf,
    preflight_result: PathBuf,
    copilot_arguments: PathBuf,
    owner_pid_marker: PathBuf,
    descendant_pid_marker: PathBuf,
    release_owner_marker: PathBuf,
    release_descendant_marker: PathBuf,
    descendant_exited_marker: PathBuf,
    launcher_pid_marker: PathBuf,
    watchdog_marker: PathBuf,
}

impl LauncherFixture {
    fn new() -> Self {
        let temporary_directory = required(
            Builder::new()
                .prefix(".f53-start-sh-launcher-")
                .tempdir_in(env!("CARGO_MANIFEST_DIR")),
            "create isolated F53 temporary workspace",
        );
        let workspace = temporary_directory
            .path()
            .join("launcher workspace with spaces");
        let fixture_bin = workspace.join("fixture bin with spaces");
        required(
            fs::create_dir_all(&fixture_bin),
            "create isolated fixture executable directory",
        );

        let launcher = workspace.join("start.sh");
        let launcher_source = Path::new(env!("CARGO_MANIFEST_DIR")).join("start.sh");
        required(
            fs::copy(&launcher_source, &launcher).map(|_| ()),
            "copy start.sh into the temporary workspace",
        );

        let preflight_executable = fixture_bin.join("engram");
        required(
            fs::write(&preflight_executable, PREFLIGHT_FIXTURE),
            "write controlled F50 preflight executable",
        );
        let copilot_executable = workspace
            .join("GitHub Copilot")
            .join("bin with spaces")
            .join("copilot");
        if let Some(parent) = copilot_executable.parent() {
            required(
                fs::create_dir_all(parent),
                "create spaced Copilot fixture directory",
            );
        }
        required(
            fs::write(&copilot_executable, COPILOT_FIXTURE),
            "write controlled Copilot executable",
        );
        let workspace_argument = workspace.join("project path with spaces");
        required(
            fs::create_dir_all(&workspace_argument),
            "create spaced workspace argument path",
        );

        let fixture = Self {
            _temporary_directory: temporary_directory,
            bash: PathBuf::from(if cfg!(windows) { "bash.exe" } else { "bash" }),
            events: workspace.join("fixture events.log"),
            preflight_result: workspace.join("typed preflight result.json"),
            copilot_arguments: workspace.join("Copilot arguments.bin"),
            owner_pid_marker: workspace.join("owned child pid.txt"),
            descendant_pid_marker: workspace.join("unowned descendant pid.txt"),
            release_owner_marker: workspace.join("release owned child.txt"),
            release_descendant_marker: workspace.join("release descendant.txt"),
            descendant_exited_marker: workspace.join("descendant exit.txt"),
            launcher_pid_marker: workspace.join("launcher pid.txt"),
            watchdog_marker: workspace.join("launcher watchdog used.txt"),
            workspace,
            fixture_bin,
            launcher,
            preflight_executable,
            copilot_executable,
            workspace_argument,
        };
        fixture.make_fixture_executables();
        fixture.validate_fixtures();
        fixture.clear_markers();
        fixture
    }

    fn make_fixture_executables(&self) {
        let preflight = shell_quote(&required(
            bash_path(&self.preflight_executable),
            "convert preflight fixture path for Bash",
        ));
        let copilot = shell_quote(&required(
            bash_path(&self.copilot_executable),
            "convert Copilot fixture path for Bash",
        ));
        let output = self.run_bash(&format!("chmod +x -- {preflight} {copilot}"));
        assert!(
            output.status.success(),
            "F53_HARNESS_INFRASTRUCTURE: could not mark controlled Bash fixtures executable: {}",
            launcher_output(&output)
        );
    }

    fn validate_fixtures(&self) {
        let preflight_output = self.run_bash(&format!(
            "{}exec engram --f53-fixture-smoke-test",
            self.shell_environment("succeeded")
        ));
        let preflight_result = required(
            Self::read_text(&self.preflight_result),
            "read F53 preflight fixture smoke-test result",
        );
        assert!(
            preflight_output.status.success()
                && preflight_result.as_deref() == Some(PREFLIGHT_SUCCEEDED),
            "F53_HARNESS_INFRASTRUCTURE: controlled preflight fixture did not report success: \
             result={preflight_result:?}; {}",
            launcher_output(&preflight_output)
        );

        self.clear_markers();
        let copilot_path = required(
            bash_path(&self.copilot_executable),
            "convert Copilot fixture path for Bash smoke test",
        );
        let expected_copilot_arguments = vec![
            copilot_path.clone(),
            "fixture smoke argument with spaces".to_owned(),
        ];
        let copilot_output = self.run_bash(&format!(
            "{}exec {} 'fixture smoke argument with spaces'",
            self.shell_environment("succeeded"),
            shell_quote(&copilot_path)
        ));
        let copilot_arguments = required(
            self.read_arguments(),
            "read Copilot fixture smoke-test arguments",
        );
        assert!(
            copilot_output.status.success()
                && copilot_arguments.as_deref() == Some(expected_copilot_arguments.as_slice()),
            "F53_HARNESS_INFRASTRUCTURE: controlled Copilot fixture did not preserve its \
             invocation: observed={copilot_arguments:?}; {}",
            launcher_output(&copilot_output)
        );
    }

    fn shell_environment(&self, mode: &str) -> String {
        let workspace = Self::bash_path_literal(&self.workspace, "convert workspace path");
        let fixture_bin = required(bash_path(&self.fixture_bin), "convert fixture PATH");
        let copilot =
            Self::bash_path_literal(&self.copilot_executable, "convert Copilot executable");
        let events = Self::bash_path_literal(&self.events, "convert event marker path");
        let preflight_result =
            Self::bash_path_literal(&self.preflight_result, "convert preflight result path");
        let copilot_arguments =
            Self::bash_path_literal(&self.copilot_arguments, "convert Copilot marker path");
        let owner_pid =
            Self::bash_path_literal(&self.owner_pid_marker, "convert owned PID marker path");
        let descendant_pid = Self::bash_path_literal(
            &self.descendant_pid_marker,
            "convert descendant PID marker path",
        );
        let release_owner = Self::bash_path_literal(
            &self.release_owner_marker,
            "convert owner release marker path",
        );
        let release_descendant = Self::bash_path_literal(
            &self.release_descendant_marker,
            "convert descendant release marker path",
        );
        let descendant_exited = Self::bash_path_literal(
            &self.descendant_exited_marker,
            "convert descendant exit marker",
        );

        format!(
            "set -euo pipefail\n\
             cd -- {workspace}\n\
             export PATH={}\n\
             export COPILOT_EXE_PATH={copilot}\n\
             export COPILOT_HOME={workspace}/.copilot\n\
             export GITHUB_TOKEN=f53-controlled-test-token\n\
             export ENGRAM_PREWARM_TIMEOUT_MS=1250\n\
             export F53_PREFLIGHT_MODE={}\n\
             export F53_EVENTS={events}\n\
             export F53_PREFLIGHT_RESULT={preflight_result}\n\
             export F53_COPILOT_ARGUMENTS={copilot_arguments}\n\
             export F53_OWNER_PID_MARKER={owner_pid}\n\
             export F53_DESCENDANT_PID_MARKER={descendant_pid}\n\
             export F53_RELEASE_OWNER_MARKER={release_owner}\n\
             export F53_RELEASE_DESCENDANT_MARKER={release_descendant}\n\
             export F53_DESCENDANT_EXITED_MARKER={descendant_exited}\n",
            shell_quote(&format!("{fixture_bin}:/usr/bin:/bin")),
            shell_quote(mode)
        )
    }

    fn run(&self, mode: &str, arguments: &[String]) -> Output {
        self.clear_markers();
        let mut command = self.shell_environment(mode);
        command.push_str("exec bash ");
        command.push_str(&Self::bash_path_literal(
            &self.launcher,
            "convert launcher for run",
        ));
        for argument in arguments {
            command.push(' ');
            command.push_str(&shell_quote(argument));
        }
        self.run_bash(&command)
    }

    fn run_cleanup(&self) -> Output {
        self.clear_markers();
        let mut command = self.shell_environment("cleanup");
        command.push_str("printf '%s\\n' \"$$\" > ");
        command.push_str(&Self::bash_path_literal(
            &self.launcher_pid_marker,
            "convert launcher PID marker",
        ));
        command.push_str("\nexec bash ");
        command.push_str(&Self::bash_path_literal(
            &self.launcher,
            "convert launcher for cleanup",
        ));
        command.push('\n');
        let mut child = required(
            Command::new(&self.bash)
                .arg("-c")
                .arg(&command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn(),
            "start controlled cleanup launcher process",
        );

        if !required(
            wait_for_child(&mut child, Duration::from_secs(5)),
            "wait for launcher cleanup deadline",
        ) {
            required(
                fs::write(
                    &self.watchdog_marker,
                    "launcher exceeded test safety deadline",
                ),
                "record cleanup test safety watchdog",
            );
            required(
                fs::write(&self.release_owner_marker, "test safety release"),
                "release hung test-created preflight child",
            );

            if !required(
                wait_for_child(&mut child, Duration::from_secs(2)),
                "wait for launcher after releasing its fixture child",
            ) {
                if let Some(process_id) = required(
                    Self::read_pid(&self.launcher_pid_marker),
                    "read test launcher PID",
                ) {
                    if required(
                        self.process_has_path(process_id, &self.launcher),
                        "verify launcher fixture path before cleanup",
                    ) {
                        let _ = self.signal_process_if_path(process_id, &self.launcher, "TERM");
                        if !required(
                            wait_for_child(&mut child, Duration::from_secs(1)),
                            "wait for verified launcher termination",
                        ) && required(
                            self.process_has_path(process_id, &self.launcher),
                            "reverify launcher fixture path before forced cleanup",
                        ) {
                            let _ = self.signal_process_if_path(process_id, &self.launcher, "KILL");
                        }
                    }
                }
            }
        }

        required(
            child.wait_with_output(),
            "collect controlled cleanup launcher output",
        )
    }

    fn process_is_fixture(&self, process_id: u32) -> io::Result<bool> {
        self.process_has_path(process_id, &self.preflight_executable)
    }

    fn process_has_path(&self, process_id: u32, path: &Path) -> io::Result<bool> {
        let fixture_path = bash_path(path)?;
        let command = format!(
            "process_id={process_id}\nfixture_path={}\n{PROCESS_PATH_CHECK}\n\
             f53_process_has_fixture_path \"$process_id\" \"$fixture_path\"",
            shell_quote(&fixture_path)
        );
        Ok(self.run_bash_io(&command)?.status.success())
    }

    fn signal_process_if_path(
        &self,
        process_id: u32,
        path: &Path,
        signal: &str,
    ) -> io::Result<bool> {
        let fixture_path = bash_path(path)?;
        let command = format!(
            "process_id={process_id}\nfixture_path={}\n{PROCESS_PATH_CHECK}\n\
             if f53_process_has_fixture_path \"$process_id\" \"$fixture_path\"; then\n\
                 kill -{signal} \"$process_id\"\n\
             else\n\
                 exit 44\n\
             fi",
            shell_quote(&fixture_path)
        );
        Ok(self.run_bash_io(&command)?.status.success())
    }

    fn bash_path_literal(path: &Path, context: &str) -> String {
        shell_quote(&required(bash_path(path), context))
    }

    fn run_bash(&self, script: &str) -> Output {
        required(self.run_bash_io(script), "run controlled Bash process")
    }

    fn run_bash_io(&self, script: &str) -> io::Result<Output> {
        Command::new(&self.bash).arg("-c").arg(script).output()
    }

    fn read_text(path: &Path) -> io::Result<Option<String>> {
        read_optional_text(path)
    }

    fn read_arguments(&self) -> io::Result<Option<Vec<String>>> {
        read_nul_delimited(&self.copilot_arguments)
    }

    fn read_events(&self) -> io::Result<Vec<String>> {
        Ok(Self::read_text(&self.events)?
            .map(|contents| contents.lines().map(str::to_owned).collect())
            .unwrap_or_default())
    }

    fn read_pid(path: &Path) -> io::Result<Option<u32>> {
        Self::read_text(path)?
            .map(|contents| {
                contents.trim().parse::<u32>().map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid F53 test-created PID marker: {error}"),
                    )
                })
            })
            .transpose()
    }

    fn clear_markers(&self) {
        for marker in [
            &self.events,
            &self.preflight_result,
            &self.copilot_arguments,
            &self.owner_pid_marker,
            &self.descendant_pid_marker,
            &self.release_owner_marker,
            &self.release_descendant_marker,
            &self.descendant_exited_marker,
            &self.launcher_pid_marker,
            &self.watchdog_marker,
        ] {
            match fs::remove_file(marker) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => panic!(
                    "F53_HARNESS_INFRASTRUCTURE: remove stale fixture marker {}: {error}",
                    marker.display()
                ),
            }
        }
    }

    fn release_and_reap_test_processes(&self) {
        let _ = fs::write(&self.release_owner_marker, "release");
        let _ = fs::write(&self.release_descendant_marker, "release");
        for marker in [&self.owner_pid_marker, &self.descendant_pid_marker] {
            if let Ok(Some(process_id)) = Self::read_pid(marker) {
                let deadline = Instant::now() + Duration::from_secs(2);
                while Instant::now() < deadline
                    && self.process_is_fixture(process_id).unwrap_or(false)
                {
                    thread::sleep(Duration::from_millis(25));
                }
                if self.process_is_fixture(process_id).unwrap_or(false) {
                    let _ =
                        self.signal_process_if_path(process_id, &self.preflight_executable, "TERM");
                }
            }
        }
    }
}

impl Drop for LauncherFixture {
    fn drop(&mut self) {
        self.release_and_reap_test_processes();
    }
}

#[test]
fn unix_launcher_requires_successful_preflight_and_preserves_spaced_invocation() {
    let fixture = LauncherFixture::new();
    let workspace_argument = required(
        bash_path(&fixture.workspace_argument),
        "convert exact spaced argument path",
    );
    let arguments = vec![
        "--session".to_owned(),
        "session name with spaces".to_owned(),
        "--workspace".to_owned(),
        workspace_argument,
    ];

    let output = fixture.run("succeeded", &arguments);
    let preflight_result = required(
        LauncherFixture::read_text(&fixture.preflight_result),
        "read successful-preflight fixture result",
    );
    let events = required(fixture.read_events(), "read successful-launch events");
    let copilot_arguments = required(fixture.read_arguments(), "read Copilot invocation");
    let mut expected_invocation = vec![required(
        bash_path(&fixture.copilot_executable),
        "convert exact Copilot executable path",
    )];
    expected_invocation.extend(arguments);

    assert!(
        output.status.success()
            && preflight_result.as_deref() == Some(PREFLIGHT_SUCCEEDED)
            && events == ["preflight:succeeded", "copilot"]
            && copilot_arguments.as_deref() == Some(expected_invocation.as_slice()),
        "F53_CONTRACT_FAILURE[successful-preflight-launch]: expected successful F50 preflight \
         before Copilot with the exact spaced executable path and arguments; \
         result={preflight_result:?}, events={events:?}, invocation={copilot_arguments:?}; {}",
        launcher_output(&output)
    );
}

#[test]
fn unix_launcher_reports_typed_failure_without_invoking_copilot() {
    let fixture = LauncherFixture::new();
    let output = fixture.run("failed", &[]);
    let preflight_result = required(
        LauncherFixture::read_text(&fixture.preflight_result),
        "read failing-preflight fixture result",
    );
    let events = required(fixture.read_events(), "read failed-launch events");
    let copilot_started = required(
        fixture.read_arguments(),
        "read Copilot marker after failed preflight",
    )
    .is_some();
    let transcript = launcher_output(&output);

    assert!(
        !output.status.success()
            && preflight_result.as_deref() == Some(PREFLIGHT_FAILED)
            && transcript.contains("McpProbeVerified")
            && events == ["preflight:failed"]
            && !copilot_started,
        "F53_CONTRACT_FAILURE[typed-failure-no-copilot]: expected typed stage \
         McpProbeVerified to be reported without invoking Copilot; \
         result={preflight_result:?}, events={events:?}, copilot_started={copilot_started}; \
         {transcript}"
    );
}

#[test]
fn unix_launcher_stops_only_its_child_and_leaves_the_unowned_descendant_alive() {
    let fixture = LauncherFixture::new();
    let output = fixture.run_cleanup();
    let owner_pid = required(
        LauncherFixture::read_pid(&fixture.owner_pid_marker),
        "read test-created launcher-owned child PID",
    );
    let descendant_pid = required(
        LauncherFixture::read_pid(&fixture.descendant_pid_marker),
        "read test-created unowned descendant PID",
    );
    let owner_running = owner_pid.is_some_and(|process_id| {
        required(
            fixture.process_is_fixture(process_id),
            "verify launcher-owned PID still has the preflight fixture path",
        )
    });
    let descendant_running = descendant_pid.is_some_and(|process_id| {
        required(
            fixture.process_is_fixture(process_id),
            "verify descendant PID still has the preflight fixture path",
        )
    });
    let events = required(fixture.read_events(), "read cleanup fixture events");
    let copilot_started = required(
        fixture.read_arguments(),
        "read Copilot marker after cleanup timeout",
    )
    .is_some();
    let watchdog_used = required(
        LauncherFixture::read_text(&fixture.watchdog_marker),
        "read launcher watchdog marker",
    )
    .is_some();
    let transcript = launcher_output(&output);

    fixture.release_and_reap_test_processes();

    assert!(
        owner_pid.is_some()
            && descendant_pid.is_some()
            && !owner_running
            && descendant_running
            && events == ["preflight:cleanup"]
            && !copilot_started
            && !watchdog_used,
        "F53_CONTRACT_FAILURE[exact-child-cleanup]: expected the launcher-owned preflight \
         child to stop while its test-created unowned descendant survives; \
         owner_pid={owner_pid:?}, owner_running={owner_running}, \
         descendant_pid={descendant_pid:?}, descendant_running={descendant_running}, \
         events={events:?}, copilot_started={copilot_started}, \
         watchdog_used={watchdog_used}, launcher_status={:?}; {transcript}",
        output.status
    );
    assert!(
        descendant_pid.is_none()
            || required(
                LauncherFixture::read_text(&fixture.descendant_exited_marker),
                "read released descendant marker",
            )
            .as_deref()
                == Some("released"),
        "F53_HARNESS_INFRASTRUCTURE: test-created descendant failed to exit after release"
    );
}

fn required<T>(result: io::Result<T>, context: &str) -> T {
    result.unwrap_or_else(|error| {
        panic!("F53_HARNESS_INFRASTRUCTURE: {context}: {error}");
    })
}

fn read_optional_text(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn read_nul_delimited(path: &Path) -> io::Result<Option<Vec<String>>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if bytes.last() != Some(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Copilot fixture argument record is not NUL-terminated",
        ));
    }
    Ok(Some(
        bytes[..bytes.len() - 1]
            .split(|byte| *byte == 0)
            .map(|value| String::from_utf8_lossy(value).into_owned())
            .collect(),
    ))
}

fn bash_path(path: &Path) -> io::Result<String> {
    let canonical = match fs::canonicalize(path) {
        Ok(canonical) => canonical,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Bash fixture path has no parent directory",
                )
            })?;
            let file_name = path.file_name().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Bash fixture path has no file name",
                )
            })?;
            fs::canonicalize(parent)?.join(file_name)
        }
        Err(error) => return Err(error),
    };
    #[cfg(windows)]
    {
        let path = canonical.to_string_lossy().replace('\\', "/");
        let path = path.strip_prefix("//?/").unwrap_or(&path);
        let Some((drive, remainder)) = path.split_once(':') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "temporary workspace path is not on a Windows drive",
            ));
        };
        if drive.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "temporary workspace path has an unsupported Windows drive",
            ));
        }
        Ok(format!(
            "/mnt/{}/{}",
            drive.to_ascii_lowercase(),
            remainder.trim_start_matches('/')
        ))
    }

    #[cfg(not(windows))]
    {
        Ok(canonical.to_string_lossy().replace('\\', "/"))
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn launcher_output(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn wait_for_child(child: &mut Child, timeout: Duration) -> io::Result<bool> {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait()?.is_some() {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        thread::sleep(Duration::from_millis(20));
    }
}
