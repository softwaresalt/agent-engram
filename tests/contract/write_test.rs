use std::fs;
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use t_mem::errors::codes::{
    BATCH_PARTIAL_FAILURE, COMPACTION_FAILED, CYCLIC_DEPENDENCY, DUPLICATE_LABEL,
    INVALID_ISSUE_TYPE, INVALID_STATUS, LABEL_VALIDATION, TASK_ALREADY_CLAIMED, TASK_NOT_CLAIMABLE,
    TASK_NOT_FOUND, WORKSPACE_NOT_SET,
};
use t_mem::server::state::AppState;
use t_mem::tools;

#[test]
async fn contract_update_task_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "id": "task:abc123",
        "status": "in_progress",
    }));

    let err = tools::dispatch(state, "update_task", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_update_task_rejects_invalid_transition() {
    // Seed workspace with a completed task
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(
        tmem_dir.join("tasks.md"),
        r"# Tasks

## task:t1

---
id: task:t1
title: Finished task
status: done
created_at: 2026-02-05T10:00:00+00:00
updated_at: 2026-02-05T10:00:00+00:00
---

Task is already complete.
",
    )
    .expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let err = tools::dispatch(
        state.clone(),
        "update_task",
        Some(json!({ "id": "t1", "status": "blocked" })),
    )
    .await
    .expect_err("expected invalid status transition");

    let code = err.to_response().error.code;
    assert_eq!(code, INVALID_STATUS);
}

#[test]
async fn contract_add_blocker_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "task_id": "task:abc123",
        "reason": "waiting on review",
    }));

    let err = tools::dispatch(state, "add_blocker", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_register_decision_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "topic": "database backend",
        "decision": "use surrealdb",
    }));

    let err = tools::dispatch(state, "register_decision", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

// ─── T057: Contract test for flush_state ────────────────────────────────────

#[test]
async fn contract_flush_state_requires_workspace() {
    let state = Arc::new(AppState::new(10));

    let err = tools::dispatch(state, "flush_state", None)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_flush_state_response_shape() {
    // Set up a real workspace with .git/
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    std::fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    // Bind workspace
    let bind_result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");
    assert!(bind_result.get("workspace_id").is_some());

    // Call flush_state
    let result = tools::dispatch(state.clone(), "flush_state", None)
        .await
        .expect("flush_state should succeed");

    // Verify contract response shape
    let files = result.get("files_written").expect("files_written field");
    assert!(files.is_array(), "files_written should be array");
    let files_arr = files.as_array().unwrap();
    assert!(
        files_arr
            .iter()
            .any(|f| f.as_str() == Some(".tmem/tasks.md")),
        "should write tasks.md"
    );
    assert!(
        files_arr
            .iter()
            .any(|f| f.as_str() == Some(".tmem/.lastflush")),
        "should write .lastflush"
    );

    let warnings = result.get("warnings").expect("warnings field");
    assert!(warnings.is_array(), "warnings should be array");

    let ts = result
        .get("flush_timestamp")
        .expect("flush_timestamp field");
    assert!(ts.is_string(), "flush_timestamp should be string");

    // Verify files exist on disk
    let tmem_dir = workspace.path().join(".tmem");
    assert!(tmem_dir.join("tasks.md").exists(), "tasks.md on disk");
    assert!(tmem_dir.join("graph.surql").exists(), "graph.surql on disk");
    assert!(tmem_dir.join(".version").exists(), ".version on disk");
    assert!(tmem_dir.join(".lastflush").exists(), ".lastflush on disk");
}

// ─── T127: Contract test for work_item_id assignment and retrieval (FR-017) ──

#[test]
async fn contract_work_item_id_roundtrip_via_update_and_graph() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(
        tmem_dir.join("tasks.md"),
        r"# Tasks

## task:wi1

---
id: task:wi1
title: Linked task
status: todo
work_item_id: AB#12345
created_at: 2026-02-05T10:00:00+00:00
updated_at: 2026-02-05T10:00:00+00:00
---

Task linked to external work item.
",
    )
    .expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    // Update the task status — work_item_id should be preserved
    tools::dispatch(
        state.clone(),
        "update_task",
        Some(json!({ "id": "wi1", "status": "in_progress" })),
    )
    .await
    .expect("update_task should succeed");

    // Retrieve via get_task_graph — verify work_item_id is present
    let graph = tools::dispatch(
        state.clone(),
        "get_task_graph",
        Some(json!({ "root_task_id": "wi1" })),
    )
    .await
    .expect("get_task_graph should succeed");

    let root = graph.get("root").expect("root node");
    assert_eq!(
        root.get("status").and_then(|s| s.as_str()),
        Some("in_progress")
    );

    // Also verify via check_status with the work_item_id
    let status_result = tools::dispatch(
        state.clone(),
        "check_status",
        Some(json!({ "work_item_ids": ["AB#12345"] })),
    )
    .await
    .expect("check_status should succeed");

    let statuses = status_result.get("statuses").expect("statuses map");
    let entry = statuses.get("AB#12345").expect("AB#12345 entry");
    assert_eq!(
        entry.get("status").and_then(|s| s.as_str()),
        Some("in_progress")
    );
}

// ─── T129: Contract test for create_task WorkspaceNotSet (1003) ─────────────

#[test]
async fn contract_create_task_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "title": "New task",
    }));

    let err = tools::dispatch(state, "create_task", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

// ─── T130: Contract test for create_task empty title TaskTitleEmpty (3013) ───

#[test]
async fn contract_create_task_rejects_empty_title() {
    let workspace = tempfile::tempdir().expect("workspace");
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

    // Empty title
    let err = tools::dispatch(state.clone(), "create_task", Some(json!({ "title": "" })))
        .await
        .expect_err("expected TaskTitleEmpty error");

    let code = err.to_response().error.code;
    assert_eq!(code, t_mem::errors::codes::TASK_TITLE_EMPTY);
}

#[test]
async fn contract_create_task_rejects_oversized_title() {
    let workspace = tempfile::tempdir().expect("workspace");
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

    // Title > 200 chars
    let long_title = "a".repeat(201);
    let err = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": long_title })),
    )
    .await
    .expect_err("expected TaskTitleTooLong error for oversized title");

    let code = err.to_response().error.code;
    assert_eq!(code, t_mem::errors::codes::TASK_TITLE_TOO_LONG);
}

