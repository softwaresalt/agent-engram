//! Integration coverage for plan unit PRE-5 (142.067-T): the CLI provenance
//! probe facade `engram::preflight_probe`.
//!
//! Every test first calls `probe_cli_read` with a nonexistent exe (plan R13.6
//! F1), which carries the `Worker: 142.067-T` placeholder marker while RED.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use engram::daemon::ipc_server::ipc_endpoint;
use engram::daemon::request_entry::GENERATION_PINNED_READS;
use engram::models::config::CodeGraphConfig;
use engram::preflight_probe::{
    HEALTH_READ, PREFLIGHT_READ, ProbeError, ProbeRead, ProbeReport, probe_cli_read,
    probe_mcp_read, wait_for_generation,
};
use engram::services::generations::GenerationId;
use engram::services::generations::candidate_build::{
    build_candidate_generation, open_generation_store, publish_sealed_candidate,
    read_server_layout, seal_candidate_inventory,
};
use engram::shim::lifecycle::check_health;
use tempfile::TempDir;

const GENERATION: &str = "cand-probe-1";
const READY_TIMEOUT: Duration = Duration::from_secs(120);
const SLACK: Duration = Duration::from_secs(1);

/// One source for every process fixture; the role is the compiled exe's stem.
const FIXTURE_SRC: &str = r##"
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
const JSON: &str = r#"{"result":{"code_files":1,"functions":2,"classes":0,"interfaces":0,"edges":0,"provenance":{"generation_id":"fixture-gen"}}}"#;
fn main() {
    let exe = std::env::current_exe().unwrap_or_default();
    let stem = exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let forever = || { std::thread::sleep(Duration::from_secs(60)); std::process::exit(0); };
    if std::env::var_os("GC_ROLE").is_some() { forever(); }
    let _ = std::fs::write(exe.with_extension("pid"), std::process::id().to_string());
    match stem.as_str() {
        "hang" => forever(),
        "big" => {
            let chunk = vec![b'x'; 65536];
            let mut out = std::io::stdout();
            for _ in 0..32 { let _ = out.write_all(&chunk); }
            let _ = out.flush();
            forever();
        }
        "grandchild" => {
            if let Ok(gc) = Command::new(&exe).env("GC_ROLE", "1").stdin(Stdio::null()).stderr(Stdio::null()).spawn() {
                let _ = std::fs::write(exe.with_extension("gc"), gc.id().to_string());
            }
            println!("{JSON}");
        }
        "stderr" => { eprintln!("STDERR_LEAK_MARKER"); println!("{JSON}"); }
        _ => println!("{JSON}"),
    }
}
"##;

fn spaced_dir() -> TempDir {
    tempfile::Builder::new()
        .prefix("preflight probe ")
        .tempdir()
        .expect("tempdir")
}

fn as_str(path: &Path) -> &str {
    path.to_str().expect("utf8 path")
}

/// Plan R13.6 F1: the first call of every test, a spawn failure by design.
fn assert_spawn_failure(dir: &Path) {
    let missing = dir.join("no such dir").join("missing engram.exe");
    let result = probe_cli_read(
        &missing,
        dir,
        &PREFLIGHT_READ,
        Instant::now() + Duration::from_secs(5),
    );
    assert!(
        matches!(result, Err(ProbeError::Spawn(_))),
        "a nonexistent exe must be Spawn: {result:?}"
    );
}

fn compile_fixture(dir: &Path, name: &str) -> PathBuf {
    let src = dir.join("fixture.rs");
    fs::write(&src, FIXTURE_SRC).expect("write fixture source");
    let exe = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    let status = Command::new("rustc")
        .args(["--edition", "2021", "-O", "-o"])
        .arg(&exe)
        .arg(&src)
        .env_remove("CARGO_BIN_EXE_engram")
        .status()
        .expect("run rustc");
    assert!(status.success(), "fixture {name} must compile");
    exe
}

fn marker_pid(exe: &Path, ext: &str) -> u32 {
    fs::read_to_string(exe.with_extension(ext))
        .unwrap_or_else(|e| panic!("marker {ext} missing: {e}"))
        .trim()
        .parse()
        .expect("pid marker")
}

fn pid_alive(pid: u32) -> bool {
    if cfg!(windows) {
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .expect("tasklist");
        String::from_utf8_lossy(&out.stdout).contains(&format!(" {pid} "))
    } else {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .is_ok_and(|s| s.success())
    }
}

