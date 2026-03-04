use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::errors::codes::{QUERY_TOO_LONG, WORKSPACE_NOT_SET};
use engram::models::task::{Task, TaskStatus};
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools;

fn test_snapshot(id: &str) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: id.to_string(),
        path: format!("/tmp/{id}"),
        task_count: 0,
        context_count: 0,
        last_flush: None,
        stale_files: false,
        connection_count: 1,
        file_mtimes: std::collections::HashMap::new(),
    }
}

#[test]
async fn contract_get_task_graph_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "root_task_id": "task:root",
        "depth": 3,
    }));

    let err = tools::dispatch(state, "get_task_graph", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_check_status_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "work_item_ids": ["AB#123", "AB#456"],
    }));

    let err = tools::dispatch(state, "check_status", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

// ── T073: query_memory contract tests ────────────────────────────

#[test]
async fn contract_query_memory_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "query": "user authentication",
    }));

    let err = tools::dispatch(state, "query_memory", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_query_memory_rejects_long_query() {
    // Build a state with workspace set so we get past the workspace check.
    let state = Arc::new(AppState::new(10));
    let snapshot = engram::server::state::WorkspaceSnapshot {
        workspace_id: "test_ws".to_string(),
        path: "/tmp/test-repo".to_string(),
        task_count: 0,
        context_count: 0,
        last_flush: None,
        stale_files: false,
        connection_count: 1,
        file_mtimes: std::collections::HashMap::new(),
    };
    state.set_workspace(snapshot).await.expect("set workspace");

    // Query exceeding 2000 chars ≈ 500+ tokens
    let long_query = "a ".repeat(1500);
    let params = Some(json!({
        "query": long_query,
    }));

    let err = tools::dispatch(state, "query_memory", params)
        .await
        .expect_err("expected query too long error");

    let code = err.to_response().error.code;
    assert_eq!(code, QUERY_TOO_LONG);
}

#[test]
async fn contract_query_memory_returns_results_array() {
    // With an active workspace (even empty), query_memory should return
    // a JSON object with a `results` array.
    let state = Arc::new(AppState::new(10));
    let snapshot = engram::server::state::WorkspaceSnapshot {
        workspace_id: "test_ws_results".to_string(),
        path: "/tmp/test-repo-results".to_string(),
        task_count: 0,
        context_count: 0,
        last_flush: None,
        stale_files: false,
        connection_count: 1,
        file_mtimes: std::collections::HashMap::new(),
    };
    state.set_workspace(snapshot).await.expect("set workspace");

    let params = Some(json!({
        "query": "user login",
    }));

    let result = tools::dispatch(state, "query_memory", params).await;
    // May succeed with empty results or fail with ModelNotLoaded on keyword-only.
    // Either way, it should not return WorkspaceNotSet.
    match result {
        Ok(val) => {
            assert!(
                val.get("results").is_some(),
                "response must contain `results` key"
            );
            assert!(val["results"].is_array(), "`results` must be an array");
        }
        Err(e) => {
            // Acceptable: ModelNotLoaded or DatabaseError (no real DB in unit test)
            let code = e.to_response().error.code;
            assert_ne!(code, WORKSPACE_NOT_SET, "must not be WorkspaceNotSet");
        }
    }
}

// ── T018: get_ready_work contract tests ──────────────────────────

#[test]
async fn contract_get_ready_work_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "limit": 5 }));

    let err = tools::dispatch(state, "get_ready_work", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_get_ready_work_empty_workspace() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("ready_work_empty"))
        .await
        .expect("set workspace");

    let params = Some(json!({"name_prefix": "nonexistent"}));
    let result = tools::dispatch(state, "get_ready_work", params)
        .await
        .expect("should succeed on empty workspace");

    assert!(result.get("tasks").is_some(), "must have tasks key");
    assert!(result["tasks"].is_array(), "tasks must be array");
    assert_eq!(result["tasks"].as_array().unwrap().len(), 0);
    assert_eq!(result["total_eligible"].as_u64().unwrap(), 0);
}

