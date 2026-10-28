//! Contract coverage for NEW-4 (142.072-T): the root preflight relay
//! `engram::cli::commands::preflight`. Tests first call a stubbed function that
//! carries the `Worker: 142.072-T` marker while RED.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use engram::cli::commands::preflight::{
    RELAY_GRACE_MS, RELAY_STAGES, RelayCause, RelayOutcome, normalize_verdict, resolve_supervisor,
    run_relay,
};
use tempfile::TempDir;

const OK: &str = r#"{"state":"Succeeded"}"#;
const SFX: &str = std::env::consts::EXE_SUFFIX;
#[rustfmt::skip]
const CAUSES: [&str; 7] = ["ok", "missing_sibling", "relay_spawn", "relay_deadline", "relay_oversize", "relay_read", "relay_verdict"];

const FIXTURE_SRC: &str = r##"
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
const OK: &str = r#"{"state":"Succeeded"}"#;
fn main() {
    let exe = std::env::current_exe().unwrap_or_default();
    let stem = exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let forever = || { std::thread::sleep(Duration::from_secs(60)); std::process::exit(0); };
    if std::env::var_os("GC_ROLE").is_some() { forever(); }
    let _ = std::fs::write(exe.with_extension("pid"), std::process::id().to_string());
    let args: Vec<String> = std::env::args().skip(1).collect();
    let _ = std::fs::write(exe.with_extension("argv"), args.join("\n"));
    match stem.as_str() {
        "hang" => forever(),
        "big" => { let _ = std::io::stdout().write_all(&vec![b'x'; 8192]); let _ = std::io::stdout().flush(); forever(); }
        "grandchild" => {
            if let Ok(gc) = Command::new(&exe).env("GC_ROLE", "1").stdin(Stdio::null()).stderr(Stdio::null()).spawn() {
                let _ = std::fs::write(exe.with_extension("gc"), gc.id().to_string());
            }
            println!("{OK}");
        }
        _ => println!("{OK}"),
    }
}
"##;

fn anchor() {
    assert_eq!(normalize_verdict(OK.as_bytes(), Some(0)), (OK, 0));
}

fn setup(role: &str) -> (TempDir, PathBuf, PathBuf) {
    let mut b = tempfile::Builder::new();
    let dir = b
        .prefix("relay fixture ")
        .tempdir_in(env!("CARGO_MANIFEST_DIR"))
        .expect("tempdir");
    let src = dir.path().join("fixture.rs");
    fs::write(&src, FIXTURE_SRC).expect("write fixture source");
    let exe = dir.path().join(format!("{role}{SFX}"));
    let status = Command::new("rustc")
        .args(["--edition", "2021", "-O", "-o"])
        .arg(&exe)
        .arg(&src)
        .env_remove("ENGRAM_DATA_DIR")
        .status()
        .expect("run rustc");
    assert!(status.success(), "fixture {role} must compile");
    let engram = dir.path().join(format!("engram{SFX}"));
    fs::write(&engram, b"").expect("engram stand-in");
    (dir, exe, engram)
}

fn marker_pid(exe: &Path, ext: &str) -> u32 {
    let text = fs::read_to_string(exe.with_extension(ext)).unwrap_or_else(|e| panic!("{ext}: {e}"));
    text.trim().parse().expect("pid marker")
}

fn tool(cmd: &str, args: &[&str]) -> Output {
    Command::new(cmd)
        .args(args)
        .env_remove("ENGRAM_DATA_DIR")
        .output()
        .expect(cmd)
}

fn pid_alive(pid: u32) -> bool {
    let p = pid.to_string();
    if cfg!(windows) {
        let out = tool("tasklist", &["/FI", &format!("PID eq {p}"), "/NH"]);
        String::from_utf8_lossy(&out.stdout).contains(&format!(" {p} "))
    } else {
        tool("kill", &["-0", &p]).status.success()
    }
}

struct PidGuard(u32);

impl Drop for PidGuard {
    fn drop(&mut self) {
        let p = self.0.to_string();
        if cfg!(windows) {
            tool("taskkill", &["/PID", &p, "/F"]);
        } else {
            tool("kill", &["-9", &p]);
        }
    }
}

fn assert_dead_soon(pid: u32, what: &str) {
    let end = Instant::now() + Duration::from_secs(5);
    while pid_alive(pid) && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!pid_alive(pid), "{what} (pid {pid}) must not be alive");
}

fn assert_relay_line(outcome: &RelayOutcome, cause: &str) -> String {
    let line = outcome.stderr_line();
    assert!(!line.contains('\n'), "exactly one line: {line:?}");
    assert!(
        !line.starts_with("engram-preflight "),
        "supervisor prefix: {line}"
    );
    let t: Vec<&str> = line.split_whitespace().collect();
    assert_eq!(t[0], "engram-preflight-relay", "{line}");
    let stage = t[1].strip_prefix("stage=").expect("stage field");
    let got = t[2].strip_prefix("cause=").expect("cause field");
    assert_eq!(got, cause, "{line}");
    assert!(CAUSES.contains(&got), "closed cause set: {line}");
    assert_eq!(got, outcome.cause.as_str());
    let ms = t[3]
        .strip_prefix("elapsed_ms=")
        .and_then(|n| n.parse::<u64>().ok());
    assert!(ms.is_some(), "elapsed_ms field: {line}");
    if cause == "missing_sibling" {
        assert_eq!(t.len(), 5, "{line}");
        assert_eq!(t[4], format!("sibling=engram-indexer{SFX}"));
    } else {
        assert_eq!(t.len(), 4, "{line}");
    }
    stage.to_owned()
}