// ─── T026: Contract tests for add_label and remove_label ────────────────────

#[test]
async fn contract_add_label_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "task_id": "task:abc123",
        "label": "frontend",
    }));

    let err = tools::dispatch(state, "add_label", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_remove_label_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "task_id": "task:abc123",
        "label": "frontend",
    }));

    let err = tools::dispatch(state, "remove_label", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_add_label_returns_label_count() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    // Create a task first
    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Label test task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    // Add a label
    let result = tools::dispatch(
        state.clone(),
        "add_label",
        Some(json!({ "task_id": task_id, "label": "frontend" })),
    )
    .await
    .expect("add_label should succeed");

    assert_eq!(result["task_id"].as_str().unwrap(), task_id);
    assert_eq!(result["label"].as_str().unwrap(), "frontend");
    assert_eq!(result["label_count"].as_u64().unwrap(), 1);

    // Add a second label
    let result2 = tools::dispatch(
        state.clone(),
        "add_label",
        Some(json!({ "task_id": task_id, "label": "bug" })),
    )
    .await
    .expect("add_label second should succeed");

    assert_eq!(result2["label_count"].as_u64().unwrap(), 2);
}

#[test]
async fn contract_add_label_duplicate_returns_error() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Dup label task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    // Add label first time
    tools::dispatch(
        state.clone(),
        "add_label",
        Some(json!({ "task_id": task_id, "label": "frontend" })),
    )
    .await
    .expect("first add_label");

    // Add same label again → duplicate error
    let err = tools::dispatch(
        state.clone(),
        "add_label",
        Some(json!({ "task_id": task_id, "label": "frontend" })),
    )
    .await
    .expect_err("expected duplicate label error");

    let code = err.to_response().error.code;
    assert_eq!(code, DUPLICATE_LABEL);
}

#[test]
async fn contract_add_label_not_in_allowed_list_returns_error() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    // Write a .tmem/config.toml with allowed_labels
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(
        tmem_dir.join("config.toml"),
        r#"allowed_labels = ["frontend", "backend", "bug"]
"#,
    )
    .expect("write config.toml");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Restricted label task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    // Try adding a label not in allowed_labels
    let err = tools::dispatch(
        state.clone(),
        "add_label",
        Some(json!({ "task_id": task_id, "label": "invalid-label" })),
    )
    .await
    .expect_err("expected label validation error");

    let code = err.to_response().error.code;
    assert_eq!(code, LABEL_VALIDATION);
}

