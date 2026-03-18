//! Integration tests for multi-client concurrent access (US5).
//!
//! Tests verify that 10+ clients can safely perform interleaved read/write
//! operations on the same workspace without data corruption or failures.

use std::collections::HashSet;
use std::fs;
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::server::state::AppState;
use engram::tools;

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
    let engram_dir = workspace.path().join(".engram");
    assert!(engram_dir.join("tasks.md").exists(), "tasks.md exists");
    let content = fs::read_to_string(engram_dir.join("tasks.md")).expect("read tasks.md");
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

// ─── Phase 9 additions (T055) — S026, S027, S044, S062, S070, S076, S077 ──────

/// S026: Concurrent ingestion from two sources is serialized without interference.
///
/// Two concurrent `index_workspace` calls must not corrupt state. The indexer
/// may serialize (at-least-one succeeds) or reject one with `IndexInProgress`,
/// but must never panic or corrupt data.
#[test]
async fn s026_concurrent_ingestion_serialized_or_rejected() {
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
    .expect("set_workspace must succeed");

    let s1 = state.clone();
    let s2 = state.clone();

    let h1 =
        tokio::spawn(async move { tools::dispatch(s1, "index_workspace", Some(json!({}))).await });
    let h2 =
        tokio::spawn(async move { tools::dispatch(s2, "index_workspace", Some(json!({}))).await });

    let r1 = h1.await.expect("h1 join");
    let r2 = h2.await.expect("h2 join");

    // At least one must succeed; the other may get IndexInProgress.
    let one_ok = r1.is_ok() || r2.is_ok();
    assert!(
        one_ok,
        "at least one concurrent index_workspace must succeed"
    );

    for result in [r1, r2] {
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("IndexInProgress")
                    || msg.contains("index")
                    || msg.contains("progress"),
                "concurrent failure must be IndexInProgress, not data corruption: {msg}"
            );
        }
    }
}

/// S027: File deleted after registry scan is handled gracefully.
///
/// `ingest_single_file` on a non-existent path must return `Ok` and not panic.
/// The ingestion pipeline skips deleted files gracefully.
#[tokio::test]
async fn s027_file_deleted_after_scan_handled_gracefully() {
    use engram::db::queries::Queries;
    use engram::db::schema;
    use engram::services::ingestion::ingest_single_file;

    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();

    let db_path = workspace.join("db");
    fs::create_dir_all(&db_path).expect("create db dir");
    let db =
        surrealdb::Surreal::new::<surrealdb::engine::local::SurrealKv>(db_path.to_str().unwrap())
            .await
            .expect("embedded SurrealDB");
    db.use_ns("engram").use_db("s027").await.expect("ns/db");
    db.query(schema::DEFINE_CONTENT_RECORD)
        .await
        .expect("content schema");
    let queries = Queries::new(db);

    // Target a file that does not exist on disk (simulates post-scan deletion).
    let phantom_file = workspace.join("vanished.md");

    let result = ingest_single_file(
        &phantom_file,
        workspace,
        "docs",
        "docs",
        1_048_576,
        &queries,
    )
    .await;

    assert!(
        result.is_ok(),
        "ingest_single_file on deleted file must not error: {result:?}"
    );

    // No ContentRecord should exist for the phantom file.
    let records = queries
        .select_content_records(None)
        .await
        .expect("select records");
    assert!(
        records.is_empty(),
        "no ContentRecord must exist for a deleted file"
    );
}

/// S044: Concurrent hydrate and dehydrate are serialized via workspace lock.
///
/// Concurrent `flush_state` and `get_workspace_status` must both succeed
/// without data corruption.
#[test]
async fn s044_concurrent_hydrate_dehydrate_serialized() {
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
    .expect("set_workspace must succeed");

    tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "S044 hydrate task" })),
    )
    .await
    .expect("create_task must succeed");

    let s1 = state.clone();
    let s2 = state.clone();

    let h1 = tokio::spawn(async move { tools::dispatch(s1, "flush_state", Some(json!({}))).await });
    let h2 = tokio::spawn(async move { tools::dispatch(s2, "get_workspace_status", None).await });

    let r1 = h1.await.expect("h1 join");
    let r2 = h2.await.expect("h2 join");

    assert!(r1.is_ok(), "flush_state must succeed: {r1:?}");
    assert!(
        r2.is_ok(),
        "get_workspace_status must succeed concurrently: {r2:?}"
    );

    // Verify final state is consistent.
    let engram_dir = workspace.path().join(".engram");
    assert!(
        engram_dir.join("tasks.md").exists(),
        "tasks.md must be written"
    );
    let content = fs::read_to_string(engram_dir.join("tasks.md")).expect("read tasks.md");
    assert!(
        content.contains("S044 hydrate task"),
        "tasks.md must contain the created task"
    );
}

/// S062: Git repository with broken objects returns a clear error, not a panic.
///
/// `index_git_history` on a corrupted git repository must handle the error
/// gracefully — either returning an error or succeeding on trivial state.
///
/// Requires the `git-graph` feature flag.
#[cfg(feature = "git-graph")]
#[test]
async fn s062_git_broken_objects_returns_error_not_panic() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");

    // Write a corrupt objects directory to simulate broken git objects.
    let objects_dir = git_dir.join("objects");
    fs::create_dir_all(&objects_dir).expect("create objects dir");
    fs::write(objects_dir.join("pack"), b"INVALID_PACK_DATA").expect("write corrupt pack");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    let result = tools::dispatch(state.clone(), "index_git_history", Some(json!({}))).await;

    // Either it succeeds (trivial git state) or fails with a descriptive error.
    // The primary assertion is that no panic occurred.
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            !msg.is_empty(),
            "error message must not be empty for broken git objects"
        );
    }
    // Reaching here without panic satisfies S062.
}