fn canonical(path: &str) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|e| panic!("canonicalize {path}: {e}"))
}

fn assert_build_failure(outcome: &RelayOutcome, cause: RelayCause, token: &str) {
    assert_eq!(
        (outcome.line, outcome.exit, outcome.cause),
        (RELAY_STAGES[0], 1, cause)
    );
    assert_eq!(assert_relay_line(outcome, token), "Build");
}

#[test]
fn succeeded_fixture_relays_line_and_exact_argv() {
    anchor();
    let (dir, sup, engram) = setup("ok");
    let workspace = dir.path().join("work space");
    fs::create_dir_all(&workspace).expect("workspace");

    let outcome = run_relay(&sup, &workspace, 5000, &engram);
    assert_eq!(
        (outcome.line, outcome.exit, outcome.cause),
        (OK, 0, RelayCause::Ok)
    );
    assert_relay_line(&outcome, "ok");
    let record = fs::read_to_string(sup.with_extension("argv")).expect("argv record");
    let a: Vec<&str> = record.split('\n').collect();
    assert_eq!(a.len(), 7, "{a:?}");
    assert_eq!(a[..2], ["preflight", "--workspace"]);
    assert_eq!(a[3..6], ["--timeout-ms", "5000", "--engram"]);
    assert_eq!(
        canonical(a[2]),
        canonical(workspace.to_str().expect("utf8"))
    );
    assert!(Path::new(a[6]).is_absolute(), "{}", a[6]);
    assert_eq!(canonical(a[6]), canonical(engram.to_str().expect("utf8")));
}

type Case = (&'static str, &'static [u8], Option<i32>, &'static str, i32);

#[test]
fn normalize_verdict_table() {
    let (mcp, b) = (RELAY_STAGES[6], RELAY_STAGES[0]);
    #[rustfmt::skip]
    let cases: [Case; 7] = [
        ("failed passes through", mcp.as_bytes(), Some(23), mcp, 1),
        ("succeeded, nonzero exit", OK.as_bytes(), Some(1), b, 1),
        ("malformed, zero exit", b"not a verdict", Some(0), b, 1),
        ("two lines", b"{\"state\":\"Succeeded\"}\n{\"state\":\"Succeeded\"}\n", Some(0), b, 1),
        ("crlf", b"{\"state\":\"Succeeded\"}\r\n", Some(0), OK, 0),
        ("succeeded, no exit code", OK.as_bytes(), None, b, 1),
        ("crash, no output", b"", Some(101), b, 1),
    ];
    for (name, stdout, code, line, exit) in cases {
        assert_eq!(normalize_verdict(stdout, code), (line, exit), "{name}");
    }
}

#[test]
fn missing_sibling_is_build_failure_with_static_name() {
    let (dir, _sup, engram) = setup("ok");
    let outcome = resolve_supervisor(&engram).expect_err("no sibling exists");
    assert_build_failure(&outcome, RelayCause::MissingSibling, "missing_sibling");
    let line = outcome.stderr_line();
    assert!(
        !line.contains(dir.path().to_str().expect("utf8")),
        "no path: {line}"
    );
}

#[test]
fn hanging_supervisor_is_killed_within_deadline() {
    anchor();
    let (dir, sup, engram) = setup("hang");
    let started = Instant::now();
    let outcome = run_relay(&sup, dir.path(), 500, &engram);
    let bound = Duration::from_millis(500 + RELAY_GRACE_MS + 3000);
    assert!(
        started.elapsed() < bound,
        "killed within T + grace: {:?}",
        started.elapsed()
    );
    assert_build_failure(&outcome, RelayCause::RelayDeadline, "relay_deadline");
    assert_dead_soon(marker_pid(&sup, "pid"), "hanging supervisor");
}

#[test]
fn inherited_grandchild_does_not_block_exit_keyed_relay() {
    anchor();
    let (dir, sup, engram) = setup("grandchild");
    let started = Instant::now();
    let outcome = run_relay(&sup, dir.path(), 3000, &engram);
    let elapsed = started.elapsed();
    let grandchild = marker_pid(&sup, "gc");
    let _guard = PidGuard(grandchild);
    let alive = pid_alive(grandchild);
    assert!(
        elapsed < Duration::from_millis(3000 + RELAY_GRACE_MS + 3000),
        "{elapsed:?}"
    );
    assert_eq!(
        (outcome.line, outcome.exit, outcome.cause),
        (OK, 0, RelayCause::Ok)
    );
    assert_relay_line(&outcome, "ok");
    assert!(
        alive,
        "grandchild {grandchild} must still hold the pipe at return"
    );
}

#[test]
fn oversize_output_is_oversize_not_deadline() {
    anchor();
    let (dir, sup, engram) = setup("big");
    let started = Instant::now();
    let outcome = run_relay(&sup, dir.path(), 20_000, &engram);
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "{:?}",
        started.elapsed()
    );
    assert_build_failure(&outcome, RelayCause::RelayOversize, "relay_oversize");
    assert_dead_soon(marker_pid(&sup, "pid"), "oversize supervisor");
}

#[test]
fn overflowing_timeout_spawns_nothing() {
    anchor();
    let (dir, sup, engram) = setup("ok");
    let outcome = run_relay(&sup, dir.path(), u64::MAX, &engram);
    assert_build_failure(&outcome, RelayCause::RelayDeadline, "relay_deadline");
    assert!(
        !sup.with_extension("pid").exists(),
        "nothing may be spawned"
    );
}
