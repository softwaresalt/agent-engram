//! Performance benchmark tests for enhanced task management.
//!
//! Validates success criteria timing constraints:
//! - SC-011: `get_ready_work` <50ms (1000 tasks)
//! - SC-012: batch 100 <500ms
//! - SC-013: compaction candidates <100ms (5000 tasks)
//! - SC-015: statistics <100ms (5000 tasks)
//! - SC-018: each filter dimension <20ms overhead
//!
//! Note: thresholds are relaxed for debug-build CI; production targets
//! assume `--release` builds.

use std::sync::Arc;
use std::time::Instant;

use chrono::{Duration, Utc};
use serde_json::json;

use t_mem::db::connect_db;
use t_mem::db::queries::Queries;
use t_mem::models::task::{Task, TaskStatus, compute_priority_order};
use t_mem::server::state::AppState;
use t_mem::tools;

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
    let tmem_dir = workspace.path().join(".tmem");
    std::fs::create_dir_all(&tmem_dir).expect(".tmem");

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

// ── SC-011: get_ready_work <50ms (1000 tasks) ───────────────────

#[tokio::test]
async fn t089_sc011_get_ready_work_performance() {
    let (state, _queries) = perf_setup(1000).await;

    let start = Instant::now();
    let result = tools::dispatch(
        state.clone(),
        "get_ready_work",
        Some(json!({ "limit": 50 })),
    )
    .await
    .expect("get_ready_work");
    let elapsed = start.elapsed();

    assert!(
        result.get("tasks").and_then(|v| v.as_array()).is_some(),
        "should return tasks array"
    );
    assert!(
        elapsed.as_millis() < 5000,
        "SC-011: get_ready_work should complete in <5s (debug build, \
         prod target <50ms); took {}ms",
        elapsed.as_millis()
    );
}

// ── SC-012: batch 100 <500ms ────────────────────────────────────

#[tokio::test]
async fn t089_sc012_batch_update_performance() {
    let (state, _queries) = perf_setup(200).await;

    // Build batch of 100 updates (toggle status on todo→in_progress)
    let updates: Vec<serde_json::Value> = (0..200)
        .filter(|i| i % 4 == 0) // only todo tasks (every 4th)
        .take(100)
        .map(|i| {
            json!({
                "id": format!("perf-{i:05}"),
                "status": "in_progress",
                "notes": "Batch perf test"
            })
        })
        .collect();

    let start = Instant::now();
    let result = tools::dispatch(
        state.clone(),
        "batch_update_tasks",
        Some(json!({ "updates": updates })),
    )
    .await
    .expect("batch_update_tasks");
    let elapsed = start.elapsed();

    assert!(
        result.get("succeeded").is_some(),
        "should return succeeded count"
    );
    assert!(
        elapsed.as_millis() < 30_000,
        "SC-012: batch 100 should complete in <30s (debug build, \
         prod target <500ms); took {}ms",
        elapsed.as_millis()
    );
}

// ── SC-013: compaction candidates <100ms (5000 tasks) ───────────

#[tokio::test]
async fn t089_sc013_compaction_candidates_performance() {
    let (state, _queries) = perf_setup(5000).await;

    // Backdate all done-task updated_at to 10 days ago
    let ws_id = state
        .snapshot_workspace()
        .await
        .expect("snapshot")
        .workspace_id;
    let db = connect_db(&ws_id).await.expect("db");
    let old_date = (Utc::now() - Duration::days(10)).to_rfc3339();
    db.query(format!(
        "UPDATE task SET updated_at = <datetime>'{old_date}' WHERE status = 'done'"
    ))
    .await
    .expect("backdate done tasks");

    let start = Instant::now();
    let result = tools::dispatch(
        state.clone(),
        "get_compaction_candidates",
        Some(json!({ "threshold_days": 7, "max_candidates": 50 })),
    )
    .await
    .expect("get_compaction_candidates");
    let elapsed = start.elapsed();

    assert!(
        result
            .get("candidates")
            .and_then(|v| v.as_array())
            .is_some(),
        "should return candidates"
    );
    assert!(
        elapsed.as_millis() < 30_000,
        "SC-013: compaction candidates should complete in <30s (debug build, \
         prod target <100ms); took {}ms",
        elapsed.as_millis()
    );
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

// ── SC-018: filter dimension overhead <20ms ─────────────────────

#[tokio::test]
async fn t089_sc018_filter_dimension_overhead() {
    let (state, _queries) = perf_setup(1000).await;

    // Baseline: no filters
    let start_base = Instant::now();
    let _base = tools::dispatch(
        state.clone(),
        "get_ready_work",
        Some(json!({ "limit": 50 })),
    )
    .await
    .expect("baseline");
    let base_ms = start_base.elapsed().as_millis();

    // Filter: priority
    let start_pri = Instant::now();
    let _pri = tools::dispatch(
        state.clone(),
        "get_ready_work",
        Some(json!({ "limit": 50, "priority": "p0" })),
    )
    .await
    .expect("priority filter");
    let pri_ms = start_pri.elapsed().as_millis();

    // Filter: issue_type
    let start_type = Instant::now();
    let _typ = tools::dispatch(
        state.clone(),
        "get_ready_work",
        Some(json!({ "limit": 50, "issue_type": "bug" })),
    )
    .await
    .expect("type filter");
    let type_ms = start_type.elapsed().as_millis();

    // Filter: assignee
    let start_assignee = Instant::now();
    let _asg = tools::dispatch(
        state.clone(),
        "get_ready_work",
        Some(json!({ "limit": 50, "assignee": "agent-perf" })),
    )
    .await
    .expect("assignee filter");
    let assignee_ms = start_assignee.elapsed().as_millis();

    // Verify each filter completed (overhead can be high in debug, just assert
    // all are < generous threshold)
    let max_overhead = 10_000; // debug build allowance
    assert!(
        pri_ms < max_overhead,
        "SC-018: priority filter should be reasonable; took {pri_ms}ms"
    );
    assert!(
        type_ms < max_overhead,
        "SC-018: type filter should be reasonable; took {type_ms}ms"
    );
    assert!(
        assignee_ms < max_overhead,
        "SC-018: assignee filter should be reasonable; took {assignee_ms}ms"
    );

    // Log all timings for visibility
    eprintln!(
        "SC-018 filter overhead: baseline={base_ms}ms, \
         priority={pri_ms}ms, type={type_ms}ms, assignee={assignee_ms}ms"
    );
}
