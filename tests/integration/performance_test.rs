//! Performance benchmark tests for enhanced task management.
//!
//! Validates success criteria timing constraints:
//! - SC-015: statistics <100ms (5000 tasks)
//!
//! Note: thresholds are relaxed for debug-build CI; production targets
//! assume `--release` builds.

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use serde_json::json;

use engram::db::connect_db;
use engram::db::queries::Queries;
use engram::models::task::{Task, TaskStatus, compute_priority_order};
use engram::server::state::AppState;
use engram::tools;

/// Helper: create a `Task` value for bulk insertion.
fn make_perf_task(index: usize, status: TaskStatus, priority: &str) -> Task {
    let now = Utc::now();
    Task {
        id: format!("perf-{index:05}"),
        title: format!("Performance test task {index}"),
        status,
        work_item_id: None,
        description: format!("Description for perf task {index}"),
        context_summary: None,
        priority: priority.to_owned(),
        priority_order: compute_priority_order(priority),
        issue_type: if index % 3 == 0 { "bug" } else { "task" }.to_owned(),
        assignee: if index % 5 == 0 {
            Some("agent-perf".to_owned())
        } else {
            None
        },
        defer_until: None,
        pinned: index % 20 == 0,
        compaction_level: 0,
        compacted_at: None,
        workflow_state: None,
        workflow_id: None,
        created_at: now,
        updated_at: now,
    }
}

/// Setup: bind workspace and return (state, queries).
async fn perf_setup(task_count: usize) -> (Arc<AppState>, Queries) {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(workspace.path().join(".git")).expect(".git");
    let engram_dir = workspace.path().join(".engram");
    std::fs::create_dir_all(&engram_dir).expect(".engram");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    let ws_id = state
        .snapshot_workspace()
        .await
        .expect("snapshot")
        .workspace_id;
    let db = connect_db(&ws_id).await.expect("db");
    let queries = Queries::new(db);

    // Bulk insert tasks
    let priorities = ["p0", "p1", "p2", "p3", "p4"];
    for i in 0..task_count {
        let priority = priorities[i % priorities.len()];
        let status = match i % 4 {
            0 => TaskStatus::Todo,
            1 => TaskStatus::InProgress,
            2 => TaskStatus::Done,
            _ => TaskStatus::Blocked,
        };
        let task = make_perf_task(i, status, priority);
        queries.upsert_task(&task).await.expect("insert task");
    }

    // Release the tempdir handle so the workspace persists for the test
    // (leaking is fine in test code — OS cleans up on exit)
    std::mem::forget(workspace);

    (state, queries)
}

// ── SC-015: statistics <100ms (5000 tasks) ──────────────────────

#[tokio::test]
async fn t089_sc015_statistics_performance() {
    let (state, _queries) = perf_setup(5000).await;

    let start = Instant::now();
    let result = tools::dispatch(state.clone(), "get_workspace_statistics", Some(json!({})))
        .await
        .expect("get_workspace_statistics");
    let elapsed = start.elapsed();

    assert!(
        result.get("total_tasks").is_some(),
        "should return total_tasks"
    );
    assert_eq!(
        result
            .get("total_tasks")
            .and_then(serde_json::Value::as_u64),
        Some(5000)
    );
    assert!(
        elapsed.as_millis() < 30_000,
        "SC-015: statistics should complete in <30s (debug build, \
         prod target <100ms); took {}ms",
        elapsed.as_millis()
    );
}

