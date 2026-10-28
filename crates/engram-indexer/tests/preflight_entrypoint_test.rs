//! Harness for the `engram-indexer preflight` entrypoint (plan unit NEW-3, task 142.071-T).
//!
//! Drives the real binary through `CARGO_BIN_EXE_engram-indexer`. Expected test count: 3.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VERDICT_FAILED_BUILD: &str = "{\"state\":\"Failed\",\"stage\":\"Build\"}\n";

fn indexer() -> &'static str {
    env!("CARGO_BIN_EXE_engram-indexer")
}

fn absolute_engram() -> PathBuf {
    PathBuf::from(indexer())
        .canonicalize()
        .expect("canonicalize indexer binary")
}

/// Sorted `(relative path, contents)` listing of every entry under `root`.
fn snapshot(root: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, Option<Vec<u8>>)>) {
        for entry in fs::read_dir(dir).expect("read dir") {
            let path = entry.expect("dir entry").path();
            let relative = path
                .strip_prefix(root)
                .expect("relative path")
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                out.push((relative, None));
                walk(&path, root, out);
            } else {
                out.push((relative, Some(fs::read(&path).expect("read file"))));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

fn run_preflight(workspace: &Path, timeout_ms: &str) -> Output {
    Command::new(indexer())
        .arg("preflight")
        .arg("--workspace")
        .arg(workspace)
        .arg("--timeout-ms")
        .arg(timeout_ms)
        .arg("--engram")
        .arg(absolute_engram())
        .env_remove("ENGRAM_DATA_DIR")
        .output()
        .expect("run engram-indexer")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn zero_deadline_and_non_git_setup_share_a_verdict_with_distinct_causes() {
    let base = tempfile::tempdir().expect("tempdir");

    let git_fixture = base.path().join("git fixture");
    fs::create_dir_all(git_fixture.join(".git")).expect("create .git");
    fs::write(
        git_fixture.join(".git").join("HEAD"),
        "ref: refs/heads/main\n",
    )
    .expect("HEAD");
    fs::write(git_fixture.join("lib.rs"), "pub fn seeded() {}\n").expect("source");
    let before = snapshot(&git_fixture);

    let output = run_preflight(&git_fixture, "0");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        VERDICT_FAILED_BUILD
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_of(&output).contains("cause=deadline"),
        "stderr: {}",
        stderr_of(&output)
    );
    assert_eq!(snapshot(&git_fixture), before, "git fixture was modified");

    let plain = base.path().join("plain dir");
    fs::create_dir_all(&plain).expect("create plain dir");
    fs::write(plain.join("note.txt"), "x\n").expect("note");
    let before = snapshot(&plain);

    let output = run_preflight(&plain, "60000");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        VERDICT_FAILED_BUILD
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("cause=setup"), "stderr: {stderr}");
    assert!(
        !stderr.contains("plain dir"),
        "stderr leaked the workspace path: {stderr}"
    );
    assert_eq!(snapshot(&plain), before, "plain dir was modified");
    assert!(!plain.join(".engram").exists());
}

#[test]
fn usage_error_exits_two_with_empty_stdout() {
    let output = Command::new(indexer())
        .arg("preflight")
        .arg("--bogus")
        .env_remove("ENGRAM_INDEXER_GENERATION_ROOT")
        .output()
        .expect("run engram-indexer");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn no_arguments_keeps_the_legacy_environment_failure() {
    let output = Command::new(indexer())
        .env_remove("ENGRAM_INDEXER_GENERATION_ROOT")
        .env_remove("ENGRAM_INDEXER_GENERATION_ID")
        .env_remove("ENGRAM_INDEXER_TARGET_RELATIVE_PATH")
        .env_remove("ENGRAM_INDEXER_DATA_DIR")
        .output()
        .expect("run engram-indexer");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_of(&output).contains("missing required environment variable"),
        "stderr: {}",
        stderr_of(&output)
    );
}
