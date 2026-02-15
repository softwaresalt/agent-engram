//! Integration tests for enhanced task management features.
//!
//! Tests the full lifecycle of priority-based ready-work queue,
//! labels, enhanced dependencies, compaction, claiming, issue
//! types, defer/pin, output controls, batch operations, and
//! comments.

use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;

use t_mem::db::connect_db;
use t_mem::db::queries::Queries;
use t_mem::models::graph::DependencyType;
use t_mem::models::task::{Task, TaskStatus};
use t_mem::server::state::{AppState, WorkspaceSnapshot};
use t_mem::tools;

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

fn make_task(id: &str, priority: &str, priority_order: u32) -> Task {
    let now = Utc::now();
    Task {
        id: id.to_string(),
        title: format!("Task {id}"),
        status: TaskStatus::Todo,
        work_item_id: None,
        description: format!("Description for {id}"),
        context_summary: None,
        priority: priority.to_string(),
        priority_order,
        issue_type: "task".to_owned(),
        assignee: None,
        defer_until: None,
        pinned: false,
        compaction_level: 0,
        compacted_at: None,
        workflow_state: None,
        workflow_id: None,
        created_at: now,
        updated_at: now,
    }
}

// ── T025: get_ready_work integration test ───────────────────────

#[tokio::test]
async fn t025_ready_work_20_tasks_with_blocking_defer_and_pin() {
    // Setup: create workspace and DB with unique ID to avoid stale data
    let state = Arc::new(AppState::new(10));
    let ws_id = format!("ready_work_{}", uuid::Uuid::new_v4());
    state
        .set_workspace(test_snapshot(&ws_id))
        .await
        .expect("set workspace");

    let db = connect_db(&ws_id).await.expect("connect db");
    let queries = Queries::new(db);

    // Create 20 tasks across priority levels p0–p4
    // Tasks 1-4: p0 (priority_order=0)
    // Tasks 5-8: p1 (priority_order=1)
    // Tasks 9-12: p2 (priority_order=2)
    // Tasks 13-16: p3 (priority_order=3)
    // Tasks 17-20: p4 (priority_order=4)
    let priorities = [("p0", 0u32), ("p1", 1), ("p2", 2), ("p3", 3), ("p4", 4)];

    for (group_idx, (priority, order)) in priorities.iter().enumerate() {
        for task_idx in 0..4 {
            let id = format!("t{}", group_idx * 4 + task_idx + 1);
            let task = make_task(&id, priority, *order);
            queries.upsert_task(&task).await.expect("upsert task");
        }
    }

    // Block 5 tasks (t2, t6, t10, t14, t18) with hard_blocker from t20
    // t20 is in 'todo' state, so these should be excluded
    for blocked_id in &["t2", "t6", "t10", "t14", "t18"] {
        queries
            .create_dependency(blocked_id, "t20", DependencyType::HardBlocker)
            .await
            .expect("create blocker");
    }

    // Defer 3 tasks (t3, t7, t11) to tomorrow
    let tomorrow = Utc::now() + Duration::days(1);
    for deferred_id in &["t3", "t7", "t11"] {
        let mut task = queries
            .get_task(deferred_id)
            .await
            .expect("get task")
            .unwrap();
        task.defer_until = Some(tomorrow);
        task.updated_at = Utc::now();
        queries.upsert_task(&task).await.expect("update deferred");
    }

    // Pin 1 low-priority task (t17, p4) — should appear first in results
    let mut pinned_task = queries.get_task("t17").await.expect("get task").unwrap();
    pinned_task.pinned = true;
    pinned_task.updated_at = Utc::now();
    queries
        .upsert_task(&pinned_task)
        .await
        .expect("update pinned");

    // Expected eligible tasks (20 total minus 5 blocked minus 3 deferred = 12):
    // t1, t4 (p0), t5, t8 (p1), t9, t12 (p2), t13, t16 (p3), t17*, t19, t20 (p4)
    // Wait — t20 is the blocker but is itself in todo state, so t20 IS eligible
    // t17 is pinned, so it should be first

    // Call get_ready_work with no filters
    let params = Some(json!({}));
    let result = tools::dispatch(state.clone(), "get_ready_work", params)
        .await
        .expect("get_ready_work should succeed");

    let tasks = result["tasks"].as_array().unwrap();
    let total = result["total_eligible"].as_u64().unwrap();

    // Should have 12 eligible tasks (default limit is 10, so only 10 returned)
    assert_eq!(total, 12, "should have 12 eligible tasks");
    assert_eq!(tasks.len(), 10, "default limit should cap at 10");

    // First task should be the pinned one (t17)
    assert_eq!(
        tasks[0]["id"].as_str().unwrap(),
        "t17",
        "pinned task should be first"
    );
    assert!(tasks[0]["pinned"].as_bool().unwrap(), "should be pinned");

    // After pinned task, remaining should be sorted by priority_order ASC
    // So next should be p0 tasks, then p1, etc.
    let second = tasks[1]["priority"].as_str().unwrap();
    assert_eq!(second, "p0", "second task should be p0");

    // Verify blocked tasks are NOT in results
    let result_ids: Vec<&str> = tasks.iter().map(|t| t["id"].as_str().unwrap()).collect();
    for blocked_id in &["t2", "t6", "t10", "t14", "t18"] {
        assert!(
            !result_ids.contains(blocked_id),
            "blocked task {blocked_id} should not appear in results"
        );
    }

    // Verify deferred tasks are NOT in results
    for deferred_id in &["t3", "t7", "t11"] {
        assert!(
            !result_ids.contains(deferred_id),
            "deferred task {deferred_id} should not appear in results"
        );
    }

    // Test limit=5 caps results
    let params_limited = Some(json!({ "limit": 5 }));
    let result_limited = tools::dispatch(state.clone(), "get_ready_work", params_limited)
        .await
        .expect("get_ready_work with limit");

    let limited_tasks = result_limited["tasks"].as_array().unwrap();
    assert_eq!(limited_tasks.len(), 5, "limit=5 should return at most 5");
    assert_eq!(
        result_limited["total_eligible"].as_u64().unwrap(),
        12,
        "total_eligible unchanged by limit"
    );
}