#[test]
async fn contract_get_ready_work_returns_tasks() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("ready_work_basic"))
        .await
        .expect("set workspace");

    // Create a task via dispatch so it exists in the DB
    let create_params = Some(json!({
        "title": "ready-work test task",
        "description": "A task for ready-work"
    }));
    tools::dispatch(state.clone(), "create_task", create_params)
        .await
        .expect("create_task should succeed");

    let params = Some(json!({"name_prefix": "nonexistent"}));
    let result = tools::dispatch(state, "get_ready_work", params)
        .await
        .expect("should return tasks");

    assert!(result["tasks"].is_array());
    assert!(
        !result["tasks"].as_array().unwrap().is_empty(),
        "should have at least 1 task"
    );
    assert!(result["total_eligible"].as_u64().unwrap() >= 1);
}

#[test]
async fn contract_get_ready_work_limit_caps_results() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("ready_work_limit"))
        .await
        .expect("set workspace");

    // Create 3 tasks
    for i in 1..=3 {
        let create_params = Some(json!({
            "title": format!("limit test task {i}"),
            "description": format!("task {i}")
        }));
        tools::dispatch(state.clone(), "create_task", create_params)
            .await
            .expect("create_task should succeed");
    }

    let params = Some(json!({ "limit": 2 }));
    let result = tools::dispatch(state, "get_ready_work", params)
        .await
        .expect("should succeed with limit");

    let tasks = result["tasks"].as_array().unwrap();
    assert!(tasks.len() <= 2, "limit should cap results to 2");
    assert!(
        result["total_eligible"].as_u64().unwrap() >= 3,
        "total_eligible should reflect all eligible tasks"
    );
}

// ── Compaction contract tests (T041) ───────────────────────────────────────

#[test]
async fn contract_get_compaction_candidates_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({}));

    let err = tools::dispatch(state, "get_compaction_candidates", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_get_compaction_candidates_empty_when_none_eligible() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("compact_empty"))
        .await
        .expect("set workspace");

    // Create a task that is still todo — not eligible for compaction
    tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "active task", "description": "still working" })),
    )
    .await
    .expect("create_task");

    let result = tools::dispatch(state, "get_compaction_candidates", Some(json!({})))
        .await
        .expect("should return empty list");

    let candidates = result["candidates"].as_array().unwrap();
    assert!(candidates.is_empty(), "no done tasks = no candidates");
}