fn kill_pid(pid: u32) {
    let _ = if cfg!(windows) {
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
    } else {
        Command::new("kill").args(["-9", &pid.to_string()]).output()
    };
}

fn assert_dead_soon(pid: u32, what: &str) {
    let end = Instant::now() + Duration::from_secs(5);
    while pid_alive(pid) && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!pid_alive(pid), "{what} (pid {pid}) must not be alive");
}

/// Kills an exact recorded PID on drop.
struct PidGuard(u32);

impl Drop for PidGuard {
    fn drop(&mut self) {
        kill_pid(self.0);
    }
}

fn read_server_fixture() -> (TempDir, PathBuf) {
    let dir = spaced_dir();
    let root = dir.path().join("work space");
    fs::create_dir_all(root.join(".git")).expect("create .git");
    fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
    fs::create_dir_all(root.join(".engram")).expect("create .engram");
    fs::write(
        root.join(".engram").join("config.toml"),
        "log_level = \"debug\"\nmode = \"read_server\"\n",
    )
    .expect("write config");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src").join("lib.rs"),
        "pub fn alpha() -> u32 {\n    1\n}\n\npub fn beta() -> u32 {\n    2\n}\n",
    )
    .expect("write lib.rs");
    (dir, root)
}

async fn publish(root: &Path) {
    let layout = read_server_layout(as_str(root)).expect("layout");
    let store = open_generation_store(&layout, true).expect("open store");
    let id = GenerationId::new(GENERATION).expect("id");
    let built = build_candidate_generation(&layout, &store, &id, &CodeGraphConfig::default())
        .await
        .expect("build");
    let sealed = seal_candidate_inventory(built).expect("seal");
    publish_sealed_candidate(&layout, &store, sealed, "probe-harness").expect("publish");
}

/// A spawned read-server daemon, killed (exact child) on drop.
struct Daemon(Child);

