//! Contract coverage for NEW-5 (142.073-T): the hidden `engram preflight`
//! subcommand, which forwards its positional workspace and `--timeout-ms` to
//! the root preflight relay and ignores the global flags and `ENGRAM_WORKSPACE`.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const SFX: &str = std::env::consts::EXE_SUFFIX;
const OK: &str = r#"{"state":"Succeeded"}"#;
const BUILD_FAILED: &str = r#"{"state":"Failed","stage":"Build"}"#;

const FIXTURE_SRC: &str = r##"
fn main() {
    let exe = std::env::current_exe().unwrap_or_default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let _ = std::fs::write(exe.with_extension("argv"), args.join("\n"));
    println!("{}", r#"{"state":"Succeeded"}"#);
}
"##;

/// Copies the real binary into a spaced temp dir so the sibling lookup is
/// controlled by the test.
fn copied_engram() -> (TempDir, PathBuf) {
    let dir = tempfile::Builder::new()
        .prefix("preflight cmd ")
        .tempdir()
        .expect("tempdir");
    let exe = dir.path().join(format!("engram{SFX}"));
    fs::copy(env!("CARGO_BIN_EXE_engram"), &exe).expect("copy engram");
    (dir, exe)
}

fn run(exe: &Path, args: &[&str], workspace_env: Option<&Path>) -> Output {
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .env_remove("ENGRAM_DATA_DIR")
        .env_remove("ENGRAM_DIRECT");
    if let Some(ws) = workspace_env {
        cmd.env("ENGRAM_WORKSPACE", ws);
    }
    // A freshly copied executable can transiently report ETXTBSY on Linux when
    // a concurrent test thread forks while the copy's write handle is open.
    for attempt in 0..20 {
        match cmd.output() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy && attempt < 19 => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            other => return other.expect("spawn engram"),
        }
    }
    unreachable!("the final attempt always returns")
}

/// Infrastructure smoke check: a copied debug binary must start at all.
fn smoke(exe: &Path) {
    let out = run(exe, &["--version"], None);
    assert!(
        out.status.success(),
        "copied engram must run: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn canonical(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|e| panic!("canonicalize {}: {e}", p.display()))
}

#[test]
fn missing_sibling_json_quiet_is_build_failure() {
    let (dir, exe) = copied_engram();
    smoke(&exe);
    let ws = dir.path().join("work space");
    fs::create_dir_all(&ws).expect("workspace");
    let ws_arg = ws.to_str().expect("utf8");

    let out = run(
        &exe,
        &[
            "--json",
            "--quiet",
            "preflight",
            ws_arg,
            "--timeout-ms",
            "5000",
        ],
        None,
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(stdout.trim_end(), BUILD_FAILED, "stderr: {stderr}");
    assert_eq!(out.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("engram-indexer"), "stderr: {stderr}");
}

#[test]
fn succeeded_sibling_ignores_env_workspace_and_forwards_positional() {
    let (dir, exe) = copied_engram();
    smoke(&exe);
    let sibling = dir.path().join(format!("engram-indexer{SFX}"));
    let src = dir.path().join("fixture.rs");
    fs::write(&src, FIXTURE_SRC).expect("fixture source");
    let status = Command::new("rustc")
        .args(["--edition", "2021", "-O", "-o"])
        .arg(&sibling)
        .arg(&src)
        .env_remove("ENGRAM_DATA_DIR")
        .status()
        .expect("rustc");
    assert!(status.success(), "fixture must compile");

    let ws = dir.path().join("positional ws");
    let other = dir.path().join("other ws");
    fs::create_dir_all(&ws).expect("workspace");
    fs::create_dir_all(&other).expect("other");

    let out = run(
        &exe,
        &[
            "preflight",
            ws.to_str().expect("utf8"),
            "--timeout-ms",
            "5000",
        ],
        Some(&other),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim_end(),
        OK,
        "stderr: {stderr}"
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {stderr}");

    let record = fs::read_to_string(sibling.with_extension("argv")).expect("argv record");
    let a: Vec<&str> = record.split('\n').collect();
    assert_eq!(a.len(), 7, "{a:?}");
    assert_eq!(a[..2], ["preflight", "--workspace"]);
    assert_eq!(a[3..6], ["--timeout-ms", "5000", "--engram"]);
    assert_eq!(canonical(Path::new(a[2])), canonical(&ws));
    assert_ne!(canonical(Path::new(a[2])), canonical(&other));
    assert_eq!(canonical(Path::new(a[6])), canonical(&exe));
}

#[test]
fn help_shows_arguments_and_top_level_hides_command() {
    let (_dir, exe) = copied_engram();
    smoke(&exe);
    let help = run(&exe, &["preflight", "--help"], None);
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(help.status.success(), "{text}");
    assert!(text.contains("<WORKSPACE>"), "{text}");
    assert!(text.contains("--timeout-ms"), "{text}");

    let top = run(&exe, &["--help"], None);
    let top_text = String::from_utf8_lossy(&top.stdout);
    assert!(top.status.success(), "{top_text}");
    assert!(
        !top_text
            .lines()
            .any(|l| l.trim_start().starts_with("preflight")),
        "preflight must stay hidden: {top_text}"
    );
}