// ── T034: add_dependency contract tests ─────────────────────────

#[test]
async fn contract_add_dependency_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let err = tools::dispatch(
        state,
        "add_dependency",
        Some(json!({
            "from_task_id": "a",
            "to_task_id": "b",
            "dependency_type": "hard_blocker",
        })),
    )
    .await
    .expect_err("expected workspace_not_set");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_add_dependency_valid_types() {
    let workspace = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    // Create two tasks
    let t1 = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Task A" })),
    )
    .await
    .expect("create task A");
    let t2 = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Task B" })),
    )
    .await
    .expect("create task B");
    let id_a = t1["task_id"].as_str().unwrap().to_string();
    let id_b = t2["task_id"].as_str().unwrap().to_string();

    // Test all 8 dependency types
    let types = [
        "hard_blocker",
        "soft_dependency",
        "child_of",
        "blocked_by",
        "duplicate_of",
        "related_to",
        "predecessor",
        "successor",
    ];

    for dep_type in types {
        let result = tools::dispatch(
            state.clone(),
            "add_dependency",
            Some(json!({
                "from_task_id": id_a,
                "to_task_id": id_b,
                "dependency_type": dep_type,
            })),
        )
        .await
        .unwrap_or_else(|_| panic!("add_dependency {dep_type} should succeed"));

        assert_eq!(
            result["dependency_type"].as_str().unwrap(),
            dep_type,
            "returned type should match for {dep_type}"
        );
    }
}

#[test]
async fn contract_add_dependency_self_reference_rejected() {
    let workspace = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Self-ref task" })),
    )
    .await
    .expect("create task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    let err = tools::dispatch(
        state.clone(),
        "add_dependency",
        Some(json!({
            "from_task_id": task_id,
            "to_task_id": task_id,
            "dependency_type": "hard_blocker",
        })),
    )
    .await
    .expect_err("expected cyclic dependency error on self-reference");

    assert_eq!(err.to_response().error.code, CYCLIC_DEPENDENCY);
}

#[test]
async fn contract_add_dependency_cycle_rejected() {
    let workspace = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    // Create A, B, C
    let a = tools::dispatch(state.clone(), "create_task", Some(json!({ "title": "A" })))
        .await
        .expect("create A");
    let b = tools::dispatch(state.clone(), "create_task", Some(json!({ "title": "B" })))
        .await
        .expect("create B");
    let c = tools::dispatch(state.clone(), "create_task", Some(json!({ "title": "C" })))
        .await
        .expect("create C");

    let id_a = a["task_id"].as_str().unwrap().to_string();
    let id_b = b["task_id"].as_str().unwrap().to_string();
    let id_c = c["task_id"].as_str().unwrap().to_string();

    // A → B → C
    tools::dispatch(
        state.clone(),
        "add_dependency",
        Some(json!({
            "from_task_id": id_a,
            "to_task_id": id_b,
            "dependency_type": "hard_blocker",
        })),
    )
    .await
    .expect("A→B should succeed");

    tools::dispatch(
        state.clone(),
        "add_dependency",
        Some(json!({
            "from_task_id": id_b,
            "to_task_id": id_c,
            "dependency_type": "hard_blocker",
        })),
    )
    .await
    .expect("B→C should succeed");

    // C → A should be rejected (cycle: A → B → C → A)
    let err = tools::dispatch(
        state.clone(),
        "add_dependency",
        Some(json!({
            "from_task_id": id_c,
            "to_task_id": id_a,
            "dependency_type": "hard_blocker",
        })),
    )
    .await
    .expect_err("expected cyclic dependency error");

    assert_eq!(err.to_response().error.code, CYCLIC_DEPENDENCY);
}

// ── Compaction contract tests (T041) ───────────────────────────────────────

#[test]
async fn contract_apply_compaction_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "compactions": [{ "task_id": "task:abc", "summary": "compacted" }]
    }));

    let err = tools::dispatch(state, "apply_compaction", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_apply_compaction_nonexistent_task() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(tmem_dir.join("tasks.md"), "# Tasks\n").expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    let err = tools::dispatch(
        state,
        "apply_compaction",
        Some(json!({
            "compactions": [{ "task_id": "nonexistent-task", "summary": "compacted" }]
        })),
    )
    .await
    .expect_err("expected compaction failed error");

    assert_eq!(err.to_response().error.code, COMPACTION_FAILED);
}