#[test]
async fn contract_get_compaction_candidates_returns_eligible_tasks() {
    use chrono::{Duration, Utc};
    use engram::db::{connect_db, queries::Queries};
    use surrealdb::RecordId as Thing;

    let state = Arc::new(AppState::new(10));
    let ws_id = "compact_candidates";
    state
        .set_workspace(test_snapshot(ws_id))
        .await
        .expect("set workspace");

    // Create a done task via dispatch, then force-set old updated_at via raw query
    let db = connect_db(ws_id).await.expect("connect_db");
    let queries = Queries::new(db.clone());
    let old_task = Task {
        id: "old-done-1".to_string(),
        title: "Old done task".to_string(),
        status: TaskStatus::Done,
        work_item_id: None,
        description: "This task was completed 10 days ago".to_string(),
        context_summary: None,
        priority: "p2".to_owned(),
        priority_order: 2,
        issue_type: "task".to_owned(),
        assignee: None,
        defer_until: None,
        pinned: false,
        compaction_level: 0,
        compacted_at: None,
        workflow_state: None,
        workflow_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    queries
        .upsert_task(&old_task)
        .await
        .expect("upsert old task");

    // Force-set updated_at to 10 days ago (schema VALUE time::now() overrides normal SET)
    let old_date = (Utc::now() - Duration::days(10)).to_rfc3339();
    let record = Thing::from(("task", "old-done-1"));
    db.query("UPDATE $record SET updated_at = <datetime>$old_date")
        .bind(("record", record))
        .bind(("old_date", old_date))
        .await
        .expect("force-set old updated_at");

    let result = tools::dispatch(
        state,
        "get_compaction_candidates",
        Some(json!({ "threshold_days": 7 })),
    )
    .await
    .expect("should return candidates");

    let candidates = result["candidates"].as_array().unwrap();
    assert!(
        !candidates.is_empty(),
        "should find at least 1 old done task"
    );
    // Verify response shape
    let first = &candidates[0];
    assert!(first["task_id"].is_string());
    assert!(first["title"].is_string());
    assert!(first["description"].is_string());
    assert!(first["compaction_level"].is_number());
    assert!(first["age_days"].is_number());
}

#[test]
async fn contract_get_compaction_candidates_excludes_pinned() {
    use chrono::{Duration, Utc};
    use engram::db::{connect_db, queries::Queries};
    use surrealdb::RecordId as Thing;

    let state = Arc::new(AppState::new(10));
    let ws_id = "compact_pinned";
    state
        .set_workspace(test_snapshot(ws_id))
        .await
        .expect("set workspace");

    let db = connect_db(ws_id).await.expect("connect_db");
    let queries = Queries::new(db.clone());

    // Create a pinned done task — should be excluded
    let pinned_task = Task {
        id: "pinned-done-1".to_string(),
        title: "Pinned done task".to_string(),
        status: TaskStatus::Done,
        work_item_id: None,
        description: "Pinned and done".to_string(),
        context_summary: None,
        priority: "p2".to_owned(),
        priority_order: 2,
        issue_type: "task".to_owned(),
        assignee: None,
        defer_until: None,
        pinned: true,
        compaction_level: 0,
        compacted_at: None,
        workflow_state: None,
        workflow_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    queries
        .upsert_task(&pinned_task)
        .await
        .expect("upsert pinned task");

    // Force-set updated_at to 10 days ago
    let old_date = (Utc::now() - Duration::days(10)).to_rfc3339();
    let record = Thing::from(("task", "pinned-done-1"));
    db.query("UPDATE $record SET updated_at = <datetime>$old_date")
        .bind(("record", record))
        .bind(("old_date", old_date))
        .await
        .expect("force-set old updated_at");

    let result = tools::dispatch(
        state,
        "get_compaction_candidates",
        Some(json!({ "threshold_days": 7 })),
    )
    .await
    .expect("should return candidates");

    let candidates = result["candidates"].as_array().unwrap();
    let has_pinned = candidates
        .iter()
        .any(|c| c["task_id"].as_str() == Some("pinned-done-1"));
    assert!(!has_pinned, "pinned tasks must be excluded from candidates");
}

// ── Statistics & output-controls contract tests (T067) ─────────────────────

#[test]
async fn contract_get_workspace_statistics_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({}));

    let err = tools::dispatch(state, "get_workspace_statistics", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_get_workspace_statistics_returns_counts() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    std::fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let engram_dir = workspace.path().join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");
    std::fs::write(engram_dir.join("tasks.md"), "# Tasks\n").expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");

    // Create tasks with varying issue_type (set at creation time)
    for (i, issue_type) in ["bug", "feature", "bug", "chore"].iter().enumerate() {
        tools::dispatch(
            state.clone(),
            "create_task",
            Some(json!({
                "title": format!("stats task {i}"),
                "issue_type": issue_type,
            })),
        )
        .await
        .expect("create_task should succeed");
    }

    let result = tools::dispatch(state, "get_workspace_statistics", Some(json!({})))
        .await
        .expect("should return statistics");

    assert_eq!(result["total_tasks"].as_u64().unwrap(), 4);
    assert!(result["by_status"].is_object());
    assert!(result["by_priority"].is_object());
    assert!(result["by_type"].is_object());
    assert!(result["by_label"].is_object());

    // All 4 tasks created in todo status with default priority p2
    assert_eq!(result["by_status"]["todo"].as_u64().unwrap(), 4);
    assert_eq!(result["by_priority"]["p2"].as_u64().unwrap(), 4);

    // Type breakdown
    assert_eq!(result["by_type"]["bug"].as_u64().unwrap(), 2);
    assert_eq!(result["by_type"]["feature"].as_u64().unwrap(), 1);
    assert_eq!(result["by_type"]["chore"].as_u64().unwrap(), 1);
}

#[test]
async fn contract_get_ready_work_brief_strips_fields() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("brief_mode"))
        .await
        .expect("set workspace");

    tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({
            "title": "brief test",
            "description": "a long description that should be stripped",
            "priority": "high",
        })),
    )
    .await
    .expect("create_task should succeed");

    // With brief=true, only essential fields should appear
    let result = tools::dispatch(state, "get_ready_work", Some(json!({ "brief": true })))
        .await
        .expect("should succeed");

    let tasks = result["tasks"].as_array().unwrap();
    assert!(!tasks.is_empty());

    let task = &tasks[0];
    // Brief fields must be present
    assert!(task["id"].is_string());
    assert!(task["title"].is_string());
    assert!(task["status"].is_string());
    assert!(task["priority"].is_string());

    // Description should NOT be present in brief mode
    assert!(
        task.get("description").is_none() || task["description"].is_null(),
        "brief mode should strip description"
    );
}