impl Daemon {
    async fn spawn(root: &Path) -> Self {
        let child = Command::new(env!("CARGO_BIN_EXE_engram"))
            .args(["daemon", "--workspace", as_str(root)])
            .env_remove("ENGRAM_DATA_DIR")
            .env_remove("ENGRAM_WORKSPACE")
            .env_remove("ENGRAM_DIRECT")
            .env("ENGRAM_IDLE_TIMEOUT_MS", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn engram daemon");
        let daemon = Self(child);
        let endpoint = ipc_endpoint(root).expect("ipc endpoint");
        let end = tokio::time::Instant::now() + READY_TIMEOUT;
        while !check_health(&endpoint).await {
            assert!(
                tokio::time::Instant::now() < end,
                "daemon never became ready"
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        daemon
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn engram_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_engram"))
}

fn assert_fixture_report(report: &ProbeReport) {
    assert!(report.accepted, "{report:?}");
    assert_eq!(report.generation_id.as_deref(), Some("fixture-gen"));
    assert_eq!(
        report.data_fingerprint.as_deref(),
        Some("code_files=1;functions=2;classes=0;interfaces=0;edges=0")
    );
}

/// Scenario 1: real published read-server fixture.
#[tokio::test]
async fn probe_reports_the_published_generation() {
    let (guard, root) = read_server_fixture();
    assert_spawn_failure(guard.path());
    publish(&root).await;
    let _daemon = Daemon::spawn(&root).await;
    wait_for_generation(
        &engram_exe(),
        &root,
        &PREFLIGHT_READ,
        GENERATION,
        Instant::now() + Duration::from_secs(60),
    )
    .expect("published generation must be observed");

    let report = probe_cli_read(
        &engram_exe(),
        &root,
        &PREFLIGHT_READ,
        Instant::now() + Duration::from_secs(30),
    )
    .expect("stats probe");
    assert!(report.accepted, "{report:?}");
    assert_eq!(report.generation_id.as_deref(), Some(GENERATION));
    let fingerprint = report.data_fingerprint.expect("fingerprint");
    assert!(
        fingerprint.starts_with("code_files=1;functions=2;"),
        "{fingerprint}"
    );

    let health = probe_cli_read(
        &engram_exe(),
        &root,
        &HEALTH_READ,
        Instant::now() + Duration::from_secs(30),
    )
    .expect("health probe");
    assert!(health.accepted, "{health:?}");
}

/// Scenario 2: a non-published expected ID never matches; the published one does.
#[tokio::test]
async fn wait_for_generation_distinguishes_expected_ids() {
    let (guard, root) = read_server_fixture();
    assert_spawn_failure(guard.path());
    publish(&root).await;
    let _daemon = Daemon::spawn(&root).await;

    let deadline = Duration::from_secs(3);
    let started = Instant::now();
    let result = wait_for_generation(
        &engram_exe(),
        &root,
        &PREFLIGHT_READ,
        "cand-other",
        started + deadline,
    );
    assert!(
        matches!(result, Err(ProbeError::Mismatch | ProbeError::Deadline)),
        "{result:?}"
    );
    assert!(
        started.elapsed() <= deadline + SLACK,
        "{:?}",
        started.elapsed()
    );

    let report = wait_for_generation(
        &engram_exe(),
        &root,
        &PREFLIGHT_READ,
        GENERATION,
        Instant::now() + Duration::from_secs(60),
    )
    .expect("published id must succeed");
    assert_eq!(report.generation_id.as_deref(), Some(GENERATION));
}

/// Scenario 3a: a hanging child is killed at the deadline.
#[test]
fn hanging_child_is_killed_at_the_deadline() {
    let dir = spaced_dir();
    assert_spawn_failure(dir.path());
    let exe = compile_fixture(dir.path(), "hang");
    let started = Instant::now();
    let result = probe_cli_read(
        &exe,
        dir.path(),
        &PREFLIGHT_READ,
        Instant::now() + Duration::from_secs(1),
    );
    assert!(matches!(result, Err(ProbeError::Deadline)), "{result:?}");
    assert!(started.elapsed() <= Duration::from_secs(1) + SLACK);
    assert_dead_soon(marker_pid(&exe, "pid"), "hanging child");
}

/// Scenario 3b: an inherited grandchild holding stdout does not delay the report.
#[test]
fn inherited_grandchild_does_not_delay_the_report() {
    let dir = spaced_dir();
    assert_spawn_failure(dir.path());
    let exe = compile_fixture(dir.path(), "grandchild");
    let deadline = Duration::from_secs(5);
    let started = Instant::now();
    let result = probe_cli_read(&exe, dir.path(), &PREFLIGHT_READ, started + deadline);
    let elapsed = started.elapsed();
    let grandchild = PidGuard(marker_pid(&exe, "gc"));
    let report = result.expect("report despite the inherited stdout");
    assert!(elapsed <= deadline + SLACK, "{elapsed:?}");
    assert_fixture_report(&report);
    assert!(
        pid_alive(grandchild.0),
        "non-vacuity: grandchild must be alive at return"
    );
}

/// Scenario 3c: output over the cap is `Oversize` and the child is reaped.
#[test]
fn oversize_output_is_typed_and_the_child_is_reaped() {
    let dir = spaced_dir();
    assert_spawn_failure(dir.path());
    let exe = compile_fixture(dir.path(), "big");
    let result = probe_cli_read(
        &exe,
        dir.path(),
        &PREFLIGHT_READ,
        Instant::now() + Duration::from_secs(20),
    );
    assert!(matches!(result, Err(ProbeError::Oversize)), "{result:?}");
    assert_dead_soon(marker_pid(&exe, "pid"), "oversize child");
}

/// Scenario 3d: the child's stderr does not reach the caller's stderr.
#[test]
fn child_stderr_does_not_reach_the_caller() {
    if let Some(fixture) = std::env::var_os("PROBE_STDERR_FIXTURE") {
        let dir = spaced_dir();
        assert_spawn_failure(dir.path());
        let report = probe_cli_read(
            Path::new(&fixture),
            dir.path(),
            &PREFLIGHT_READ,
            Instant::now() + Duration::from_secs(10),
        )
        .expect("stderr fixture probe");
        assert_fixture_report(&report);
        return;
    }
    let dir = spaced_dir();
    assert_spawn_failure(dir.path());
    let exe = compile_fixture(dir.path(), "stderr");
    let output = Command::new(std::env::current_exe().expect("current exe"))
        .args([
            "--exact",
            "child_stderr_does_not_reach_the_caller",
            "--nocapture",
        ])
        .env("PROBE_STDERR_FIXTURE", &exe)
        .env_remove("CARGO_BIN_EXE_engram")
        .stdin(Stdio::null())
        .output()
        .expect("run inner test");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "inner probe failed: {stderr}");
    assert!(
        !stderr.contains("STDERR_LEAK_MARKER"),
        "child stderr leaked: {stderr}"
    );
}

/// Plan R14.6 feasibility row.
#[test]
fn preflight_read_is_a_generation_pinned_read() {
    let dir = spaced_dir();
    assert_spawn_failure(dir.path());
    assert!(
        GENERATION_PINNED_READS.contains(&PREFLIGHT_READ.mcp_tool),
        "PREFLIGHT_READ must be generation pinned"
    );
}

// ---------------------------------------------------------------------------
// PRE-6 (142.068-T): the stdio MCP provenance probe `probe_mcp_read`.
// ---------------------------------------------------------------------------

/// Test-local read whose MCP arguments are not JSON (plan R13.6 F1).
const BAD_ARGS_READ: ProbeRead = ProbeRead {
    cli_args: &["stats"],
    mcp_tool: "get_workspace_statistics",
    mcp_arguments_json: "{not json",
};

/// Test-local read whose MCP arguments are JSON but not an object.
const NON_OBJECT_ARGS_READ: ProbeRead = ProbeRead {
    cli_args: &["stats"],
    mcp_tool: "get_workspace_statistics",
    mcp_arguments_json: "[1]",
};

/// MCP fixture; the role is the compiled exe's stem. It speaks line-delimited
/// JSON-RPC: `initialize` -> id 1, `tools/call` -> id 2, exit 0 on stdin close.
const MCP_FIXTURE_SRC: &str = r##"
use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::time::Duration;
const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"0"}}}"#;
const CALL: &str = r#"{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{}"}],"structuredContent":{"code_files":1,"functions":2,"classes":0,"interfaces":0,"edges":0,"provenance":{"generation_id":"fixture-gen"}},"isError":false}}"#;
fn main() {
    let exe = std::env::current_exe().unwrap_or_default();
    let stem = exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let forever = || { std::thread::sleep(Duration::from_secs(60)); std::process::exit(0); };
    if let Some(role) = std::env::var_os("GC_ROLE") {
        if role == "chat" {
            let mut out = std::io::stdout();
            for _ in 0..30000 {
                let _ = writeln!(out, "{}", r#"{"jsonrpc":"2.0","method":"notifications/message","params":{}}"#);
                let _ = out.flush();
                std::thread::sleep(Duration::from_millis(2));
            }
            std::process::exit(0);
        }
        forever();
    }
    let _ = std::fs::write(exe.with_extension("pid"), std::process::id().to_string());
    if stem == "mcpchatty" {
        if let Ok(gc) = Command::new(&exe).env("GC_ROLE", "chat").stdin(Stdio::null()).stderr(Stdio::null()).spawn() {
            let _ = std::fs::write(exe.with_extension("gc"), gc.id().to_string());
        }
        std::process::exit(0);
    }
    match stem.as_str() {
        "mcphang" => forever(),
        "mcpgarbage" => { println!("this is not json"); forever(); }
        "mcpexit" => std::process::exit(3),
        _ => {}
    }
    if stem == "mcpgrandchild" {
        if let Ok(gc) = Command::new(&exe).env("GC_ROLE", "1").stdin(Stdio::null()).stderr(Stdio::null()).spawn() {
            let _ = std::fs::write(exe.with_extension("gc"), gc.id().to_string());
        }
    }
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.contains("\"method\":\"initialize\"") { let _ = writeln!(out, "{INIT}"); }
        else if line.contains("\"method\":\"tools/call\"") {
            let _ = writeln!(out, "{}", r#"{"jsonrpc":"2.0","method":"notifications/message","params":{}}"#);
            match stem.as_str() {
                "mcperror" => { let _ = writeln!(out, "{}", r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32000,"message":"refused"}}"#); }
                "mcpnostruct" => { let _ = writeln!(out, "{}", r#"{"jsonrpc":"2.0","id":2,"result":{"content":[]}}"#); }
                _ => { let _ = writeln!(out, "{CALL}"); }
            }
        }
        let _ = out.flush();
    }
}
"##;

fn compile_mcp_fixture(dir: &Path, name: &str) -> PathBuf {
    let src = dir.join("mcp_fixture.rs");
    fs::write(&src, MCP_FIXTURE_SRC).expect("write mcp fixture source");
    let exe = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    let status = Command::new("rustc")
        .args(["--edition", "2021", "-O", "-o"])
        .arg(&exe)
        .arg(&src)
        .env_remove("CARGO_BIN_EXE_engram")
        .status()
        .expect("run rustc");
    assert!(status.success(), "mcp fixture {name} must compile");
    exe
}

/// Reads a PID marker, retrying while the fixture is still writing it.
fn wait_marker_pid(exe: &Path, ext: &str) -> u32 {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(pid) = fs::read_to_string(exe.with_extension(ext))
            .ok()
            .and_then(|text| text.trim().parse().ok())
        {
            return pid;
        }
        assert!(Instant::now() < end, "pid marker {ext} never appeared");
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Plan R13.6 F1: the first call of every PRE-6 test; invalid MCP arguments
/// are `Malformed` and spawn nothing (the exe does not exist).
fn assert_bad_arguments_spawn_nothing(dir: &Path) {
    let missing = dir.join("no such dir").join("missing engram.exe");
    for read in [&BAD_ARGS_READ, &NON_OBJECT_ARGS_READ] {
        let result = probe_mcp_read(&missing, dir, read, Instant::now() + Duration::from_secs(5));
        assert!(
            matches!(result, Err(ProbeError::Malformed("mcp_arguments"))),
            "invalid MCP arguments must be Malformed(mcp_arguments) without a spawn: {result:?}"
        );
    }
}

/// PRE-6 scenario 1: the real shim reports the same provenance as the CLI.
#[tokio::test]
async fn mcp_probe_matches_the_cli_provenance() {
    let (guard, root) = read_server_fixture();
    assert_bad_arguments_spawn_nothing(guard.path());
    publish(&root).await;
    let _daemon = Daemon::spawn(&root).await;
    wait_for_generation(
        &engram_exe(),
        &root,
        &PREFLIGHT_READ,
        GENERATION,
        Instant::now() + Duration::from_secs(60),
    )
    .expect("published generation must be observed");

    let status = Command::new(engram_exe())
        .arg("--workspace")
        .arg(&root)
        .args(["--json", "workspace-status"])
        .env_remove("ENGRAM_DATA_DIR")
        .env_remove("ENGRAM_WORKSPACE")
        .env_remove("ENGRAM_DIRECT")
        .stdin(Stdio::null())
        .output()
        .expect("run workspace-status");
    let body: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("workspace-status JSON");
    let generation = &body["result"]["generation"];
    assert!(
        generation["active_revision"].as_u64().is_some()
            && generation["active_revision"] == generation["published_revision"],
        "the published generation must be active: {generation}"
    );
    let cli = probe_cli_read(
        &engram_exe(),
        &root,
        &PREFLIGHT_READ,
        Instant::now() + Duration::from_secs(30),
    )
    .expect("cli probe");
    let mcp = probe_mcp_read(
        &engram_exe(),
        &root,
        &PREFLIGHT_READ,
        Instant::now() + Duration::from_secs(60),
    )
    .expect("mcp probe");
    assert!(mcp.accepted, "{mcp:?}");
    assert_eq!(mcp.generation_id.as_deref(), Some(GENERATION));
    assert_eq!(mcp.generation_id, cli.generation_id);
    assert!(mcp.data_fingerprint.is_some(), "{mcp:?}");
    assert_eq!(mcp.data_fingerprint, cli.data_fingerprint);
}

/// PRE-6 scenario 2: non-JSON output, early exit, and invalid arguments.
#[test]
fn mcp_probe_rejects_bad_output() {
    let dir = spaced_dir();
    assert_bad_arguments_spawn_nothing(dir.path());
    let garbage = compile_mcp_fixture(dir.path(), "mcpgarbage");
    let result = probe_mcp_read(
        &garbage,
        dir.path(),
        &PREFLIGHT_READ,
        Instant::now() + Duration::from_secs(10),
    );
    assert!(
        matches!(result, Err(ProbeError::Malformed(_))),
        "{result:?}"
    );
    assert_dead_soon(marker_pid(&garbage, "pid"), "garbage child");

    let early = compile_mcp_fixture(dir.path(), "mcpexit");
    let result = probe_mcp_read(
        &early,
        dir.path(),
        &PREFLIGHT_READ,
        Instant::now() + Duration::from_secs(10),
    );
    assert!(
        matches!(result, Err(ProbeError::Exit(_) | ProbeError::Malformed(_))),
        "{result:?}"
    );
}

/// PRE-6 scenario 3a: a never-answering shim is killed at the deadline.
#[test]
fn mcp_probe_kills_a_hanging_shim_at_the_deadline() {
    let dir = spaced_dir();
    assert_bad_arguments_spawn_nothing(dir.path());
    let exe = compile_mcp_fixture(dir.path(), "mcphang");
    let started = Instant::now();
    let result = probe_mcp_read(
        &exe,
        dir.path(),
        &PREFLIGHT_READ,
        started + Duration::from_secs(1),
    );
    assert!(matches!(result, Err(ProbeError::Deadline)), "{result:?}");
    assert!(started.elapsed() <= Duration::from_secs(1) + SLACK);
    assert_dead_soon(wait_marker_pid(&exe, "pid"), "hanging shim");
}

/// PRE-6 scenario 3b: an inherited grandchild holding stdout does not delay the report.
#[test]
fn mcp_probe_ignores_an_inherited_grandchild() {
    let dir = spaced_dir();
    assert_bad_arguments_spawn_nothing(dir.path());
    let exe = compile_mcp_fixture(dir.path(), "mcpgrandchild");
    let deadline = Duration::from_secs(5);
    let started = Instant::now();
    let result = probe_mcp_read(&exe, dir.path(), &PREFLIGHT_READ, started + deadline);
    let elapsed = started.elapsed();
    let grandchild = PidGuard(wait_marker_pid(&exe, "gc"));
    let report = result.expect("report despite the inherited stdout");
    assert!(elapsed <= deadline + SLACK, "{elapsed:?}");
    assert_fixture_report(&report);
    assert!(
        pid_alive(grandchild.0),
        "non-vacuity: grandchild must be alive at return"
    );
}

/// PRE-6 scenario 2 (typed refusals): an error reply is `accepted: false`, and
/// a result without `structuredContent` is `Malformed`; a notification line
/// before the response is skipped in both cases.
#[test]
fn mcp_probe_types_refusals_and_missing_structured_content() {
    let dir = spaced_dir();
    assert_bad_arguments_spawn_nothing(dir.path());
    let refused = compile_mcp_fixture(dir.path(), "mcperror");
    let report = probe_mcp_read(
        &refused,
        dir.path(),
        &PREFLIGHT_READ,
        Instant::now() + Duration::from_secs(10),
    )
    .expect("typed refusal");
    assert!(!report.accepted, "{report:?}");
    assert_eq!(report.generation_id, None);

    let bare = compile_mcp_fixture(dir.path(), "mcpnostruct");
    let result = probe_mcp_read(
        &bare,
        dir.path(),
        &PREFLIGHT_READ,
        Instant::now() + Duration::from_secs(10),
    );
    assert!(
        matches!(result, Err(ProbeError::Malformed("no_structured_content"))),
        "{result:?}"
    );
}

/// Copilot review regression: a grandchild that keeps the stdout pipe busy
/// with notifications must not hide the shim's exit; the probe reports `Exit`
/// well before the deadline instead of consuming it.
#[test]
fn mcp_probe_detects_exit_despite_a_chatty_grandchild() {
    let dir = spaced_dir();
    assert_bad_arguments_spawn_nothing(dir.path());
    let exe = compile_mcp_fixture(dir.path(), "mcpchatty");
    let deadline = Duration::from_secs(10);
    let started = Instant::now();
    let result = probe_mcp_read(&exe, dir.path(), &PREFLIGHT_READ, started + deadline);
    let elapsed = started.elapsed();
    let grandchild = PidGuard(wait_marker_pid(&exe, "gc"));
    assert!(matches!(result, Err(ProbeError::Exit(_))), "{result:?}");
    assert!(
        elapsed + SLACK < deadline,
        "exit must be detected before the deadline: {elapsed:?}"
    );
    assert!(
        pid_alive(grandchild.0),
        "non-vacuity: chatty grandchild must be alive at return"
    );
}