// ── Claim/Release contract tests (T049) ────────────────────────────────────

#[test]
async fn contract_claim_task_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "task_id": "task:abc",
        "claimant": "agent-1"
    }));

    let err = tools::dispatch(state, "claim_task", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_release_task_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "task_id": "task:abc" }));

    let err = tools::dispatch(state, "release_task", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_claim_task_sets_assignee() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(tmem_dir.join("tasks.md"), "# Tasks\n").expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    // Create a task first
    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Claimable task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    // Claim the task
    let result = tools::dispatch(
        state.clone(),
        "claim_task",
        Some(json!({ "task_id": task_id, "claimant": "agent-1" })),
    )
    .await
    .expect("claim_task should succeed");

    assert_eq!(result["claimant"].as_str().unwrap(), "agent-1");
    assert!(result["context_id"].is_string());
    assert!(result["claimed_at"].is_string());
}

#[test]
async fn contract_claim_already_claimed_returns_error() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(tmem_dir.join("tasks.md"), "# Tasks\n").expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Claim conflict task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    // First claim succeeds
    tools::dispatch(
        state.clone(),
        "claim_task",
        Some(json!({ "task_id": task_id, "claimant": "agent-1" })),
    )
    .await
    .expect("first claim should succeed");

    // Second claim by different agent should fail with TASK_ALREADY_CLAIMED
    let err = tools::dispatch(
        state.clone(),
        "claim_task",
        Some(json!({ "task_id": task_id, "claimant": "agent-2" })),
    )
    .await
    .expect_err("second claim should fail");

    assert_eq!(err.to_response().error.code, TASK_ALREADY_CLAIMED);
}

#[test]
async fn contract_release_unclaimed_returns_error() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(tmem_dir.join("tasks.md"), "# Tasks\n").expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Unclaimed task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    // Release unclaimed task should fail with TASK_NOT_CLAIMABLE
    let err = tools::dispatch(
        state.clone(),
        "release_task",
        Some(json!({ "task_id": task_id })),
    )
    .await
    .expect_err("release unclaimed should fail");

    assert_eq!(err.to_response().error.code, TASK_NOT_CLAIMABLE);
}

#[test]
async fn contract_release_records_previous_claimant() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(tmem_dir.join("tasks.md"), "# Tasks\n").expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Release test task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    // Claim then release
    tools::dispatch(
        state.clone(),
        "claim_task",
        Some(json!({ "task_id": task_id, "claimant": "agent-1" })),
    )
    .await
    .expect("claim");

    let release_result = tools::dispatch(
        state.clone(),
        "release_task",
        Some(json!({ "task_id": task_id })),
    )
    .await
    .expect("release should succeed");

    assert_eq!(
        release_result["previous_claimant"].as_str().unwrap(),
        "agent-1"
    );
    assert!(release_result["context_id"].is_string());
    assert!(release_result["released_at"].is_string());
}

// ── T054: issue_type contract tests ─────────────────────────────

#[test]
async fn contract_update_task_issue_type_change_creates_context_note() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    // Create a task (defaults to issue_type "task")
    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Type change test" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    // Update issue_type to "bug"
    let result = tools::dispatch(
        state.clone(),
        "update_task",
        Some(json!({
            "id": task_id,
            "status": "todo",
            "issue_type": "bug",
        })),
    )
    .await
    .expect("update_task with issue_type");

    assert_eq!(result["task_id"].as_str().unwrap(), task_id);
    assert!(result["context_id"].is_string());
    assert!(result["updated_at"].is_string());
}

#[test]
async fn contract_update_task_invalid_issue_type_returns_error() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    // Write a .tmem/config.toml with allowed_types
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(
        tmem_dir.join("config.toml"),
        r#"allowed_types = ["task", "bug", "spike"]
"#,
    )
    .expect("write config.toml");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Invalid type test" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    // Try changing to a type not in allowed_types
    let err = tools::dispatch(
        state.clone(),
        "update_task",
        Some(json!({
            "id": task_id,
            "status": "todo",
            "issue_type": "epic",
        })),
    )
    .await
    .expect_err("expected invalid issue type error");

    let code = err.to_response().error.code;
    assert_eq!(code, INVALID_ISSUE_TYPE);
}