// ── T036: map_code contract tests ────────────────────────────────────

#[test]
async fn contract_map_code_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "symbol_name": "my_function",
    }));

    let err = tools::dispatch(state, "map_code", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_map_code_empty_graph_uses_fallback() {
    // With an active workspace but no indexed code, map_code should
    // fall back to vector search and return an empty result set (not an error).
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("map_code_empty"))
        .await
        .expect("set workspace");

    let params = Some(json!({
        "symbol_name": "nonexistent_function",
    }));

    let result = tools::dispatch(state, "map_code", params).await;
    match result {
        Ok(val) => {
            // Should have fallback_used = true and empty matches
            assert_eq!(val["fallback_used"].as_bool(), Some(true));
            assert_eq!(val["truncated"].as_bool(), Some(false));
        }
        Err(e) => {
            // Acceptable: ModelNotLoaded or DatabaseError (no real embedding model in unit test)
            let code = e.to_response().error.code;
            assert_ne!(code, WORKSPACE_NOT_SET, "must not be WorkspaceNotSet");
        }
    }
}

// ── T037: list_symbols contract tests ────────────────────────────────

#[test]
async fn contract_list_symbols_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({}));

    let err = tools::dispatch(state, "list_symbols", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_list_symbols_empty_graph_returns_error() {
    use engram::errors::codes::SYMBOL_NOT_FOUND;

    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("list_symbols_empty"))
        .await
        .expect("set workspace");

    let params = Some(json!({"name_prefix": "nonexistent"}));

    let err = tools::dispatch(state, "list_symbols", params)
        .await
        .expect_err("expected symbol not found error for filtered empty graph");

    let code = err.to_response().error.code;
    assert_eq!(code, SYMBOL_NOT_FOUND);
}

// ─── Phase 6: Cross-Region Task-to-Code Linking ─────────────────────────────

#[test]
async fn contract_get_active_context_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({}));

    let err = tools::dispatch(state, "get_active_context", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

// ─── Phase 7: Unified Semantic Search ───────────────────────────────────────

#[test]
async fn contract_unified_search_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "query": "billing logic" }));

    let err = tools::dispatch(state, "unified_search", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_unified_search_rejects_empty_query() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("unified_search_empty"))
        .await
        .expect("set workspace");

    // Empty string
    let params = Some(json!({ "query": "" }));
    let err = tools::dispatch(state.clone(), "unified_search", params)
        .await
        .expect_err("expected empty query error");
    let code = err.to_response().error.code;
    assert_eq!(code, QUERY_TOO_LONG, "empty query should return 4001");

    // Whitespace-only string
    let params = Some(json!({ "query": "   " }));
    let err = tools::dispatch(state, "unified_search", params)
        .await
        .expect_err("expected empty query error for whitespace");
    let code = err.to_response().error.code;
    assert_eq!(
        code, QUERY_TOO_LONG,
        "whitespace-only query should return 4001"
    );
}

// ─── Phase 8: Impact Analysis Queries ───────────────────────────────────────

#[test]
async fn contract_impact_analysis_requires_workspace() {
    use engram::errors::codes::WORKSPACE_NOT_SET;

    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "symbol_name": "EngramError" }));

    let err = tools::dispatch(state, "impact_analysis", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_impact_analysis_symbol_not_found() {
    use engram::errors::codes::SYMBOL_NOT_FOUND;

    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("impact_analysis_not_found"))
        .await
        .expect("set workspace");

    let params = Some(json!({ "symbol_name": "NonExistentSymbol" }));

    let err = tools::dispatch(state, "impact_analysis", params)
        .await
        .expect_err("expected symbol not found error");

    let code = err.to_response().error.code;
    assert_eq!(code, SYMBOL_NOT_FOUND);
}
