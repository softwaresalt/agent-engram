//! RED local-daemon integration harness for HCL indexing and sync (121.005-T).

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use engram::daemon::protocol::{IpcRequest, IpcResponse};
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::{Value, json};

const READY_TIMEOUT: Duration = Duration::from_secs(15);
const IPC_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_TIMEOUT: Duration = Duration::from_secs(5);
const INDEX_IN_PROGRESS: i64 = 7003;

struct LocalDaemon {
    child: Child,
    endpoint: String,
}

impl LocalDaemon {
    async fn spawn(workspace: &Path) -> Self {
        let endpoint = engram::daemon::ipc_server::ipc_endpoint(workspace)
            .expect("derive isolated daemon endpoint");
        let binary = daemon_binary();
        let mut child = Command::new(&binary)
            .args(["daemon", "--workspace"])
            .arg(workspace)
            .env_remove("ENGRAM_DATA_DIR")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|error| panic!("spawn {}: {error}", binary.display()));

        let deadline = Instant::now() + READY_TIMEOUT;
        while !check_health(&endpoint).await {
            if let Some(status) = child.try_wait().expect("query daemon status") {
                panic!("daemon exited before readiness: {status}");
            }
            assert!(
                Instant::now() < deadline,
                "daemon did not become healthy within {READY_TIMEOUT:?}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Self { child, endpoint }
    }

    async fn shutdown(&mut self) {
        let response = send_request(
            &self.endpoint,
            &request(900, "_shutdown", None),
            IPC_TIMEOUT,
        )
        .await
        .expect("send bounded daemon shutdown");
        assert!(
            response.error.is_none(),
            "daemon shutdown returned an error: {:?}",
            response.error
        );

        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            if self.child.try_wait().expect("query daemon exit").is_some() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "daemon did not exit within the shutdown budget"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

impl Drop for LocalDaemon {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn daemon_binary() -> PathBuf {
    let current = std::env::current_exe().expect("resolve harness executable");
    let directory = current.parent().expect("harness executable parent");
    let direct = directory.join(format!("engram{}", std::env::consts::EXE_SUFFIX));
    if direct.is_file() {
        return direct;
    }
    directory
        .parent()
        .expect("Cargo test dependency directory parent")
        .join(format!("engram{}", std::env::consts::EXE_SUFFIX))
}

fn request(id: i64, method: &str, params: Option<Value>) -> IpcRequest {
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(id)),
        method: method.to_owned(),
        params,
    }
}

fn write_file(workspace: &Path, relative: &str, contents: &str) {
    let path = workspace.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create daemon fixture parent");
    }
    std::fs::write(path, contents).expect("write daemon fixture");
}

fn mixed_workspace() -> tempfile::TempDir {
    let workspace = tempfile::tempdir().expect("create isolated daemon workspace");
    write_file(workspace.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write_file(
        workspace.path(),
        ".engram/config.toml",
        "[code_graph]\nsupported_languages = [\"hcl\"]\n",
    );
    write_file(
        workspace.path(),
        "infra/main.tf",
        "resource \"aws_instance\" \"web\" {\n  region = var.region\n}\n",
    );
    write_file(
        workspace.path(),
        "infra/values.tfvars",
        "region = \"us-west-2\"\n",
    );
    write_file(
        workspace.path(),
        "infra/service.hcl",
        "service \"api\" {\n  endpoint = module.vpc.id\n}\n",
    );
    workspace
}

fn error_code(response: &IpcResponse) -> Option<i64> {
    response
        .error
        .as_ref()
        .and_then(|error| error.data.as_ref())
        .and_then(|data| data["engram_code"].as_i64())
}

async fn send_success(endpoint: &str, id: i64, method: &str, params: Option<Value>) -> Value {
    let response = send_request(endpoint, &request(id, method, params), IPC_TIMEOUT)
        .await
        .unwrap_or_else(|error| panic!("{method} IPC transport failed: {error}"));
    assert!(
        response.error.is_none(),
        "{method} returned an IPC error: {:?}",
        response.error
    );
    response
        .result
        .unwrap_or_else(|| panic!("{method} response omitted result"))
}

async fn send_index_operation(
    endpoint: &str,
    id: i64,
    method: &str,
    params: Option<Value>,
) -> Value {
    let deadline = Instant::now() + POLL_TIMEOUT;
    loop {
        let response = send_request(endpoint, &request(id, method, params.clone()), IPC_TIMEOUT)
            .await
            .unwrap_or_else(|error| panic!("{method} IPC transport failed: {error}"));
        if error_code(&response) == Some(INDEX_IN_PROGRESS) {
            assert!(
                Instant::now() < deadline,
                "{method} remained busy beyond {POLL_TIMEOUT:?}"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }
        assert!(
            response.error.is_none(),
            "{method} returned an IPC error: {:?}",
            response.error
        );
        return response
            .result
            .unwrap_or_else(|| panic!("{method} response omitted result"));
    }
}

async fn poll_for_symbols(endpoint: &str, expected: &[&str], marker: &str) -> Vec<Value> {
    let deadline = Instant::now() + POLL_TIMEOUT;
    let mut last_symbols = Vec::new();
    loop {
        let response = send_request(
            endpoint,
            &request(10, "list_symbols", Some(json!({ "limit": 50 }))),
            IPC_TIMEOUT,
        )
        .await
        .expect("list_symbols IPC transport");
        if response.error.is_none() {
            last_symbols = response
                .result
                .as_ref()
                .and_then(|result| result["symbols"].as_array())
                .cloned()
                .unwrap_or_default();
            let names: Vec<&str> = last_symbols
                .iter()
                .filter_map(|symbol| symbol["name"].as_str())
                .collect();
            if expected.iter().all(|name| names.contains(name)) {
                return last_symbols;
            }
        }

        assert!(
            Instant::now() < deadline,
            "RED:{marker} timed out waiting for {expected:?}; last symbols={last_symbols:?}, error={:?}",
            response.error
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn cold_start_lists_and_maps_all_three_hcl_aliases() {
    let workspace = mixed_workspace();
    let daemon = LocalDaemon::spawn(workspace.path()).await;
    let expected = [
        "hcl.block.resource.aws_instance.web",
        "hcl.attribute.region",
        "hcl.block.service.api",
    ];
    let _ = poll_for_symbols(&daemon.endpoint, &expected, "HCL_DAEMON_COLD_START_MISSING").await;

    let mapped = send_success(
        &daemon.endpoint,
        11,
        "map_code",
        Some(json!({ "symbol_name": expected[0] })),
    )
    .await;
    assert_eq!(mapped["root"]["name"], expected[0]);
    assert_eq!(mapped["fallback_used"], false);
}

#[tokio::test]
async fn modified_hcl_file_and_explicit_sync_replace_symbols_without_duplicates() {
    let workspace = mixed_workspace();
    let daemon = LocalDaemon::spawn(workspace.path()).await;
    let initial = send_index_operation(
        &daemon.endpoint,
        20,
        "index_workspace",
        Some(json!({ "force": true })),
    )
    .await;
    assert_eq!(
        initial["files_parsed"], 3,
        "RED:HCL_DAEMON_INDEX_MISSING expected all aliases; result={initial}"
    );
    assert_eq!(initial["errors"], json!([]));

    write_file(
        workspace.path(),
        "infra/main.tf",
        "resource \"aws_instance\" \"web_v2\" {\n  region = var.region\n}\n",
    );
    let synced =
        send_index_operation(&daemon.endpoint, 21, "sync_workspace", Some(json!({}))).await;
    assert_eq!(synced["files_modified"], 1);
    assert_eq!(synced["errors"], json!([]));

    let symbols = poll_for_symbols(
        &daemon.endpoint,
        &["hcl.block.resource.aws_instance.web_v2"],
        "HCL_DAEMON_SYNC_MISSING",
    )
    .await;
    let names: Vec<&str> = symbols
        .iter()
        .filter_map(|symbol| symbol["name"].as_str())
        .collect();
    assert_eq!(
        names
            .iter()
            .filter(|name| **name == "hcl.block.resource.aws_instance.web_v2")
            .count(),
        1
    );
    assert!(!names.contains(&"hcl.block.resource.aws_instance.web"));
}

#[tokio::test]
async fn malformed_hcl_stays_bounded_and_a_restart_remains_healthy() {
    let workspace = mixed_workspace();
    write_file(
        workspace.path(),
        "infra/malformed.tf",
        "resource \"aws_instance\" \"broken\" {\n  ami = var.\n",
    );

    let mut first = LocalDaemon::spawn(workspace.path()).await;
    let indexed = send_index_operation(
        &first.endpoint,
        30,
        "index_workspace",
        Some(json!({ "force": true })),
    )
    .await;
    assert!(
        indexed["files_parsed"].as_u64().unwrap_or(0) >= 3,
        "RED:HCL_DAEMON_MALFORMED_PATH_MISSING valid aliases were not indexed: {indexed}"
    );
    assert!(
        indexed["errors"]
            .as_array()
            .is_some_and(|errors| errors.len() <= 1),
        "malformed HCL errors must be per-file and bounded: {indexed}"
    );
    assert!(check_health(&first.endpoint).await);
    first.shutdown().await;
    drop(first);

    let restarted = LocalDaemon::spawn(workspace.path()).await;
    assert!(check_health(&restarted.endpoint).await);
    let symbols = poll_for_symbols(
        &restarted.endpoint,
        &["hcl.block.resource.aws_instance.web"],
        "HCL_DAEMON_RESTART_MISSING",
    )
    .await;
    let matches = symbols
        .iter()
        .filter(|symbol| symbol["name"] == "hcl.block.resource.aws_instance.web")
        .count();
    assert_eq!(matches, 1, "restart must not duplicate HCL symbols");
}
