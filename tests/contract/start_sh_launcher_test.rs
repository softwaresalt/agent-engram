//! Behavior-level contract tests for the F53 Unix launcher (`start.sh`).
//!
//! The launcher is copied into an isolated workspace whose path contains
//! spaces. Controlled `engram` (preflight) and `copilot` executables record
//! what the launcher does, so each test asserts observable behavior only.
//!
//! See docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md
//! ("The PA-2 launcher contract text").
#![forbid(unsafe_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    thread,
    time::Duration,
};

use tempfile::TempDir;

/// Controlled `engram`: behavior comes from `mode.txt` in the workspace root.
const ENGRAM_FIXTURE: &str = r#"#!/usr/bin/env bash
root="$(cd "$(dirname "$0")/../.." && pwd)"
printf 'preflight\n' >> "$root/events.log"
for argument in "$@"; do printf 'arg:%s\n' "$argument" >> "$root/events.log"; done
case "$(cat "$root/mode.txt")" in
    succeeded)
        printf '%s\n' '{"state":"Succeeded"}'
        ;;
    failed)
        printf '%s\n' '{"state":"Failed","stage":"McpProbeVerified"}'
        printf '%s\n' 'preflight-stderr-detail' >&2
        exit 23
        ;;
    degraded)
        printf '%s\n' '{"state":"Degraded","stage":"IndexReady"}'
        ;;
    hang)
        sleep 60 >/dev/null 2>&1 &
        printf '%s\n' "$!" > "$root/descendant.pid"
        printf '%s\n' "$$" > "$root/owner.pid"
        wait
        ;;
esac
"#;

/// Controlled `copilot`: records that it was launched and its arguments.
const COPILOT_FIXTURE: &str = r#"#!/usr/bin/env bash
root="$(cd "$(dirname "$0")/../.." && pwd)"
printf 'copilot\n' >> "$root/events.log"
for argument in "$@"; do printf 'copilot-arg:%s\n' "$argument" >> "$root/events.log"; done
"#;

struct Fixture {
    _directory: TempDir,
    bash: PathBuf,
    workspace: PathBuf,
}

struct Run {
    output: Output,
    events: Vec<String>,
}

impl Run {
    fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).into_owned()
    }

    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }

    fn launched(&self) -> bool {
        self.events.iter().any(|event| event == "copilot")
    }

    fn describe(&self) -> String {
        format!(
            "status={:?}\nstdout={}\nstderr={}\nevents={:?}",
            self.output.status.code(),
            self.stdout(),
            self.stderr(),
            self.events
        )
    }
}

fn forward_slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn git_bash_or_bash() -> PathBuf {
    if cfg!(windows) {
        // `bash.exe` on a Windows PATH is often the WSL shim; prefer Git Bash.
        let git_bash = PathBuf::from(r"C:\Program Files\Git\bin\bash.exe");
        if git_bash.is_file() {
            return git_bash;
        }
    }
    PathBuf::from("bash")
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let directory = TempDir::new().expect("create temporary directory");
        let workspace = directory.path().join("launcher workspace with spaces");
        let engram_dir = workspace.join("target").join("debug");
        let copilot_dir = workspace.join("GitHub Copilot").join("bin dir");
        for dir in [&engram_dir, &copilot_dir] {
            fs::create_dir_all(dir).expect("create fixture directory");
        }
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("start.sh");
        fs::copy(source, workspace.join("start.sh")).expect("copy start.sh");
        fs::write(engram_dir.join("engram"), ENGRAM_FIXTURE).expect("write engram fixture");
        fs::write(copilot_dir.join("copilot"), COPILOT_FIXTURE).expect("write copilot fixture");
        fs::write(workspace.join("mode.txt"), mode).expect("write mode");
        let fixture = Self {
            _directory: directory,
            bash: git_bash_or_bash(),
            workspace,
        };
        let mark = format!(
            "chmod +x '{}' '{}'",
            forward_slashes(&engram_dir.join("engram")),
            forward_slashes(&copilot_dir.join("copilot"))
        );
        let output = fixture.bash_output(&mark);
        assert!(output.status.success(), "chmod failed: {output:?}");
        fixture
    }

    fn bash_output(&self, script: &str) -> Output {
        Command::new(&self.bash)
            .arg("-c")
            .arg(script)
            .output()
            .expect("run bash")
    }

    fn copilot_path(&self) -> String {
        forward_slashes(
            &self
                .workspace
                .join("GitHub Copilot")
                .join("bin dir")
                .join("copilot"),
        )
    }

    fn read(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.workspace.join(name))
            .ok()
            .map(|text| text.trim().to_owned())
    }

    fn pid_alive(&self, pid: &str) -> bool {
        self.bash_output(&format!("kill -0 {pid}")).status.success()
    }

    fn run_launcher(&self, budget_ms: Option<&str>) -> Run {
        let mut command = Command::new(&self.bash);
        command
            .arg(forward_slashes(&self.workspace.join("start.sh")))
            .arg("--probe")
            .arg("argument with spaces")
            .current_dir(&self.workspace)
            .env("COPILOT_EXE_PATH", self.copilot_path())
            .env("GITHUB_TOKEN", "fixture-token")
            .env_remove("ENGRAM_PREWARM_TIMEOUT_MS");
        if let Some(budget) = budget_ms {
            command.env("ENGRAM_PREWARM_TIMEOUT_MS", budget);
        }
        let output = command.output().expect("run start.sh");
        let events = self
            .read("events.log")
            .map(|text| text.lines().map(str::to_owned).collect())
            .unwrap_or_default();
        Run { output, events }
    }
}

