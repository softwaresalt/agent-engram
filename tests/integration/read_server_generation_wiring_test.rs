//! Integration coverage for plan unit PRE-3a (142.076-T): read-server startup
//! identity is sourced from `read_server_layout`.
//!
//! The harness spawns the real `CARGO_BIN_EXE_engram daemon --workspace <W>`
//! with `mode = "read_server"` persisted in `<W>/.engram/config.toml`.
//!
//! * Scenario 1, identity part: the reported `workspace_id` equals both
//!   `read_server_layout(W).workspace_id` and
//!   `workspace_hash(&canonicalize_workspace(W), "main")`.
//! * Scenario 3: managed mode reaches `ready` and creates no generation
//!   state; a read-server daemon on a non-Git directory refuses to start with
//!   the same status and `NotGitRoot` text HEAD produced (RED-phase baseline).
//!
//! PRE-3b (142.077-T) adds:
//!
//! * Scenario 1, activation part: a revision published before spawn is
//!   activated and observable at the first `ready`.
//! * Scenario 2 except its transient step: absent / empty / wrong-digest /
//!   valid-revision-2 progression.
//! * P2-1: an install failure with the root present at startup is final.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use engram::daemon::ipc_server::ipc_endpoint;
use engram::daemon::protocol::{IpcRequest, IpcResponse};
use engram::daemon::request_entry::GENERATION_PINNED_READS;
use engram::db::workspace::{canonicalize_workspace, workspace_hash};
use engram::models::config::CodeGraphConfig;
use engram::services::generations::GenerationId;
use engram::services::generations::candidate_build::{
    build_candidate_generation, open_generation_store, publish_sealed_candidate,
    read_server_layout, seal_candidate_inventory,
};
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::Value;
use tempfile::TempDir;
use tokio::process::{Child, Command};

const READY_TIMEOUT: Duration = Duration::from_secs(120);
const IPC_TIMEOUT: Duration = Duration::from_secs(30);
const REFUSAL_TIMEOUT: Duration = Duration::from_secs(120);
const ACTIVATION_TIMEOUT: Duration = Duration::from_secs(30);
/// Longer than two capped (2 s) driver ticks plus the 100 ms start of backoff.
const QUIET_WINDOW: Duration = Duration::from_secs(5);
const PRODUCER: &str = "wiring-harness";
/// Text of `WorkspaceError::NotGitRoot` captured in the RED-phase baseline.
const NOT_GIT_ROOT_TEXT: &str = "is not a Git repository root";

fn spaced_dir() -> TempDir {
    tempfile::Builder::new()
        .prefix("read server wiring ")
        .tempdir()
        .expect("tempdir")
}

fn write_config(root: &Path, mode: Option<&str>) {
    let engram_dir = root.join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");
    let mut config = String::from("log_level = \"debug\"\n");
    if let Some(mode) = mode {
        writeln!(config, "mode = \"{mode}\"").expect("write mode");
    }
    fs::write(engram_dir.join("config.toml"), config).expect("write config");
}

fn git_fixture(mode: Option<&str>) -> (TempDir, PathBuf) {
    let dir = spaced_dir();
    let root = dir.path().join("work space");
    fs::create_dir_all(root.join(".git")).expect("create .git");
    fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
    write_config(&root, mode);
    (dir, root)
}

