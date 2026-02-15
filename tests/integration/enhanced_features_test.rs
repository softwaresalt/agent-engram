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
use t_mem::services::dehydration::dehydrate_workspace;
use t_mem::services::hydration::hydrate_into_db;
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

// ── T033: Labels integration test ───────────────────────────────

#[tokio::test]
async fn t033_labels_add_remove_filter_and_flush_rehydrate() {
    let state = Arc::new(AppState::new(10));
    let ws_id = format!("labels_{}", uuid::Uuid::new_v4());

    // Use a temp directory for flush/rehydrate round-trip
    let tmpdir = tempfile::tempdir().expect("create tempdir");
    let ws_path = tmpdir.path().to_string_lossy().to_string();

    state
        .set_workspace(WorkspaceSnapshot {
            workspace_id: ws_id.clone(),
            path: ws_path.clone(),
            task_count: 0,
            context_count: 0,
            last_flush: None,
            stale_files: false,
            connection_count: 1,
            file_mtimes: std::collections::HashMap::new(),
        })
        .await
        .expect("set workspace");

    let db = connect_db(&ws_id).await.expect("connect db");
    let queries = Queries::new(db);

    // Create 5 tasks
    for i in 1..=5 {
        let task = make_task(&format!("lbl{i}"), "p2", 2);
        queries.upsert_task(&task).await.expect("upsert task");
    }

    // Assign labels via dispatch:
    // lbl1: frontend, bug
    // lbl2: frontend, backend
    // lbl3: bug, backend
    // lbl4: frontend, bug, backend
    // lbl5: (no labels)
    let label_assignments = [
        ("lbl1", vec!["frontend", "bug"]),
        ("lbl2", vec!["frontend", "backend"]),
        ("lbl3", vec!["bug", "backend"]),
        ("lbl4", vec!["frontend", "bug", "backend"]),
    ];

    for (task_id, labels) in &label_assignments {
        for label in labels {
            let result = tools::dispatch(
                state.clone(),
                "add_label",
                Some(json!({ "task_id": *task_id, "label": *label })),
            )
            .await
            .expect("add_label should succeed");
            assert!(result["label_count"].as_u64().unwrap() > 0);
        }
    }

    // Verify label counts
    assert_eq!(queries.count_labels_for_task("lbl1").await.unwrap(), 2);
    assert_eq!(queries.count_labels_for_task("lbl4").await.unwrap(), 3);
    assert_eq!(queries.count_labels_for_task("lbl5").await.unwrap(), 0);

    // Remove one label and verify
    let remove_result = tools::dispatch(
        state.clone(),
        "remove_label",
        Some(json!({ "task_id": "lbl4", "label": "backend" })),
    )
    .await
    .expect("remove_label should succeed");
    assert_eq!(remove_result["label_count"].as_u64().unwrap(), 2);

    // Re-add the label for the filter test
    tools::dispatch(
        state.clone(),
        "add_label",
        Some(json!({ "task_id": "lbl4", "label": "backend" })),
    )
    .await
    .expect("re-add label");

    // Filter by ["frontend", "bug"] AND logic — should return lbl1, lbl4
    let filter_result = tools::dispatch(
        state.clone(),
        "get_ready_work",
        Some(json!({ "label": ["frontend", "bug"] })),
    )
    .await
    .expect("get_ready_work with label filter");

    let filtered_tasks = filter_result["tasks"].as_array().unwrap();
    let filtered_ids: Vec<&str> = filtered_tasks
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();

    assert_eq!(
        filtered_ids.len(),
        2,
        "AND filter on [frontend, bug] should return 2 tasks, got: {filtered_ids:?}"
    );
    assert!(filtered_ids.contains(&"lbl1"), "lbl1 has both labels");
    assert!(filtered_ids.contains(&"lbl4"), "lbl4 has both labels");

    // ── Flush (dehydrate) to .tmem/ files ──
    dehydrate_workspace(&queries, tmpdir.path())
        .await
        .expect("dehydrate should succeed");

    // Verify tasks.md was written with labels
    let tasks_md =
        std::fs::read_to_string(tmpdir.path().join(".tmem/tasks.md")).expect("read tasks.md");
    assert!(
        tasks_md.contains("labels:"),
        "tasks.md should contain labels frontmatter"
    );
    // lbl4 has frontend, bug, backend — verify comma-separated format
    assert!(
        tasks_md.contains("backend") && tasks_md.contains("frontend"),
        "tasks.md should contain label values"
    );

    // ── Rehydrate into a fresh DB namespace ──
    let fresh_ws_id = format!("labels_fresh_{}", uuid::Uuid::new_v4());
    let fresh_db = connect_db(&fresh_ws_id).await.expect("connect fresh db");
    let fresh_queries = Queries::new(fresh_db);

    let hydration_result = hydrate_into_db(tmpdir.path(), &fresh_queries)
        .await
        .expect("rehydrate should succeed");

    assert_eq!(hydration_result.tasks_loaded, 5, "should rehydrate 5 tasks");

    // Verify labels survived the round-trip
    let lbl1_labels = fresh_queries
        .get_labels_for_task("lbl1")
        .await
        .expect("get labels lbl1");
    assert_eq!(
        lbl1_labels.len(),
        2,
        "lbl1 should have 2 labels after rehydration"
    );
    assert!(lbl1_labels.contains(&"bug".to_string()));
    assert!(lbl1_labels.contains(&"frontend".to_string()));

    let lbl4_labels = fresh_queries
        .get_labels_for_task("lbl4")
        .await
        .expect("get labels lbl4");
    assert_eq!(
        lbl4_labels.len(),
        3,
        "lbl4 should have 3 labels after rehydration"
    );

    let lbl5_labels = fresh_queries
        .get_labels_for_task("lbl5")
        .await
        .expect("get labels lbl5");
    assert_eq!(
        lbl5_labels.len(),
        0,
        "lbl5 should have 0 labels after rehydration"
    );
}

