//! End-to-end daemon test for graph and vector rehydration across restart.
//!
//! Proves that:
//! 1. A daemon can index a real workspace and flush `.engram/code-graph/*.jsonl`
//! 2. After the embedded `SurrealDB` files are deleted, a restarted daemon
//!    rehydrates graph state from JSONL
//! 3. Persisted non-zero embeddings in `nodes.jsonl` are restored into the DB,
//!    making vector coverage visible through the IPC tool surface

use std::fs;
use std::time::{Duration, Instant};

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::{Value, json};

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

fn make_request(id: i64, method: &str, params: Option<Value>) -> IpcRequest {
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(id))),
        method: method.to_owned(),
        params,
    }
}

async fn send_ok(endpoint: &str, id: i64, method: &str, params: Option<Value>) -> Value {
    let request = make_request(id, method, params);
    let response = send_request(endpoint, &request, Duration::from_secs(20))
        .await
        .unwrap_or_else(|e| panic!("{method} IPC call failed: {e}"));

    assert!(
        response.error.is_none(),
        "{method} returned an error: {:?}",
        response.error
    );

    response
        .result
        .unwrap_or_else(|| panic!("{method} response missing result field"))
}

async fn shutdown_and_wait(endpoint: &str) {
    let request = make_request(900, "_shutdown", None);
    let response = send_request(endpoint, &request, Duration::from_secs(5))
        .await
        .expect("_shutdown IPC must succeed");

    let result = response
        .result
        .as_ref()
        .expect("_shutdown must return a result");
    assert_eq!(
        result["status"], "shutting_down",
        "_shutdown must report shutting_down status"
    );

    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if !check_health(endpoint).await {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "daemon must stop responding within 8s of _shutdown"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn inject_meaningful_embedding(
    workspace_path: &std::path::Path,
    symbol_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let nodes_path = workspace_path
        .join(".engram")
        .join("code-graph")
        .join("nodes.jsonl");
    let content = fs::read_to_string(&nodes_path)?;
    let mut mutated = false;
    let embedding: Vec<f32> = std::iter::once(1.0_f32)
        .chain(std::iter::repeat_n(0.0_f32, 383))
        .collect();

    let updated_lines: Vec<String> = content
        .lines()
        .map(|line| {
            let mut value: Value =
                serde_json::from_str(line).unwrap_or_else(|e| panic!("invalid JSONL line: {e}"));
            if value["type"] == "function" && value["name"] == symbol_name {
                value["embedding"] =
                    serde_json::to_value(&embedding).expect("embedding vector must serialize");
                mutated = true;
            }
            serde_json::to_string(&value).expect("JSONL line must serialize")
        })
        .collect();

    assert!(
        mutated,
        "nodes.jsonl must contain function symbol `{symbol_name}`"
    );

    let updated = format!("{}\n", updated_lines.join("\n"));
    fs::write(nodes_path, updated)?;
    Ok(())
}

fn create_sample_workspace() -> tempfile::TempDir {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let workspace_path = workspace.path();
    fs::create_dir_all(workspace_path.join(".git")).expect("create .git");
    fs::write(
        workspace_path.join(".git").join("HEAD"),
        "ref: refs/heads/main\n",
    )
    .expect("write HEAD");
    fs::create_dir_all(workspace_path.join("src")).expect("create src");
    fs::write(
        workspace_path.join("src").join("lib.rs"),
        r"
pub fn hydrated_vector_anchor() -> i32 {
    42
}

pub fn helper_neighbor() -> i32 {
    hydrated_vector_anchor()
}
",
    )
    .expect("write sample source");

    workspace
}

async fn index_flush_and_seed_embedding(workspace_path: &std::path::Path) {
    let harness1 = DaemonHarness::spawn_for_workspace(workspace_path, Duration::from_secs(15))
        .await
        .expect("daemon 1 must spawn");
    let endpoint1 = harness1.ipc_path().to_str().expect("UTF-8").to_owned();

    let index_result = send_ok(&endpoint1, 1, "index_workspace", Some(json!({}))).await;
    assert!(
        index_result["files_parsed"].as_u64().unwrap_or(0) >= 1,
        "index_workspace must parse at least one Rust file: {index_result}"
    );

    let status_before = send_ok(&endpoint1, 2, "get_workspace_status", None).await;
    assert!(
        status_before["code_graph"]["functions"]
            .as_u64()
            .unwrap_or(0)
            >= 2,
        "indexed workspace must report function count before restart: {status_before}"
    );

    let flush_result = send_ok(&endpoint1, 3, "flush_state", Some(json!({}))).await;
    assert!(
        flush_result["files_written"]
            .as_array()
            .is_some_and(|files| !files.is_empty()),
        "flush_state must write code graph files: {flush_result}"
    );

    let nodes_path = workspace_path
        .join(".engram")
        .join("code-graph")
        .join("nodes.jsonl");
    assert!(nodes_path.exists(), "nodes.jsonl must exist after flush");

    // Inject a non-zero embedding into persisted JSONL so restart must restore
    // vector coverage from disk, independent of model download availability.
    inject_meaningful_embedding(workspace_path, "hydrated_vector_anchor")
        .expect("embedding injection into nodes.jsonl");

    shutdown_and_wait(&endpoint1).await;
}

fn delete_embedded_db_dir(workspace_path: &std::path::Path) {
    let db_dir = workspace_path.join(".engram").join("db").join("main");
    if db_dir.exists() {
        fs::remove_dir_all(&db_dir).expect("remove embedded DB directory");
    }
}

async fn assert_rehydrated_graph_and_vector_state(workspace_path: &std::path::Path) {
    let harness2 = DaemonHarness::spawn_for_workspace(workspace_path, Duration::from_secs(15))
        .await
        .expect("daemon 2 must spawn");
    let endpoint2 = harness2.ipc_path().to_str().expect("UTF-8").to_owned();

    let status_after = send_ok(&endpoint2, 4, "get_workspace_status", None).await;
    assert!(
        status_after["code_graph"]["functions"]
            .as_u64()
            .unwrap_or(0)
            >= 2,
        "restarted daemon must report rehydrated function count: {status_after}"
    );
    assert!(
        status_after["code_graph"]["edges"].as_u64().unwrap_or(0) >= 2,
        "restarted daemon must report rehydrated edge count: {status_after}"
    );

    let map_result = send_ok(
        &endpoint2,
        5,
        "map_code",
        Some(json!({ "symbol_name": "hydrated_vector_anchor" })),
    )
    .await;
    assert_eq!(
        map_result["root"]["name"], "hydrated_vector_anchor",
        "map_code must find the rehydrated symbol after restart"
    );

    let stats_result = send_ok(&endpoint2, 6, "get_workspace_statistics", None).await;
    assert!(
        stats_result["functions"].as_u64().unwrap_or(0) >= 2,
        "workspace statistics must report rehydrated symbols: {stats_result}"
    );
    assert!(
        stats_result["embedding_status"]["symbols_with_embeddings"]
            .as_u64()
            .unwrap_or(0)
            >= 1,
        "vector hydration must restore at least one non-zero embedding: {stats_result}"
    );
    assert!(
        stats_result["embedding_status"]["coverage_percent"]
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "vector hydration must report positive coverage after restart: {stats_result}"
    );
}

#[tokio::test]
async fn daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted() {
    // GIVEN a real workspace with Rust code and git metadata
    let workspace = create_sample_workspace();
    let workspace_path = workspace.path();

    // WHEN daemon 1 indexes and flushes the workspace
    index_flush_and_seed_embedding(workspace_path).await;

    // Delete the embedded DB directory so daemon 2 cannot reuse prior DB state.
    delete_embedded_db_dir(workspace_path);

    // THEN daemon 2 must rehydrate graph + vector state from JSONL artifacts.
    assert_rehydrated_graph_and_vector_state(workspace_path).await;
}