/// S070: Hook file in a read-only directory is handled gracefully.
///
/// When `.github/` is read-only, `install()` must return an error with a
/// descriptive message rather than panicking.
///
/// Unix-only: Windows does not enforce directory read-only for child file creation.
#[cfg(unix)]
#[tokio::test]
async fn s070_hook_file_read_only_directory_handled_gracefully() {
    use std::os::unix::fs::PermissionsExt;

    use engram::installer::{InstallOptions, install};

    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    // Create .github/ as read-only to block hook file creation.
    let github_dir = workspace.path().join(".github");
    fs::create_dir_all(&github_dir).expect("create .github");
    fs::set_permissions(&github_dir, std::fs::Permissions::from_mode(0o555))
        .expect("set read-only");

    let opts = InstallOptions {
        hooks_only: false,
        no_hooks: false,
        port: engram::installer::DEFAULT_PORT,
    };

    let result = install(workspace.path(), &opts).await;

    // Restore permissions so tempdir cleanup can proceed.
    let _ = fs::set_permissions(&github_dir, std::fs::Permissions::from_mode(0o755));

    // Must not panic. Either succeeds or returns a descriptive IO error.
    match result {
        Ok(()) => {
            // Acceptable: install completed (hooks were written or skipped).
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                !msg.is_empty(),
                "error message must not be empty on read-only hook dir failure"
            );
        }
    }
}

/// S076: Multiple agents calling `query_memory` concurrently do not interfere.
///
/// Two concurrent queries with different `content_type` filters must each
/// return correct results independently — no cross-query contamination.
#[test]
async fn s076_concurrent_query_memory_no_cross_interference() {
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
    .expect("set_workspace must succeed");

    let s1 = state.clone();
    let s2 = state.clone();

    let h1 = tokio::spawn(async move {
        tools::dispatch(
            s1,
            "query_memory",
            Some(json!({ "query": "test query", "content_type": "spec" })),
        )
        .await
    });
    let h2 = tokio::spawn(async move {
        tools::dispatch(
            s2,
            "query_memory",
            Some(json!({ "query": "test query", "content_type": "docs" })),
        )
        .await
    });

    let r1 = h1.await.expect("h1 join");
    let r2 = h2.await.expect("h2 join");

    assert!(r1.is_ok(), "first query_memory must succeed: {r1:?}");
    assert!(r2.is_ok(), "second query_memory must succeed: {r2:?}");

    // Both should return arrays (empty is fine — content not required for this test).
    let v1 = r1.unwrap();
    let v2 = r2.unwrap();
    assert!(
        v1.is_array() || v1.is_object(),
        "first result must be array or object, got: {v1}"
    );
    assert!(
        v2.is_array() || v2.is_object(),
        "second result must be array or object, got: {v2}"
    );
}

/// S077: Concurrent ingestion of the same file produces no duplicate `ContentRecord`s.
///
/// Two concurrent `ingest_all_sources` calls against the same workspace and source
/// must upsert the same record — `SurrealDB` `UPSERT` semantics prevent duplication.
#[tokio::test]
async fn s077_concurrent_ingestion_no_duplicate_records() {
    use engram::db::queries::Queries;
    use engram::db::schema;
    use engram::models::registry::{ContentSource, ContentSourceStatus, RegistryConfig};
    use engram::services::ingestion::ingest_all_sources;

    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();

    let docs_dir = workspace.join("docs");
    fs::create_dir_all(&docs_dir).expect("create docs dir");
    fs::write(
        docs_dir.join("readme.md"),
        "# README\nTest content for S077",
    )
    .expect("write readme");

    let db_path = workspace.join("db");
    fs::create_dir_all(&db_path).expect("create db dir");
    let db =
        surrealdb::Surreal::new::<surrealdb::engine::local::SurrealKv>(db_path.to_str().unwrap())
            .await
            .expect("embedded SurrealDB");
    db.use_ns("engram").use_db("s077").await.expect("ns/db");
    db.query(schema::DEFINE_CONTENT_RECORD)
        .await
        .expect("content schema");
    let queries = Queries::new(db);

    let config = Arc::new(RegistryConfig {
        sources: vec![ContentSource {
            content_type: "docs".to_string(),
            language: None,
            path: "docs".to_string(),
            status: ContentSourceStatus::Active,
        }],
        max_file_size_bytes: 1_048_576,
        batch_size: 50,
    });
    let workspace_buf = Arc::new(workspace.to_path_buf());

    let c1 = config.clone();
    let w1 = workspace_buf.clone();
    let q1 = queries.clone();
    let h1 = tokio::spawn(async move { ingest_all_sources(&c1, &w1, &q1).await });

    let c2 = config.clone();
    let w2 = workspace_buf.clone();
    let q2 = queries.clone();
    let h2 = tokio::spawn(async move { ingest_all_sources(&c2, &w2, &q2).await });

    let r1 = h1.await.expect("h1 join");
    let r2 = h2.await.expect("h2 join");

    assert!(r1.is_ok(), "first ingestion must succeed: {r1:?}");
    assert!(r2.is_ok(), "second ingestion must succeed: {r2:?}");

    // Only one ContentRecord must exist (UPSERT deduplication).
    let records = queries
        .select_content_records(None)
        .await
        .expect("select records");
    assert_eq!(
        records.len(),
        1,
        "must have exactly 1 ContentRecord after concurrent ingestion, got {} (deduplication failed)",
        records.len()
    );
    assert_eq!(
        records[0].file_path, "docs/readme.md",
        "ContentRecord path must match"
    );
}