// ── T040: Enhanced dependency graph integration test ────────────

#[tokio::test]
async fn t040_parent_children_duplicate_blocked_by_in_ready_work() {
    let state = Arc::new(AppState::new(10));
    let ws_id = format!("deps_{}", uuid::Uuid::new_v4());

    let tmpdir = tempfile::tempdir().expect("create tempdir");
    let ws_path = tmpdir.path().to_string_lossy().to_string();

    state
        .set_workspace(WorkspaceSnapshot {
            workspace_id: ws_id.clone(),
            path: ws_path.clone(),
            task_count: 0,
            context_count: 0,
            last_flush: None,
            stale_files: false,
            connection_count: 1,
            file_mtimes: std::collections::HashMap::new(),
        })
        .await
        .expect("set workspace");

    let db = connect_db(&ws_id).await.expect("connect db");
    let queries = Queries::new(db);

    // Create parent and 3 children
    let parent = make_task("parent", "p2", 2);
    queries.upsert_task(&parent).await.expect("upsert parent");

    for i in 1..=3 {
        let child = make_task(&format!("child{i}"), "p2", 2);
        queries.upsert_task(&child).await.expect("upsert child");
    }

    // Create child_of edges: child1→parent, child2→parent, child3→parent
    for i in 1..=3 {
        tools::dispatch(
            state.clone(),
            "add_dependency",
            Some(json!({
                "from_task_id": format!("child{i}"),
                "to_task_id": "parent",
                "dependency_type": "child_of",
            })),
        )
        .await
        .expect("add child_of dependency");
    }

    // Create a duplicate task
    let dup = make_task("dup_task", "p2", 2);
    queries.upsert_task(&dup).await.expect("upsert dup");

    // Mark dup_task as duplicate_of child1
    tools::dispatch(
        state.clone(),
        "add_dependency",
        Some(json!({
            "from_task_id": "dup_task",
            "to_task_id": "child1",
            "dependency_type": "duplicate_of",
        })),
    )
    .await
    .expect("add duplicate_of");

    // Create a blocked task
    let blocked = make_task("blocked_task", "p2", 2);
    queries.upsert_task(&blocked).await.expect("upsert blocked");

    // blocked_task blocked_by child2
    tools::dispatch(
        state.clone(),
        "add_dependency",
        Some(json!({
            "from_task_id": "blocked_task",
            "to_task_id": "child2",
            "dependency_type": "blocked_by",
        })),
    )
    .await
    .expect("add blocked_by");

    // Verify get_ready_work: dup_task excluded (duplicate_of), blocked_task excluded (blocked_by)
    let result = tools::dispatch(state.clone(), "get_ready_work", Some(json!({})))
        .await
        .expect("get_ready_work");

    let tasks = result["tasks"].as_array().unwrap();
    let ids: Vec<&str> = tasks.iter().map(|t| t["id"].as_str().unwrap()).collect();

    // dup_task should be excluded (duplicate_of edge)
    assert!(
        !ids.contains(&"dup_task"),
        "duplicate task should be excluded from ready-work"
    );

    // blocked_task should be excluded (blocked_by edge)
    assert!(
        !ids.contains(&"blocked_task"),
        "blocked_by task should be excluded from ready-work"
    );

    // parent and children should be present
    assert!(ids.contains(&"parent"), "parent should be in ready-work");
    assert!(ids.contains(&"child1"), "child1 should be in ready-work");
    assert!(ids.contains(&"child2"), "child2 should be in ready-work");
    assert!(ids.contains(&"child3"), "child3 should be in ready-work");

    // Verify get_task_graph shows parent with children and type annotations
    let _graph_result = tools::dispatch(
        state.clone(),
        "get_task_graph",
        Some(json!({ "root_task_id": "parent" })),
    )
    .await
    .expect("get_task_graph");

    // Parent should have no outgoing edges (children point TO parent)
    // But children have outgoing child_of edges
    // get_task_graph shows edges FROM root, so parent won't show children
    // Let's check from child1 instead
    let child1_graph = tools::dispatch(
        state.clone(),
        "get_task_graph",
        Some(json!({ "root_task_id": "child1", "depth": 2 })),
    )
    .await
    .expect("get_task_graph for child1");

    let root = &child1_graph["root"];
    assert_eq!(root["id"].as_str().unwrap(), "child1");

    // child1 should have a child_of edge to parent
    let children = root["children"].as_array().unwrap();
    assert!(!children.is_empty(), "child1 should have child_of edge");
    assert_eq!(
        children[0]["dependency_type"].as_str().unwrap(),
        "child_of",
        "edge type should be child_of"
    );

    // Mark all children done
    for i in 1..=3 {
        tools::dispatch(
            state.clone(),
            "update_task",
            Some(json!({
                "id": format!("child{i}"),
                "status": "done",
            })),
        )
        .await
        .expect("mark child done");
    }

    // Now blocked_task should be in ready-work (child2 is done, no longer blocking)
    let result_after = tools::dispatch(state.clone(), "get_ready_work", Some(json!({})))
        .await
        .expect("get_ready_work after children done");

    let tasks_after = result_after["tasks"].as_array().unwrap();
    let ids_after: Vec<&str> = tasks_after
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();

    // blocked_task should now be eligible (child2 is done)
    assert!(
        ids_after.contains(&"blocked_task"),
        "blocked_task should be in ready-work after blocker is done"
    );

    // parent should still be eligible
    assert!(
        ids_after.contains(&"parent"),
        "parent should still be in ready-work"
    );

    // Flush and rehydrate to verify edge persistence
    dehydrate_workspace(&queries, tmpdir.path())
        .await
        .expect("dehydrate");

    let graph_surql =
        std::fs::read_to_string(tmpdir.path().join(".tmem/graph.surql")).expect("read graph.surql");
    assert!(
        graph_surql.contains("child_of"),
        "graph.surql should contain child_of edges"
    );
    assert!(
        graph_surql.contains("duplicate_of"),
        "graph.surql should contain duplicate_of edge"
    );
    assert!(
        graph_surql.contains("blocked_by"),
        "graph.surql should contain blocked_by edge"
    );

    // Rehydrate into fresh DB
    let fresh_ws_id = format!("deps_fresh_{}", uuid::Uuid::new_v4());
    let fresh_db = connect_db(&fresh_ws_id).await.expect("connect fresh db");
    let fresh_queries = Queries::new(fresh_db);

    let hydration = hydrate_into_db(tmpdir.path(), &fresh_queries)
        .await
        .expect("rehydrate");

    assert_eq!(hydration.tasks_loaded, 6, "should rehydrate 6 tasks");
    assert!(
        hydration.edges_loaded >= 5,
        "should rehydrate at least 5 edges (3 child_of + 1 duplicate_of + 1 blocked_by)"
    );
}