fn as_str(path: &Path) -> &str {
    path.to_str().expect("utf8 path")
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(dir).expect("read_dir") {
            let path = entry.expect("entry").path();
            let rel = path.strip_prefix(base).expect("relative").to_path_buf();
            if path.is_dir() {
                out.insert(rel, None);
                walk(base, &path, out);
            } else {
                out.insert(rel, Some(fs::read(&path).expect("read")));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// A spawned daemon that is killed when dropped.
struct Daemon {
    _child: Child,
    endpoint: String,
}

impl Daemon {
    async fn spawn(root: &Path) -> Self {
        let endpoint = ipc_endpoint(root).expect("ipc endpoint");
        let child = Command::new(env!("CARGO_BIN_EXE_engram"))
            .args(["daemon", "--workspace", as_str(root)])
            .env_remove("ENGRAM_DATA_DIR")
            .env_remove("ENGRAM_DIRECT")
            .env_remove("ENGRAM_WORKSPACE")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn engram daemon");
        let daemon = Self {
            _child: child,
            endpoint,
        };
        let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
        while !check_health(&daemon.endpoint).await {
            assert!(
                tokio::time::Instant::now() < deadline,
                "daemon did not reach ready before the timeout"
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        daemon
    }

    /// Send `method` and return the raw response, errors included.
    async fn raw(&self, method: &str) -> IpcResponse {
        let request = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(Value::Number(1.into())),
            method: method.to_owned(),
            params: None,
        };
        send_request(&self.endpoint, &request, IPC_TIMEOUT)
            .await
            .expect("IPC request must succeed")
    }

    /// `get_workspace_status` result, or `None` on any IPC or domain error.
    async fn try_status(&self) -> Option<Value> {
        let request = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(Value::Number(1.into())),
            method: "get_workspace_status".to_owned(),
            params: None,
        };
        let response = send_request(&self.endpoint, &request, IPC_TIMEOUT)
            .await
            .ok()?;
        if response.error.is_some() {
            return None;
        }
        response.result
    }

    async fn call(&self, method: &str) -> Value {
        let request = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(Value::Number(1.into())),
            method: method.to_owned(),
            params: None,
        };
        let response = send_request(&self.endpoint, &request, IPC_TIMEOUT)
            .await
            .expect("IPC request must succeed");
        assert!(
            response.error.is_none(),
            "unexpected IPC error from {method}: {:?}",
            response.error
        );
        response.result.unwrap_or(Value::Null)
    }
}

/// Scenario 1, identity part.
#[tokio::test]
async fn read_server_reports_the_layout_workspace_id() {
    let (_guard, root) = git_fixture(Some("read_server"));
    let daemon = Daemon::spawn(&root).await;

    let report = daemon.call("get_health_report").await;
    let reported = report["workspace_id"]
        .as_str()
        .unwrap_or_else(|| panic!("health report must carry a workspace_id: {report}"));

    let layout = read_server_layout(as_str(&root)).expect("layout");
    let canonical = canonicalize_workspace(as_str(&root)).expect("canonical workspace");
    assert_eq!(reported, layout.workspace_id, "layout identity");
    assert_eq!(
        reported,
        workspace_hash(&canonical, "main"),
        "inline identity"
    );
}

/// Scenario 3, managed part.
#[tokio::test]
async fn managed_mode_reaches_ready_without_generation_state() {
    let (_guard, root) = git_fixture(None);
    let daemon = Daemon::spawn(&root).await;

    assert!(
        !root.join(".engram").join("generations").exists(),
        "managed mode must not create .engram/generations"
    );
    let status = daemon.call("get_workspace_status").await;
    assert!(
        status.get("generation").is_none_or(Value::is_null),
        "managed mode must report no generation observability: {status}"
    );
}

/// Scenario 3, non-Git refusal part.
#[tokio::test]
async fn read_server_refuses_a_non_git_directory_and_leaves_it_unchanged() {
    let dir = spaced_dir();
    let root = dir.path().join("plain space");
    write_config(&root, Some("read_server"));
    let before = snapshot(&root);

    let output = tokio::time::timeout(
        REFUSAL_TIMEOUT,
        Command::new(env!("CARGO_BIN_EXE_engram"))
            .args(["daemon", "--workspace", as_str(&root)])
            .env_remove("ENGRAM_DATA_DIR")
            .env_remove("ENGRAM_DIRECT")
            .env_remove("ENGRAM_WORKSPACE")
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("daemon must exit on refusal")
    .expect("run daemon");

    // RED-phase baseline at HEAD: exit status 1, empty stdout, refusal on stderr.
    assert_eq!(output.status.code(), Some(1), "refusal exit status");
    assert!(output.stdout.is_empty(), "refusal writes nothing to stdout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(NOT_GIT_ROOT_TEXT),
        "refusal text must name NotGitRoot: {stderr}"
    );

    // HEAD creates only the empty `.engram/run` directory before refusing.
    let mut expected = before;
    expected.insert(PathBuf::from(".engram").join("run"), None);
    assert_eq!(snapshot(&root), expected, "directory content after refusal");
}

// ── PRE-3b (142.077-T) ──────────────────────────────────────────────────────

fn write_source(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src").join("lib.rs"),
        "pub fn alpha() -> u32 {\n    1\n}\n\npub fn beta() -> u32 {\n    2\n}\n",
    )
    .expect("write lib.rs");
}

/// Publish the next revision through PRE-2; `tamper` corrupts the sealed
/// database after sealing so the published digest no longer matches.
async fn publish_revision(root: &Path, id: &str, tamper: bool) -> u64 {
    let layout = read_server_layout(as_str(root)).expect("layout");
    let store = open_generation_store(&layout, true).expect("open store");
    let id = GenerationId::new(id).expect("id");
    let built = build_candidate_generation(&layout, &store, &id, &CodeGraphConfig::default())
        .await
        .expect("build");
    let sealed = seal_candidate_inventory(built).expect("seal");
    if tamper {
        let database = layout.generations_root.join(id.as_str()).join("engram.db");
        let mut bytes = fs::read(&database).expect("read database");
        bytes.push(0);
        fs::write(&database, bytes).expect("tamper database");
    }
    publish_sealed_candidate(&layout, &store, sealed, PRODUCER)
        .expect("publish")
        .value()
}

fn snapshot_if_present(dir: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    if dir.exists() {
        snapshot(dir)
    } else {
        BTreeMap::new()
    }
}

fn generations_root(root: &Path) -> PathBuf {
    root.join(".engram").join("generations")
}

async fn generation_of(daemon: &Daemon) -> Value {
    daemon.call("get_workspace_status").await["generation"].clone()
}

/// Poll `get_workspace_status.generation` until `done` holds.
async fn wait_for_generation(daemon: &Daemon, what: &str, done: impl Fn(&Value) -> bool) -> Value {
    let deadline = tokio::time::Instant::now() + ACTIVATION_TIMEOUT;
    loop {
        let generation = generation_of(daemon).await;
        if done(&generation) {
            return generation;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "Worker: 142.077-T: timed out waiting for {what}; last generation: {generation}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn assert_daemon_answers(daemon: &Daemon) {
    assert!(check_health(&daemon.endpoint).await, "_health stays ready");
    let status = daemon.call("get_daemon_status").await;
    assert!(
        status.get("generation").is_none(),
        "get_daemon_status carries no provenance: {status}"
    );
}

/// Scenario 1, activation part [064-T `AC2a`].
#[tokio::test]
async fn read_server_activates_a_published_revision_at_first_ready() {
    let (_guard, root) = git_fixture(Some("read_server"));
    write_source(&root);
    assert_eq!(publish_revision(&root, "cand-wire-1", false).await, 1);

    let daemon = Daemon::spawn(&root).await;
    let generation = generation_of(&daemon).await;
    assert!(
        !generation.is_null(),
        "Worker: 142.077-T: generation must be non-null at the first ready: {generation}"
    );
    wait_for_generation(&daemon, "active_revision == 1", |g| {
        g["active_revision"].as_u64() == Some(1)
    })
    .await;
    assert_daemon_answers(&daemon).await;
}

/// Scenario 2, no generations root [064-T `AC3a`].
#[tokio::test]
async fn read_server_without_a_root_stays_ready_and_creates_nothing() {
    let (_guard, root) = git_fixture(Some("read_server"));
    let daemon = Daemon::spawn(&root).await;

    assert!(generation_of(&daemon).await.is_null(), "null generation");
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(
        !generations_root(&root).exists(),
        "root must not be created"
    );
    assert_daemon_answers(&daemon).await;
}

/// Scenario 2, empty root through valid revision 2 [064-T `AC3a`].
#[tokio::test]
async fn read_server_progresses_from_empty_root_to_a_valid_revision() {
    let (_guard, root) = git_fixture(Some("read_server"));
    write_source(&root);
    fs::create_dir_all(generations_root(&root)).expect("create empty root");
    let daemon = Daemon::spawn(&root).await;

    let first = generation_of(&daemon).await;
    assert!(
        !first.is_null(),
        "Worker: 142.077-T: root present at startup installs the gate: {first}"
    );
    let before = snapshot(&generations_root(&root));
    tokio::time::sleep(QUIET_WINDOW).await;
    let quiet = generation_of(&daemon).await;
    assert!(quiet["active_revision"].is_null(), "no attempt: {quiet}");
    assert!(
        quiet["last_failed_revision"].is_null(),
        "no attempt: {quiet}"
    );
    assert_eq!(
        snapshot(&generations_root(&root)),
        before,
        "nothing written"
    );
    assert_daemon_answers(&daemon).await;

    assert_eq!(publish_revision(&root, "cand-wire-bad", true).await, 1);
    wait_for_generation(&daemon, "last_failed_revision == 1", |g| {
        g["last_failed_revision"].as_u64() == Some(1)
    })
    .await;
    let runtime = snapshot_if_present(&generations_root(&root).join("runtime"));
    tokio::time::sleep(QUIET_WINDOW).await;
    let still = generation_of(&daemon).await;
    assert_eq!(still["last_failed_revision"].as_u64(), Some(1), "{still}");
    assert!(still["active_revision"].is_null(), "{still}");
    assert_eq!(
        snapshot_if_present(&generations_root(&root).join("runtime")),
        runtime,
        "rejected revision is never re-attempted"
    );
    assert_daemon_answers(&daemon).await;

    assert_eq!(publish_revision(&root, "cand-wire-2", false).await, 2);
    wait_for_generation(&daemon, "active_revision == 2", |g| {
        g["active_revision"].as_u64() == Some(2)
    })
    .await;
    assert_daemon_answers(&daemon).await;
}

/// A root absent at startup is installed late, once it appears.
#[tokio::test]
async fn read_server_installs_late_when_the_root_was_absent_at_startup() {
    let (_guard, root) = git_fixture(Some("read_server"));
    write_source(&root);
    let daemon = Daemon::spawn(&root).await;
    assert!(generation_of(&daemon).await.is_null(), "null generation");

    assert_eq!(publish_revision(&root, "cand-wire-late", false).await, 1);
    wait_for_generation(&daemon, "late active_revision == 1", |g| {
        g["active_revision"].as_u64() == Some(1)
    })
    .await;
    assert_daemon_answers(&daemon).await;
}

/// P2-1 no-late-install [064-T AC5].
#[tokio::test]
async fn read_server_never_installs_late_after_a_startup_install_failure() {
    let (guard, root) = git_fixture(Some("read_server"));
    write_source(&root);
    let runtime = generations_root(&root).join("runtime");
    fs::create_dir_all(generations_root(&root)).expect("create root");
    let outside = guard.path().join("outside");
    fs::create_dir_all(&outside).expect("create outside");
    assert!(
        make_dir_link(&runtime, &outside),
        "cannot create the runtime directory link; this test cannot exercise the startup install failure"
    );

    let daemon = Daemon::spawn(&root).await;
    assert!(
        generation_of(&daemon).await.is_null(),
        "install failure leaves no gate"
    );

    #[cfg(unix)]
    fs::remove_file(&runtime).expect("remove link");
    #[cfg(windows)]
    fs::remove_dir(&runtime).expect("remove link");
    assert_eq!(publish_revision(&root, "cand-wire-final", false).await, 1);
    tokio::time::sleep(QUIET_WINDOW).await;
    let generation = generation_of(&daemon).await;
    assert!(
        generation.is_null(),
        "Worker: 142.077-T: no late install after a startup failure with the root present: {generation}"
    );
    assert_daemon_answers(&daemon).await;
}

// ── PRE-3c (142.064-T) ──────────────────────────────────────────────────────

/// Window within which a transiently failed revision must activate once its
/// missing inventory file is restored (064-T `AC3b`).
const TRANSIENT_RECOVERY_TIMEOUT: Duration = Duration::from_secs(10);

/// Path of the sealed generation database of the only published generation.
fn sealed_database(root: &Path, id: &str) -> PathBuf {
    generations_root(root).join(id).join("engram.db")
}

/// RED-phase precondition [064.003-ST D1]: an inventory file that is absent
/// is rated `Transient` by `classify`, so the retry scenario below really
/// exercises the transient path and not a permanent rejection.
#[tokio::test]
async fn an_absent_inventory_file_is_classified_transient() {
    use engram::services::generations::{GenerationActivator, RejectionClass, classify};

    let (_guard, root) = git_fixture(Some("read_server"));
    write_source(&root);
    assert_eq!(publish_revision(&root, "cand-wire-class", false).await, 1);
    let database = sealed_database(&root, "cand-wire-class");
    fs::remove_file(&database).expect("remove sealed database");

    let layout = read_server_layout(as_str(&root)).expect("layout");
    let store = open_generation_store(&layout, false).expect("open store");
    let activator = GenerationActivator::new(
        store,
        layout.runtime_root.clone(),
        layout.expected_identity(),
        layout.activation_deadline,
    );
    let error = activator
        .activate_initial()
        .await
        .expect_err("activation must fail while the inventory file is absent");
    assert_eq!(
        classify(&error),
        RejectionClass::Transient,
        "absent inventory file must be transient: {error}"
    );
}

/// Poll `get_workspace_status.generation` until `done` holds, failing with the
/// `Worker: 142.064-T` marker when `timeout` elapses first.
async fn wait_for_recovery(
    daemon: &Daemon,
    what: &str,
    timeout: Duration,
    done: impl Fn(&Value) -> bool,
) -> Value {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let generation = generation_of(daemon).await;
        if done(&generation) {
            return generation;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "Worker: 142.064-T: timed out waiting for {what}; last generation: {generation}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// Scenario 2, transient step [064-T `AC3b`; 064.003-ST D1, D2].
///
/// The inventory file is absent at spawn (a transient rejection), then
/// restored: the same revision must activate with no new revision published.
#[tokio::test]
async fn read_server_retries_a_transient_failure_of_the_same_revision() {
    let (_guard, root) = git_fixture(Some("read_server"));
    write_source(&root);
    assert_eq!(
        publish_revision(&root, "cand-wire-transient", false).await,
        1
    );
    let database = sealed_database(&root, "cand-wire-transient");
    let parked = database.with_extension("db.parked");
    fs::rename(&database, &parked).expect("park sealed database");

    let daemon = Daemon::spawn(&root).await;
    let failed = wait_for_recovery(
        &daemon,
        "last_failed_revision == 1",
        TRANSIENT_RECOVERY_TIMEOUT,
        |g| g["last_failed_revision"].as_u64() == Some(1),
    )
    .await;
    assert!(failed["active_revision"].is_null(), "{failed}");

    fs::rename(&parked, &database).expect("restore sealed database");
    let recovered = wait_for_recovery(
        &daemon,
        "active_revision == 1 after the inventory file is restored",
        TRANSIENT_RECOVERY_TIMEOUT,
        |g| g["active_revision"].as_u64() == Some(1),
    )
    .await;
    assert_eq!(
        recovered["active_revision"].as_u64(),
        Some(1),
        "no new revision is published: {recovered}"
    );
    assert_daemon_answers(&daemon).await;
}

/// Create a directory link (symlink, or junction on Windows). Returns `false`
/// with a logged reason when the link cannot be created; callers must fail loudly.
fn make_dir_link(link: &Path, target: &Path) -> bool {
    #[cfg(windows)]
    {
        match std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
        {
            Ok(out) if out.status.success() => true,
            other => {
                eprintln!("mklink /J failed: {other:?}");
                false
            }
        }
    }
    #[cfg(unix)]
    {
        match std::os::unix::fs::symlink(target, link) {
            Ok(()) => true,
            Err(e) => {
                eprintln!("symlink failed: {e}");
                false
            }
        }
    }
}

// ── PRE-4 (142.066-T) ───────────────────────────────────────────────────────

/// A filesystem snapshot entry: kind and length (or a recorded metadata error).
type FsSnapshot = BTreeMap<PathBuf, (String, u64)>;

/// Run `engram --workspace <root> --json <args>`; return exit code and the
/// parsed stdout envelope (`Value::Null` when stdout is not JSON).
async fn run_cli(root: &Path, args: &[&str]) -> (i32, Value) {
    let output = tokio::time::timeout(
        IPC_TIMEOUT * 2,
        Command::new(env!("CARGO_BIN_EXE_engram"))
            .args(["--workspace", as_str(root), "--json"])
            .args(args)
            .env_remove("ENGRAM_DATA_DIR")
            .env_remove("ENGRAM_DIRECT")
            .env_remove("ENGRAM_WORKSPACE")
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("engram CLI timed out")
    .expect("engram CLI must execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope = serde_json::from_str::<Value>(stdout.trim())
        .or_else(|_| serde_json::from_str::<Value>(stdout.trim().lines().last().unwrap_or("")))
        .unwrap_or(Value::Null);
    (output.status.code().unwrap_or(-1), envelope)
}

fn write_second_source(root: &Path) {
    fs::write(
        root.join("src").join("extra.rs"),
        "pub fn gamma() -> u32 {\n    3\n}\n",
    )
    .expect("write extra.rs");
}

fn fs_walk(base: &Path, dir: &Path, out: &mut FsSnapshot) {
    let rel = |path: &Path| path.strip_prefix(base).unwrap_or(path).to_path_buf();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            out.insert(
                rel(dir),
                (format!("read_dir_error:{:?}", error.raw_os_error()), 0),
            );
            return;
        }
    };
    for entry in entries {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(error) => {
                out.insert(
                    rel(dir).join("<entry>"),
                    (format!("entry_error:{:?}", error.raw_os_error()), 0),
                );
                continue;
            }
        };
        // Metadata only: no content read, so a Windows sharing violation
        // (error 33) on a live `.db` is recorded, never skipped.
        match fs::symlink_metadata(&path) {
            Ok(meta) => {
                let kind = if meta.is_dir() {
                    "dir"
                } else if meta.is_file() {
                    "file"
                } else {
                    "other"
                };
                out.insert(rel(&path), (kind.to_owned(), meta.len()));
                if meta.is_dir() {
                    fs_walk(base, &path, out);
                }
            }
            Err(error) => {
                out.insert(
                    rel(&path),
                    (format!("metadata_error:{:?}", error.raw_os_error()), 0),
                );
            }
        }
    }
}

/// Every entry's relative path, kind and length under `root`.
fn fs_snapshot(root: &Path) -> FsSnapshot {
    let mut out = BTreeMap::new();
    fs_walk(root, root, &mut out);
    out
}

/// F: the (`path`, `branch`, `db_path`, `generation`) tuple of one status response.
fn binding_fingerprint(status: &Value) -> Value {
    serde_json::json!([
        status["path"],
        status["branch"],
        status["db_path"],
        status["generation"]
    ])
}

/// Wait for the R7.7 two-observation settled state at revision 2.
async fn settle_at_revision_2(daemon: &Daemon, root: &Path) {
    let deadline = tokio::time::Instant::now() + ACTIVATION_TIMEOUT;
    let mut last = (Value::Null, Value::Null);
    loop {
        if let Some(status1) = daemon.try_status().await {
            let snapshot1 = fs_snapshot(root);
            tokio::time::sleep(Duration::from_millis(250)).await;
            let snapshot2 = fs_snapshot(root);
            if let Some(status2) = daemon.try_status().await {
                let (g1, g2) = (&status1["generation"], &status2["generation"]);
                last = (g1.clone(), g2.clone());
                let at_two = |g: &Value| {
                    g["active_revision"].as_u64() == Some(2)
                        && g["published_revision"].as_u64() == Some(2)
                };
                if at_two(g1)
                    && at_two(g2)
                    && snapshot1 == snapshot2
                    && binding_fingerprint(&status1) == binding_fingerprint(&status2)
                {
                    return;
                }
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "Worker: 142.066-T: revision 2 never settled; last G1: {}, last G2: {}",
            last.0,
            last.1
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// Scenario 1: a real CLI `stats` read is pinned to the published generation.
#[tokio::test]
async fn pinned_stats_read_reports_provenance_and_counts() {
    assert!(
        GENERATION_PINNED_READS.contains(&"get_workspace_statistics"),
        "Worker: 142.066-T GENERATION_PINNED_READS"
    );
    let (_guard, root) = git_fixture(Some("read_server"));
    write_source(&root);
    assert_eq!(publish_revision(&root, "cand-pin-1", false).await, 1);

    let daemon = Daemon::spawn(&root).await;
    wait_for_generation(&daemon, "active_revision == 1", |g| {
        g["active_revision"].as_u64() == Some(1)
    })
    .await;

    let (code, envelope) = run_cli(&root, &["stats"]).await;
    assert_eq!(code, 0, "stats must exit 0: {envelope}");
    let result = &envelope["result"];
    assert_eq!(
        result
            .pointer("/provenance/generation_id")
            .and_then(Value::as_str),
        Some("cand-pin-1"),
        "provenance generation id: {envelope}"
    );
    assert_eq!(result["code_files"].as_u64(), Some(1), "{envelope}");
}

/// Scenario 2: reconciliation at request entry picks up revision 2 and
/// quiesces afterwards.
#[tokio::test]
async fn pinned_stats_read_follows_reconciliation_to_revision_2() {
    assert!(
        GENERATION_PINNED_READS.contains(&"get_workspace_statistics"),
        "Worker: 142.066-T GENERATION_PINNED_READS"
    );
    // Static check: no descriptor declares a generation-control method.
    for descriptor in engram::tools::capabilities::all_descriptors() {
        let name = descriptor.name.to_ascii_lowercase();
        assert!(
            !["generation", "publish", "activate", "reconcile"]
                .iter()
                .any(|word| name.contains(word)),
            "descriptor {name} declares a generation-control method"
        );
    }

    let (_guard, root) = git_fixture(Some("read_server"));
    write_source(&root);
    assert_eq!(publish_revision(&root, "cand-pin-1", false).await, 1);
    let daemon = Daemon::spawn(&root).await;
    wait_for_generation(&daemon, "active_revision == 1", |g| {
        g["active_revision"].as_u64() == Some(1)
    })
    .await;

    write_second_source(&root);
    assert_eq!(publish_revision(&root, "cand-pin-2", false).await, 2);

    // Bounded re-probing: 50 ms doubling to 500 ms, 30 s limit.
    let deadline = tokio::time::Instant::now() + ACTIVATION_TIMEOUT;
    let mut delay = Duration::from_millis(50);
    loop {
        let (code, envelope) = run_cli(&root, &["stats"]).await;
        let result = &envelope["result"];
        if code == 0
            && result
                .pointer("/provenance/generation_id")
                .and_then(Value::as_str)
                == Some("cand-pin-2")
            && result["code_files"].as_u64() == Some(2)
        {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "Worker: 142.066-T: revision 2 never observed through stats; last: {envelope}"
        );
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(Duration::from_millis(500));
    }

    settle_at_revision_2(&daemon, &root).await;

    // Quiescence: warm-up read, then reads change nothing.
    let (code, _) = run_cli(&root, &["stats"]).await;
    assert_eq!(code, 0, "warm-up stats");
    let status_before = daemon.try_status().await.expect("status before");
    let fingerprint_before = binding_fingerprint(&status_before);
    let snapshot_before = fs_snapshot(&root);
    for _ in 0..3 {
        let (code, _) = run_cli(&root, &["stats"]).await;
        assert_eq!(code, 0, "quiescence stats");
    }
    tokio::time::sleep(QUIET_WINDOW).await;
    let status_after = daemon.try_status().await.expect("status after");
    assert_eq!(
        binding_fingerprint(&status_after),
        fingerprint_before,
        "binding fingerprint unchanged by pinned reads"
    );
    assert_eq!(
        fs_snapshot(&root),
        snapshot_before,
        "filesystem unchanged by pinned reads"
    );
}

/// Scenario 3: regression witness.
#[tokio::test]
async fn pinned_read_refusal_and_unpinned_paths_are_unchanged() {
    assert!(
        GENERATION_PINNED_READS.contains(&"get_workspace_statistics"),
        "Worker: 142.066-T GENERATION_PINNED_READS"
    );
    // Read-server daemon with no generations root: typed refusal for `stats`.
    let (_guard, root) = git_fixture(Some("read_server"));
    let daemon = Daemon::spawn(&root).await;
    let response = daemon.raw("get_workspace_statistics").await;
    let error = response.error.expect("stats must be refused");
    assert_eq!(
        error.data.as_ref().and_then(|d| d["engram_code"].as_u64()),
        Some(17_001),
        "typed not-yet-activated code: {error:?}"
    );
    for method in ["get_daemon_status", "get_workspace_status"] {
        let response = daemon.raw(method).await;
        assert!(response.error.is_none(), "{method} accepted: {response:?}");
        let result = response.result.unwrap_or(Value::Null);
        assert!(
            result.get("provenance").is_none(),
            "{method} carries no provenance: {result}"
        );
    }
    assert!(check_health(&daemon.endpoint).await, "_health stays ready");

    // Managed mode: `stats` is unchanged and carries no provenance.
    let (_managed_guard, managed_root) = git_fixture(None);
    let managed = Daemon::spawn(&managed_root).await;
    let response = managed.raw("get_workspace_statistics").await;
    if let Some(result) = &response.result {
        assert!(
            result.get("provenance").is_none(),
            "managed stats carries no provenance: {result}"
        );
    }
    if let Some(error) = &response.error {
        assert_ne!(
            error.data.as_ref().and_then(|d| d["engram_code"].as_u64()),
            Some(17_001),
            "managed stats is never refused as not-yet-activated: {error:?}"
        );
    }
}