#[test]
async fn contract_create_task_with_issue_type() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    // Create a task with explicit issue_type "bug"
    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Bug report", "issue_type": "bug" })),
    )
    .await
    .expect("create_task with issue_type");

    let task_id = created["task_id"].as_str().unwrap().to_string();
    assert_eq!(created["issue_type"].as_str().unwrap(), "bug");

    // Verify via get_ready_work filter
    let ready = tools::dispatch(
        state.clone(),
        "get_ready_work",
        Some(json!({ "issue_type": "bug" })),
    )
    .await
    .expect("get_ready_work");

    let tasks = ready["tasks"].as_array().unwrap();
    assert!(tasks.iter().any(|t| t["id"].as_str().unwrap() == task_id));
}

#[test]
async fn contract_create_task_invalid_issue_type_returns_error() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    // Write a .tmem/config.toml with allowed_types
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(
        tmem_dir.join("config.toml"),
        r#"allowed_types = ["task", "bug", "spike"]
"#,
    )
    .expect("write config.toml");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    // Try creating with a type not in allowed_types
    let err = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Epic task", "issue_type": "epic" })),
    )
    .await
    .expect_err("expected invalid issue type error");

    let code = err.to_response().error.code;
    assert_eq!(code, INVALID_ISSUE_TYPE);
}

// ── T059: Defer/Pin contract tests ──────────────────────────────

#[test]
async fn contract_defer_task_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "task_id": "task:abc",
        "until": "2099-01-01T00:00:00Z"
    }));

    let err = tools::dispatch(state, "defer_task", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_undefer_task_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "task_id": "task:abc" }));

    let err = tools::dispatch(state, "undefer_task", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_pin_task_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "task_id": "task:abc" }));

    let err = tools::dispatch(state, "pin_task", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_unpin_task_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "task_id": "task:abc" }));

    let err = tools::dispatch(state, "unpin_task", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_defer_task_sets_defer_until_and_creates_note() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(tmem_dir.join("tasks.md"), "# Tasks\n").expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Deferrable task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    let defer_date = "2099-06-15T12:00:00Z";
    let result = tools::dispatch(
        state.clone(),
        "defer_task",
        Some(json!({ "task_id": task_id, "until": defer_date })),
    )
    .await
    .expect("defer_task should succeed");

    assert_eq!(result["task_id"].as_str().unwrap(), task_id);
    assert_eq!(result["defer_until"].as_str().unwrap(), defer_date);
    assert!(result["context_id"].is_string());
    assert!(result["updated_at"].is_string());
}

#[test]
async fn contract_undefer_task_clears_and_returns_previous() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(tmem_dir.join("tasks.md"), "# Tasks\n").expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Undeferrable task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    let defer_date = "2099-06-15T12:00:00Z";
    tools::dispatch(
        state.clone(),
        "defer_task",
        Some(json!({ "task_id": task_id, "until": defer_date })),
    )
    .await
    .expect("defer_task");

    let result = tools::dispatch(
        state.clone(),
        "undefer_task",
        Some(json!({ "task_id": task_id })),
    )
    .await
    .expect("undefer_task should succeed");

    assert_eq!(result["task_id"].as_str().unwrap(), task_id);
    assert!(
        result["previous_defer_until"].is_string(),
        "should return previous defer_until date"
    );
    assert!(result["context_id"].is_string());
}

#[test]
async fn contract_pin_task_sets_pinned_and_creates_note() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(tmem_dir.join("tasks.md"), "# Tasks\n").expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Pinnable task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    let result = tools::dispatch(
        state.clone(),
        "pin_task",
        Some(json!({ "task_id": task_id })),
    )
    .await
    .expect("pin_task should succeed");

    assert_eq!(result["task_id"].as_str().unwrap(), task_id);
    assert!(result["pinned"].as_bool().unwrap());
    assert!(result["context_id"].is_string());
    assert!(result["updated_at"].is_string());
}

