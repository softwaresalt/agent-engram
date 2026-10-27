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

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use engram::daemon::ipc_server::ipc_endpoint;
use engram::daemon::protocol::IpcRequest;
use engram::db::workspace::{canonicalize_workspace, workspace_hash};
use engram::services::generations::candidate_build::read_server_layout;
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::Value;
use tempfile::TempDir;
use tokio::process::{Child, Command};

const READY_TIMEOUT: Duration = Duration::from_secs(120);
const IPC_TIMEOUT: Duration = Duration::from_secs(30);
const REFUSAL_TIMEOUT: Duration = Duration::from_secs(120);
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
