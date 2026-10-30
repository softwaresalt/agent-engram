//! Bash startup is independent of index preparation.
#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bash() -> PathBuf {
    if cfg!(windows) {
        for variable in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Some(base) = std::env::var_os(variable) {
                let base = PathBuf::from(base);
                let base = if variable == "LOCALAPPDATA" {
                    base.join("Programs")
                } else {
                    base
                };
                let candidate = base.join("Git").join("bin").join("bash.exe");
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from("bash")
}

fn unix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[test]
fn launches_without_engram_preserving_workspace_arguments_and_exit_status() {
    let temp = tempfile::tempdir().expect("fixture");
    let workspace = temp.path().join("work space");
    fs::create_dir(&workspace).expect("workspace");
    let script = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("start.sh"))
        .expect("launcher");
    fs::write(workspace.join("start.sh"), script.replace("\r\n", "\n")).expect("copy launcher");
    let copilot = workspace.join("copilot");
    fs::write(
        &copilot,
        "#!/usr/bin/env bash\nprintf '%s\\n' \"$PWD\" \"$@\" > \"$LAUNCH_RECORD\"\nexit 17\n",
    )
    .expect("Copilot stub");
    let shell = bash();
    let chmod = Command::new(&shell)
        .args(["-c", "chmod +x \"$1\"", "--"])
        .arg(unix_path(&copilot))
        .status()
        .expect("chmod");
    assert!(chmod.success());
    let record = workspace.join("launch.txt");
    let output = Command::new(&shell)
        .arg(unix_path(&workspace.join("start.sh")))
        .args(["--probe", "argument with spaces"])
        .current_dir(temp.path())
        .env("COPILOT_EXE_PATH", unix_path(&copilot))
        .env("LAUNCH_RECORD", unix_path(&record))
        .env("GITHUB_TOKEN", "fixture-token")
        .env("ENGRAM_EXE_PATH", "missing-engram")
        .env("ENGRAM_PREWARM_TIMEOUT_MS", "obsolete-not-a-budget")
        .output()
        .expect("launch");
    assert_eq!(output.status.code(), Some(17), "{output:?}");
    let recorded = fs::read_to_string(record).expect("Copilot launched");
    let lines: Vec<_> = recorded.lines().collect();
    assert!(lines[0].ends_with("/work space"), "{recorded}");
    assert_eq!(&lines[1..], ["--probe", "argument with spaces"]);
    assert!(!workspace.join(".engram").exists());
}

#[test]
fn launcher_contains_no_indexing_or_preflight_entrypoint() {
    let script = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("start.sh"))
        .expect("launcher");
    assert!(!script.contains("run_engram_preflight"));
    assert!(!script.contains("ENGRAM_PREWARM_TIMEOUT_MS"));
    assert!(!script.contains("mktemp"));
}