#[test]
async fn contract_unpin_task_clears_pinned() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(tmem_dir.join("tasks.md"), "# Tasks\n").expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Unpinnable task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    // Pin first
    tools::dispatch(
        state.clone(),
        "pin_task",
        Some(json!({ "task_id": task_id })),
    )
    .await
    .expect("pin_task");

    // Unpin
    let result = tools::dispatch(
        state.clone(),
        "unpin_task",
        Some(json!({ "task_id": task_id })),
    )
    .await
    .expect("unpin_task should succeed");

    assert_eq!(result["task_id"].as_str().unwrap(), task_id);
    assert!(!result["pinned"].as_bool().unwrap());
    assert!(result["context_id"].is_string());
}

// ── Batch update contract tests (T073) ──────────────────────────────────

#[test]
async fn contract_batch_update_tasks_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "updates": [{ "id": "task:x", "status": "in_progress" }]
    }));

    let err = tools::dispatch(state, "batch_update_tasks", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_batch_update_tasks_returns_per_item_results() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    fs::create_dir_all(workspace.path().join(".tmem")).expect("create .tmem");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    // Create 3 tasks
    let mut task_ids = Vec::new();
    for i in 0..3 {
        let created = tools::dispatch(
            state.clone(),
            "create_task",
            Some(json!({ "title": format!("Batch task {i}") })),
        )
        .await
        .expect("create_task");
        task_ids.push(created["task_id"].as_str().unwrap().to_string());
    }

    // Batch update: 3 valid tasks to in_progress
    let result = tools::dispatch(
        state.clone(),
        "batch_update_tasks",
        Some(json!({
            "updates": [
                { "id": task_ids[0], "status": "in_progress" },
                { "id": task_ids[1], "status": "in_progress" },
                { "id": task_ids[2], "status": "in_progress" },
            ]
        })),
    )
    .await
    .expect("batch_update_tasks should succeed");

    assert_eq!(result["succeeded"].as_u64().unwrap(), 3);
    assert_eq!(result["failed"].as_u64().unwrap(), 0);

    let results = result["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    for item in results {
        assert!(item["success"].as_bool().unwrap());
        assert_eq!(item["previous_status"].as_str().unwrap(), "todo");
        assert_eq!(item["new_status"].as_str().unwrap(), "in_progress");
    }
}

#[test]
async fn contract_batch_update_with_invalid_id_returns_partial_failure() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    fs::create_dir_all(workspace.path().join(".tmem")).expect("create .tmem");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    // Create 1 valid task
    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Valid batch task" })),
    )
    .await
    .expect("create_task");
    let valid_id = created["task_id"].as_str().unwrap().to_string();

    // Batch with one valid + one invalid
    let err = tools::dispatch(
        state.clone(),
        "batch_update_tasks",
        Some(json!({
            "updates": [
                { "id": valid_id, "status": "in_progress" },
                { "id": "task:nonexistent", "status": "in_progress" },
            ]
        })),
    )
    .await
    .expect_err("batch with invalid ID should return partial failure error");

    assert_eq!(err.to_response().error.code, BATCH_PARTIAL_FAILURE);
}

// ── Comment contract tests (T073) ───────────────────────────────────────

#[test]
async fn contract_add_comment_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "task_id": "task:x",
        "content": "A comment",
        "author": "agent-1"
    }));

    let err = tools::dispatch(state, "add_comment", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_add_comment_returns_comment_id() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    fs::create_dir_all(workspace.path().join(".tmem")).expect("create .tmem");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    // Create a task
    let created = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Commentable task" })),
    )
    .await
    .expect("create_task");
    let task_id = created["task_id"].as_str().unwrap().to_string();

    // Add a comment
    let result = tools::dispatch(
        state.clone(),
        "add_comment",
        Some(json!({
            "task_id": task_id,
            "content": "This is a discussion comment",
            "author": "agent-1"
        })),
    )
    .await
    .expect("add_comment should succeed");

    assert!(result["comment_id"].is_string());
    assert_eq!(result["task_id"].as_str().unwrap(), task_id);
    assert_eq!(result["author"].as_str().unwrap(), "agent-1");
    assert!(result["created_at"].is_string());
}

#[test]
async fn contract_add_comment_nonexistent_task_fails() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    fs::create_dir_all(workspace.path().join(".tmem")).expect("create .tmem");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    let err = tools::dispatch(
        state.clone(),
        "add_comment",
        Some(json!({
            "task_id": "task:nonexistent",
            "content": "Ghost comment",
            "author": "agent-1"
        })),
    )
    .await
    .expect_err("add_comment on nonexistent task should fail");

    assert_eq!(err.to_response().error.code, TASK_NOT_FOUND);
}
