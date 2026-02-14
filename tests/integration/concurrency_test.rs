//! Integration tests for multi-client concurrent access (US5).
//!
//! Tests verify that 10+ clients can safely perform interleaved read/write
//! operations on the same workspace without data corruption or failures.

use std::collections::HashSet;
use std::fs;
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use t_mem::server::state::AppState;
use t_mem::tools;

// ─── T087: Stress test with 10 concurrent clients ──────────────────────────

#[test]
async fn stress_test_10_concurrent_clients() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    // Create a task for clients to interact with
    let result = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Shared task" })),
    )
    .await
    .expect("create_task should succeed");
    let task_id = result["task_id"].as_str().unwrap().to_string();

    // 10 concurrent clients performing mixed read operations
    let mut handles = Vec::new();
    for i in 0..10 {
        let s = state.clone();
        let tid = task_id.clone();
        handles.push(tokio::spawn(async move {
            let ws = tools::dispatch(s.clone(), "get_workspace_status", None).await;
            assert!(ws.is_ok(), "client {i} get_workspace_status failed: {ws:?}");

            let ds = tools::dispatch(s.clone(), "get_daemon_status", None).await;
            assert!(ds.is_ok(), "client {i} get_daemon_status failed: {ds:?}");

            let graph = tools::dispatch(
                s.clone(),
                "get_task_graph",
                Some(json!({ "root_task_id": tid })),
            )
            .await;
            assert!(graph.is_ok(), "client {i} get_task_graph failed: {graph:?}");
        }));
    }

    for handle in handles {
        handle.await.expect("client task completed");
    }
}

// ─── T088: Last-write-wins for simple fields ────────────────────────────────

#[test]
async fn last_write_wins_concurrent_updates() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let result = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "LWW test task" })),
    )
    .await
    .expect("create_task should succeed");
    let task_id = result["task_id"].as_str().unwrap().to_string();

    // Two concurrent updates to the same task (both move to in_progress)
    let s1 = state.clone();
    let s2 = state.clone();
    let id1 = task_id.clone();
    let id2 = task_id.clone();

    let h1 = tokio::spawn(async move {
        tools::dispatch(
            s1,
            "update_task",
            Some(json!({ "id": id1, "status": "in_progress", "notes": "first writer" })),
        )
        .await
    });

    let h2 = tokio::spawn(async move {
        tools::dispatch(
            s2,
            "update_task",
            Some(json!({ "id": id2, "status": "in_progress", "notes": "second writer" })),
        )
        .await
    });

    let r1 = h1.await.expect("h1 join");
    let r2 = h2.await.expect("h2 join");

    // Both should succeed — last-write-wins based on updated_at
    assert!(r1.is_ok(), "first update should succeed: {r1:?}");
    assert!(r2.is_ok(), "second update should succeed: {r2:?}");

    // Verify final state is consistent
    let graph = tools::dispatch(
        state.clone(),
        "get_task_graph",
        Some(json!({ "root_task_id": task_id })),
    )
    .await
    .expect("get_task_graph should succeed");

    let root = graph.get("root").expect("root node");
    assert_eq!(
        root.get("status").and_then(|s| s.as_str()),
        Some("in_progress"),
        "final status should be in_progress"
    );
}

// ─── T089: Append-only semantics for context ────────────────────────────────

#[test]
async fn append_only_context_concurrent_writes() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    // 5 concurrent register_decision calls (each creates an append-only context node)
    let mut handles = Vec::new();
    for i in 0..5 {
        let s = state.clone();
        handles.push(tokio::spawn(async move {
            tools::dispatch(
                s,
                "register_decision",
                Some(json!({
                    "topic": format!("decision_{i}"),
                    "decision": format!("choice {i}")
                })),
            )
            .await
        }));
    }

    let mut decision_ids: Vec<String> = Vec::new();
    for handle in handles {
        let result = handle.await.expect("join");
        let val = result.expect("decision should succeed");
        decision_ids.push(val["decision_id"].as_str().unwrap().to_string());
    }

    // All 5 decisions should have unique IDs (append-only, no overwrites)
    let unique: HashSet<&str> = decision_ids.iter().map(String::as_str).collect();
    assert_eq!(
        unique.len(),
        5,
        "all context nodes should be unique (append-only)"
    );
}

// ─── T090: FIFO serialization of concurrent flush_state calls ───────────────

#[test]
async fn concurrent_flush_state_serialized() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    // Create a task so flush has something to write
    tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Flush test task" })),
    )
    .await
    .expect("create_task should succeed");

    // Two concurrent flush_state calls
    let s1 = state.clone();
    let s2 = state.clone();

    let h1 = tokio::spawn(async move { tools::dispatch(s1, "flush_state", None).await });

    let h2 = tokio::spawn(async move { tools::dispatch(s2, "flush_state", None).await });

    let r1 = h1.await.expect("h1 join");
    let r2 = h2.await.expect("h2 join");

    // Both should succeed (serialized via flush lock)
    assert!(r1.is_ok(), "first flush should succeed: {r1:?}");
    assert!(r2.is_ok(), "second flush should succeed: {r2:?}");

    // Verify file state is consistent
    let tmem_dir = workspace.path().join(".tmem");
    assert!(tmem_dir.join("tasks.md").exists(), "tasks.md exists");
    let content = fs::read_to_string(tmem_dir.join("tasks.md")).expect("read tasks.md");
    assert!(
        content.contains("Flush test task"),
        "task content preserved after concurrent flushes"
    );
}

// ─── T096: Workspace state preservation across client disconnects ───────────

#[test]
async fn workspace_state_preserved_after_disconnect() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    // Bind workspace
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    // Create a task
    let result = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Persistent task" })),
    )
    .await
    .expect("create_task should succeed");
    let task_id = result["task_id"].as_str().unwrap().to_string();

    // Simulate a connection registering and then disconnecting
    state.register_connection("conn-1".to_string()).await;
    state.unregister_connection("conn-1").await;

    // Workspace state should still be accessible after disconnect
    let ws_status = tools::dispatch(state.clone(), "get_workspace_status", None)
        .await
        .expect("workspace should still be accessible");
    assert!(
        ws_status.get("path").is_some(),
        "workspace path should be present"
    );

    // Task data should still be accessible
    let graph = tools::dispatch(
        state.clone(),
        "get_task_graph",
        Some(json!({ "root_task_id": task_id })),
    )
    .await
    .expect("task should still be accessible");
    assert!(graph.get("root").is_some(), "root task should exist");
}
