//! Integration coverage for the daemon IPC error transport seam.
//!
//! Exercises a real daemon request/response round trip and verifies that a
//! structured domain error keeps its stable code, symbolic name, and details
//! when it crosses the IPC boundary.

#![forbid(unsafe_code)]

use std::fs;
use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::db::workspace::canonicalize_workspace;
use engram::errors::{EngramError, ReadServerRefusalError};
use engram::models::config::DaemonMode;
use engram::shim::ipc_client::send_request;
use serde_json::{Value, json};
use tempfile::TempDir;

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

fn init_git_workspace(path: &std::path::Path) {
    fs::create_dir_all(path.join(".git")).expect("create .git");
    fs::write(path.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write git HEAD");
}

fn write_mode_config(path: &std::path::Path, mode: DaemonMode) {
    let engram_dir = path.join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");
    fs::write(
        engram_dir.join("config.toml"),
        format!("mode = \"{}\"\n", mode.as_str()),
    )
    .expect("write config.toml");
}

fn request(id: i64, method: &str, params: Option<Value>) -> IpcRequest {
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(id))),
        method: method.to_owned(),
        params,
    }
}

#[tokio::test]
async fn read_server_set_workspace_refusal_preserves_structured_domain_error() {
    let active_workspace = TempDir::new().expect("active workspace tempdir");
    let requested_workspace = TempDir::new().expect("requested workspace tempdir");

    init_git_workspace(active_workspace.path());
    init_git_workspace(requested_workspace.path());
    write_mode_config(active_workspace.path(), DaemonMode::ReadServer);

    let requested_workspace_path =
        canonicalize_workspace(requested_workspace.path().to_string_lossy().as_ref())
            .expect("canonicalize requested workspace")
            .display()
            .to_string();

    let harness =
        DaemonHarness::spawn_for_workspace(active_workspace.path(), Duration::from_secs(15))
            .await
            .expect("daemon must spawn in read_server mode");
    let endpoint = harness
        .ipc_path()
        .to_str()
        .expect("UTF-8 IPC path")
        .to_owned();

    let response = send_request(
        &endpoint,
        &request(
            1,
            "set_workspace",
            Some(json!({ "path": requested_workspace_path })),
        ),
        Duration::from_secs(10),
    )
    .await
    .expect("IPC request must complete");

    assert!(
        response.result.is_none(),
        "error responses must not also contain a result: {response:?}"
    );
    let error = response
        .error
        .as_ref()
        .expect("read-server retarget must return an IPC error");

    let expected = EngramError::from(ReadServerRefusalError::WorkspaceRetargetRefused {
        requested_workspace: requested_workspace_path,
    })
    .to_response();

    assert_eq!(
        error.code, -32_603,
        "domain errors must use JSON-RPC internal error on the IPC wire"
    );
    assert_eq!(
        error.message, expected.error.message,
        "IPC error message must preserve the domain envelope message"
    );

    let data = error
        .data
        .as_ref()
        .expect("domain IPC errors must carry structured data");
    assert_eq!(
        data.get("engram_code"),
        Some(&json!(expected.error.code)),
        "stable Engram code must survive the daemon IPC round trip"
    );
    assert_eq!(
        data.get("engram_name"),
        Some(&json!(expected.error.name)),
        "symbolic Engram name must survive the daemon IPC round trip"
    );
    assert_eq!(
        data.get("engram_details"),
        expected.error.details.as_ref(),
        "structured Engram details must survive the daemon IPC round trip"
    );
}