#[test]
fn launches_agent_only_after_preflight_succeeded() {
    let fixture = Fixture::new("succeeded");
    let run = fixture.run_launcher(None);

    assert!(run.output.status.success(), "{}", run.describe());
    // Preflight must run first, then the agent; both exactly once.
    let order: Vec<&str> = run
        .events
        .iter()
        .map(String::as_str)
        .filter(|event| *event == "preflight" || *event == "copilot")
        .collect();
    assert_eq!(order, ["preflight", "copilot"], "{}", run.describe());
    // PA-2: `preflight "$WORKSPACE" --timeout-ms <outer budget 15000 - 3000>`,
    // with the spaced workspace path preserved as one argument.
    let preflight_args: Vec<&String> = run
        .events
        .iter()
        .filter(|event| event.starts_with("arg:"))
        .collect();
    assert_eq!(preflight_args.len(), 4, "{}", run.describe());
    assert_eq!(preflight_args[0], "arg:preflight", "{}", run.describe());
    assert!(
        preflight_args[1].ends_with("launcher workspace with spaces"),
        "{}",
        run.describe()
    );
    assert_eq!(preflight_args[2], "arg:--timeout-ms", "{}", run.describe());
    assert_eq!(preflight_args[3], "arg:12000", "{}", run.describe());
    // Caller arguments are forwarded to the agent unchanged.
    assert!(
        run.events.contains(&"copilot-arg:--probe".to_owned())
            && run
                .events
                .contains(&"copilot-arg:argument with spaces".to_owned()),
        "{}",
        run.describe()
    );
}

#[test]
fn preflight_failure_reports_typed_stage_and_does_not_launch() {
    let fixture = Fixture::new("failed");
    let run = fixture.run_launcher(None);

    assert!(!run.output.status.success(), "{}", run.describe());
    assert!(!run.launched(), "{}", run.describe());
    assert!(
        run.stdout()
            .contains("engram preflight failed at stage McpProbeVerified."),
        "{}",
        run.describe()
    );
    // stdout and stderr of the failed preflight are surfaced, not swallowed.
    assert!(
        run.stdout().contains(r#""state":"Failed""#),
        "{}",
        run.describe()
    );
    assert!(
        run.stderr().contains("preflight-stderr-detail"),
        "{}",
        run.describe()
    );
}

#[test]
fn exit_zero_without_succeeded_verdict_does_not_launch() {
    let fixture = Fixture::new("degraded");
    let run = fixture.run_launcher(None);

    assert!(!run.output.status.success(), "{}", run.describe());
    assert!(!run.launched(), "{}", run.describe());
    assert!(
        run.stdout()
            .contains("engram preflight failed at stage IndexReady."),
        "{}",
        run.describe()
    );
}

#[test]
fn timeout_cleanup_terminates_only_the_launchers_own_child() {
    let fixture = Fixture::new("hang");
    // Smallest legal budget: the launcher waits 3001 ms, then must clean up.
    let run = fixture.run_launcher(Some("3001"));
    // Give any (incorrect) signal delivery a moment to land before sampling.
    thread::sleep(Duration::from_millis(300));

    let owner = fixture.read("owner.pid").unwrap_or_default();
    let descendant = fixture.read("descendant.pid").unwrap_or_default();
    let owner_alive = !owner.is_empty() && fixture.pid_alive(&owner);
    let descendant_alive = !descendant.is_empty() && fixture.pid_alive(&descendant);
    if descendant_alive {
        // Never leak the unowned descendant out of the test.
        let _ = fixture.bash_output(&format!("kill {descendant}"));
    }

    assert!(
        !owner.is_empty() && !descendant.is_empty(),
        "{}",
        run.describe()
    );
    assert!(!run.output.status.success(), "{}", run.describe());
    assert!(!run.launched(), "{}", run.describe());
    assert!(!owner_alive, "owned child survived: {}", run.describe());
    assert!(
        descendant_alive,
        "unowned descendant was terminated: {}",
        run.describe()
    );
}
