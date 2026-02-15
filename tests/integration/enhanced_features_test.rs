#![allow(clippy::too_many_lines)]
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

// ── T047: Compaction candidates, apply, and graph preservation ──

#[tokio::test]
async fn t047_compaction_50_done_tasks_apply_and_graph_preserved() {
    use surrealdb::RecordId as Thing;

    let state = Arc::new(AppState::new(10));
    let ws_id = format!("compact_{}", uuid::Uuid::new_v4());

    let tmpdir = tempfile::tempdir().expect("create tempdir");
    let ws_path = tmpdir.path().to_string_lossy().to_string();

    state
        .set_workspace(WorkspaceSnapshot {
            workspace_id: ws_id.clone(),
            path: ws_path,
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
    let queries = Queries::new(db.clone());

    // Create 50 done tasks
    let old_date = (Utc::now() - Duration::days(14)).to_rfc3339();
    for i in 1..=50 {
        let task = Task {
            id: format!("cpt{i}"),
            title: format!("Compactable task {i}"),
            status: TaskStatus::Done,
            work_item_id: None,
            description: format!(
                "This is the original verbose description for task {i} that should be compacted."
            ),
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
        queries.upsert_task(&task).await.expect("upsert task");

        // Force-set updated_at to 14 days ago
        let record = Thing::from(("task", task.id.as_str()));
        db.query("UPDATE $record SET updated_at = <datetime>$old_date")
            .bind(("record", record))
            .bind(("old_date", old_date.clone()))
            .await
            .expect("force old updated_at");
    }

    // Add dependency edges between a few tasks to verify graph preservation
    // cpt1 → cpt2 (hard_blocker), cpt3 → cpt4 (child_of), cpt5 → cpt6 (related_to)
    for (from, to, dep_type) in [
        ("cpt1", "cpt2", "hard_blocker"),
        ("cpt3", "cpt4", "child_of"),
        ("cpt5", "cpt6", "related_to"),
    ] {
        tools::dispatch(
            state.clone(),
            "add_dependency",
            Some(json!({
                "from_task_id": from,
                "to_task_id": to,
                "dependency_type": dep_type,
            })),
        )
        .await
        .expect("add dependency");
    }

    // Call get_compaction_candidates with threshold=7 (all 50 are 14 days old)
    let result = tools::dispatch(
        state.clone(),
        "get_compaction_candidates",
        Some(json!({ "threshold_days": 7, "max_candidates": 50 })),
    )
    .await
    .expect("get_compaction_candidates");

    let candidates = result["candidates"].as_array().unwrap();
    assert_eq!(
        candidates.len(),
        50,
        "all 50 done tasks should be candidates"
    );

    // Verify candidate shape
    let first = &candidates[0];
    assert!(first["task_id"].is_string());
    assert!(first["age_days"].as_i64().unwrap() >= 14);
    assert_eq!(first["compaction_level"].as_u64().unwrap(), 0);

    // Apply compaction to 10 tasks with summaries
    let compactions: Vec<serde_json::Value> = (1..=10)
        .map(|i| {
            json!({
                "task_id": format!("cpt{i}"),
                "summary": format!("Compacted summary for task {i}"),
            })
        })
        .collect();

    let apply_result = tools::dispatch(
        state.clone(),
        "apply_compaction",
        Some(json!({ "compactions": compactions })),
    )
    .await
    .expect("apply_compaction");

    let results = apply_result["results"].as_array().unwrap();
    assert_eq!(results.len(), 10, "should have 10 compaction results");

    // All compacted tasks should have compaction_level=1 and compacted_at set
    for r in results {
        assert_eq!(
            r["new_compaction_level"].as_u64().unwrap(),
            1,
            "compaction_level should be 1 after first compaction"
        );
        assert!(
            r["compacted_at"].is_string(),
            "compacted_at should be set after compaction"
        );
    }

    // Verify graph edges are preserved after compaction
    let graph = tools::dispatch(
        state.clone(),
        "get_task_graph",
        Some(json!({ "root_task_id": "cpt1", "depth": 1 })),
    )
    .await
    .expect("get_task_graph after compaction");

    let root = &graph["root"];
    assert_eq!(root["id"].as_str().unwrap(), "cpt1");
    let children = root["children"].as_array().unwrap();
    assert!(
        !children.is_empty(),
        "cpt1 should still have edges after compaction"
    );
    assert_eq!(
        children[0]["dependency_type"].as_str().unwrap(),
        "hard_blocker",
        "edge type should be preserved"
    );

    // Verify non-compacted tasks still have original descriptions
    let remaining = tools::dispatch(
        state.clone(),
        "get_compaction_candidates",
        Some(json!({ "threshold_days": 7, "max_candidates": 50 })),
    )
    .await
    .expect("get remaining candidates");

    let remaining_candidates = remaining["candidates"].as_array().unwrap();
    // 10 were compacted (updated_at refreshed to now), 40 still old
    assert_eq!(
        remaining_candidates.len(),
        40,
        "40 tasks should remain as candidates after compacting 10"
    );

    // Flush and verify compacted tasks survive dehydration/rehydration
    dehydrate_workspace(&queries, tmpdir.path())
        .await
        .expect("dehydrate after compaction");

    let tasks_md =
        std::fs::read_to_string(tmpdir.path().join(".tmem/tasks.md")).expect("read tasks.md");
    assert!(
        tasks_md.contains("Compacted summary for task 1"),
        "tasks.md should have compacted summary"
    );

    let fresh_ws_id = format!("compact_fresh_{}", uuid::Uuid::new_v4());
    let fresh_db = connect_db(&fresh_ws_id).await.expect("connect fresh db");
    let fresh_queries = Queries::new(fresh_db);

    let hydration = hydrate_into_db(tmpdir.path(), &fresh_queries)
        .await
        .expect("rehydrate");

    assert_eq!(hydration.tasks_loaded, 50, "should rehydrate all 50 tasks");
    assert!(
        hydration.edges_loaded >= 3,
        "should rehydrate at least 3 edges"
    );

    // Verify compacted task has the summary after rehydration
    let rehydrated = fresh_queries
        .get_task("cpt1")
        .await
        .expect("get rehydrated cpt1")
        .expect("cpt1 should exist");
    assert_eq!(
        rehydrated.description, "Compacted summary for task 1",
        "description should be compacted summary after rehydration"
    );
    assert_eq!(
        rehydrated.compaction_level, 1,
        "compaction_level should survive rehydration"
    );
}

// ── T048: Pinned exclusion and compaction_level increment ───────

#[tokio::test]
async fn t048_pinned_excluded_and_compaction_level_increments() {
    use surrealdb::RecordId as Thing;

    let state = Arc::new(AppState::new(10));
    let ws_id = format!("compact_pin_{}", uuid::Uuid::new_v4());

    state
        .set_workspace(test_snapshot(&ws_id))
        .await
        .expect("set workspace");

    let db = connect_db(&ws_id).await.expect("connect db");
    let queries = Queries::new(db.clone());

    let old_date = (Utc::now() - Duration::days(14)).to_rfc3339();

    // Create a pinned done task — should be excluded from candidates
    let pinned = Task {
        id: "pinned1".to_string(),
        title: "Pinned done task".to_string(),
        status: TaskStatus::Done,
        work_item_id: None,
        description: "This should NOT be compacted".to_string(),
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
        .upsert_task(&pinned)
        .await
        .expect("upsert pinned task");

    // Force old updated_at on pinned task
    let record = Thing::from(("task", "pinned1"));
    db.query("UPDATE $record SET updated_at = <datetime>$old_date")
        .bind(("record", record))
        .bind(("old_date", old_date.clone()))
        .await
        .expect("force old updated_at on pinned");

    // Create an unpinned done task
    let unpinned = Task {
        id: "unpinned1".to_string(),
        title: "Unpinned done task".to_string(),
        status: TaskStatus::Done,
        work_item_id: None,
        description: "Original description that will be compacted twice".to_string(),
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
        .upsert_task(&unpinned)
        .await
        .expect("upsert unpinned task");

    // Force old updated_at on unpinned task
    let record = Thing::from(("task", "unpinned1"));
    db.query("UPDATE $record SET updated_at = <datetime>$old_date")
        .bind(("record", record))
        .bind(("old_date", old_date.clone()))
        .await
        .expect("force old updated_at on unpinned");

    // Get candidates — only unpinned1 should appear
    let result = tools::dispatch(
        state.clone(),
        "get_compaction_candidates",
        Some(json!({ "threshold_days": 7 })),
    )
    .await
    .expect("get candidates");

    let candidates = result["candidates"].as_array().unwrap();
    let ids: Vec<&str> = candidates
        .iter()
        .map(|c| c["task_id"].as_str().unwrap())
        .collect();

    assert!(
        !ids.contains(&"pinned1"),
        "pinned task must be excluded from candidates"
    );
    assert!(
        ids.contains(&"unpinned1"),
        "unpinned done task should be a candidate"
    );

    // First compaction: compaction_level 0 → 1
    let apply1 = tools::dispatch(
        state.clone(),
        "apply_compaction",
        Some(json!({
            "compactions": [{
                "task_id": "unpinned1",
                "summary": "First compaction summary"
            }]
        })),
    )
    .await
    .expect("first apply_compaction");

    let r1 = &apply1["results"].as_array().unwrap()[0];
    assert_eq!(
        r1["new_compaction_level"].as_u64().unwrap(),
        1,
        "first compaction should set level to 1"
    );

    // Force old updated_at again so it becomes a candidate again
    let record = Thing::from(("task", "unpinned1"));
    db.query("UPDATE $record SET updated_at = <datetime>$old_date")
        .bind(("record", record))
        .bind(("old_date", old_date.clone()))
        .await
        .expect("force old updated_at again");

    // Second compaction: compaction_level 1 → 2
    let apply2 = tools::dispatch(
        state.clone(),
        "apply_compaction",
        Some(json!({
            "compactions": [{
                "task_id": "unpinned1",
                "summary": "Second compaction — even shorter"
            }]
        })),
    )
    .await
    .expect("second apply_compaction");

    let r2 = &apply2["results"].as_array().unwrap()[0];
    assert_eq!(
        r2["new_compaction_level"].as_u64().unwrap(),
        2,
        "second compaction should increment to level 2"
    );

    // Verify final DB state
    let final_task = queries
        .get_task("unpinned1")
        .await
        .expect("get task")
        .expect("task should exist");
    assert_eq!(final_task.compaction_level, 2);
    assert_eq!(final_task.description, "Second compaction — even shorter");
    assert!(final_task.compacted_at.is_some());

    // Verify pinned task is untouched
    let pinned_check = queries
        .get_task("pinned1")
        .await
        .expect("get task")
        .expect("pinned task should exist");
    assert_eq!(pinned_check.description, "This should NOT be compacted");
    assert_eq!(pinned_check.compaction_level, 0);
    assert!(pinned_check.compacted_at.is_none());
}

// ── T053: Task claiming and assignment integration test ─────────

#[tokio::test]
async fn t053_claim_release_conflict_audit_trail_and_assignee_filter() {
    let state = Arc::new(AppState::new(10));
    let ws_id = format!("claim_{}", uuid::Uuid::new_v4());

    state
        .set_workspace(test_snapshot(&ws_id))
        .await
        .expect("set workspace");

    let db = connect_db(&ws_id).await.expect("connect db");
    let queries = Queries::new(db);

    // Create 3 tasks
    for i in 1..=3 {
        let task = make_task(&format!("cl{i}"), "p2", 2);
        queries.upsert_task(&task).await.expect("upsert task");
    }

    // Client A claims cl1
    let claim_a = tools::dispatch(
        state.clone(),
        "claim_task",
        Some(json!({ "task_id": "cl1", "claimant": "agent-1" })),
    )
    .await
    .expect("agent-1 claim should succeed");

    assert_eq!(claim_a["claimant"].as_str().unwrap(), "agent-1");
    assert!(claim_a["context_id"].is_string());
    assert!(claim_a["claimed_at"].is_string());

    // Client B tries to claim cl1 — should be rejected with TASK_ALREADY_CLAIMED
    let err = tools::dispatch(
        state.clone(),
        "claim_task",
        Some(json!({ "task_id": "cl1", "claimant": "agent-2" })),
    )
    .await
    .expect_err("agent-2 claim should be rejected");

    assert_eq!(
        err.to_response().error.code,
        t_mem::errors::codes::TASK_ALREADY_CLAIMED
    );

    // Agent-2 claims cl2 (different task, should succeed)
    let claim_b = tools::dispatch(
        state.clone(),
        "claim_task",
        Some(json!({ "task_id": "cl2", "claimant": "agent-2" })),
    )
    .await
    .expect("agent-2 claim of cl2 should succeed");
    assert_eq!(claim_b["claimant"].as_str().unwrap(), "agent-2");

    // Verify get_ready_work with assignee filter
    let ready_agent1 = tools::dispatch(
        state.clone(),
        "get_ready_work",
        Some(json!({ "assignee": "agent-1" })),
    )
    .await
    .expect("get_ready_work with assignee filter");

    let agent1_tasks = ready_agent1["tasks"].as_array().unwrap();
    assert_eq!(
        agent1_tasks.len(),
        1,
        "agent-1 should have exactly 1 claimed task"
    );
    assert_eq!(agent1_tasks[0]["id"].as_str().unwrap(), "cl1");

    // Client B releases cl1 (third-party release is allowed)
    let release = tools::dispatch(
        state.clone(),
        "release_task",
        Some(json!({ "task_id": "cl1" })),
    )
    .await
    .expect("third-party release should succeed");

    assert_eq!(
        release["previous_claimant"].as_str().unwrap(),
        "agent-1",
        "release should report previous claimant"
    );
    assert!(release["context_id"].is_string());
    assert!(release["released_at"].is_string());

    // After release, cl1 should no longer appear in agent-1's filtered results
    let ready_after = tools::dispatch(
        state.clone(),
        "get_ready_work",
        Some(json!({ "assignee": "agent-1" })),
    )
    .await
    .expect("get_ready_work after release");

    let after_tasks = ready_after["tasks"].as_array().unwrap();
    assert!(
        after_tasks.is_empty(),
        "agent-1 should have no claimed tasks after release"
    );

    // Verify context notes were created (audit trail)
    // The claim and release should each have created a context note
    // linked to cl1. The DB should have at least 2 context records.
    let all_contexts = queries.all_contexts().await.expect("all contexts");
    let cl1_notes: Vec<_> = all_contexts
        .iter()
        .filter(|c| c.content.contains("agent-1"))
        .collect();

    assert!(
        cl1_notes.len() >= 2,
        "should have at least 2 context notes mentioning agent-1 (claim + release)"
    );

    // Verify one note has "Claimed by" and one has "Released"
    let has_claim = cl1_notes.iter().any(|c| c.content.contains("Claimed by"));
    let has_release = cl1_notes.iter().any(|c| c.content.contains("Released"));
    assert!(has_claim, "should have a claim context note");
    assert!(has_release, "should have a release context note");

    // cl3 is unclaimed — release should fail
    let unclaimed_err = tools::dispatch(
        state.clone(),
        "release_task",
        Some(json!({ "task_id": "cl3" })),
    )
    .await
    .expect_err("release of unclaimed task should fail");

    assert_eq!(
        unclaimed_err.to_response().error.code,
        t_mem::errors::codes::TASK_NOT_CLAIMABLE
    );
}

// ── T058: Issue types integration test ──────────────────────────

#[tokio::test]
async fn t058_issue_types_filter_custom_type_and_context_note() {
    // Set up workspace with allowed_types config
    let workspace = tempfile::tempdir().expect("workspace");
    std::fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let tmem_dir = workspace.path().join(".tmem");
    std::fs::create_dir_all(&tmem_dir).expect("create .tmem");
    std::fs::write(
        tmem_dir.join("config.toml"),
        r#"allowed_types = ["task", "bug", "spike", "decision", "milestone"]
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

    // Create tasks with different issue_types
    let task_result = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Normal task" })),
    )
    .await
    .expect("create default task");
    let task_id = task_result["task_id"].as_str().unwrap().to_string();
    assert_eq!(task_result["issue_type"].as_str().unwrap(), "task");

    let bug_result = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Fix crash", "issue_type": "bug" })),
    )
    .await
    .expect("create bug");
    let bug_id = bug_result["task_id"].as_str().unwrap().to_string();
    assert_eq!(bug_result["issue_type"].as_str().unwrap(), "bug");

    let spike_result = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Research options", "issue_type": "spike" })),
    )
    .await
    .expect("create spike");
    let spike_id = spike_result["task_id"].as_str().unwrap().to_string();
    assert_eq!(spike_result["issue_type"].as_str().unwrap(), "spike");

    // ── Filter by issue_type on get_ready_work ──────────────────
    let bugs_only = tools::dispatch(
        state.clone(),
        "get_ready_work",
        Some(json!({ "issue_type": "bug" })),
    )
    .await
    .expect("get_ready_work bug filter");

    let bug_tasks = bugs_only["tasks"].as_array().unwrap();
    assert_eq!(bug_tasks.len(), 1, "only one bug task expected");
    assert_eq!(bug_tasks[0]["id"].as_str().unwrap(), bug_id);

    // No filter returns all 3
    let all_tasks = tools::dispatch(state.clone(), "get_ready_work", Some(json!({})))
        .await
        .expect("get_ready_work all");
    assert_eq!(all_tasks["tasks"].as_array().unwrap().len(), 3);

    // ── Invalid type rejected on create ─────────────────────────
    let err = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Epic task", "issue_type": "epic" })),
    )
    .await
    .expect_err("epic not in allowed_types");
    assert_eq!(
        err.to_response().error.code,
        t_mem::errors::codes::INVALID_ISSUE_TYPE
    );

    // ── Type change via update_task creates context note ────────
    let update_result = tools::dispatch(
        state.clone(),
        "update_task",
        Some(json!({
            "id": task_id,
            "status": "todo",
            "issue_type": "decision",
        })),
    )
    .await
    .expect("update issue_type to decision");
    assert!(update_result["context_id"].is_string());

    // Verify task now has issue_type "decision" via get_ready_work filter
    let decisions = tools::dispatch(
        state.clone(),
        "get_ready_work",
        Some(json!({ "issue_type": "decision" })),
    )
    .await
    .expect("filter decisions");
    let decision_tasks = decisions["tasks"].as_array().unwrap();
    assert_eq!(decision_tasks.len(), 1);
    assert_eq!(decision_tasks[0]["id"].as_str().unwrap(), task_id);
    assert_eq!(
        decision_tasks[0]["issue_type"].as_str().unwrap(),
        "decision"
    );

    // Verify invalid type change rejected on update
    let err = tools::dispatch(
        state.clone(),
        "update_task",
        Some(json!({
            "id": spike_id,
            "status": "todo",
            "issue_type": "feature",
        })),
    )
    .await
    .expect_err("feature not in allowed_types");
    assert_eq!(
        err.to_response().error.code,
        t_mem::errors::codes::INVALID_ISSUE_TYPE
    );

    // ── Flush and rehydrate preserves issue_type ────────────────
    tools::dispatch(state.clone(), "flush_state", None)
        .await
        .expect("flush_state");

    // Read tasks.md and verify issue_type appears in frontmatter
    let tasks_md = std::fs::read_to_string(tmem_dir.join("tasks.md")).expect("read tasks.md");
    assert!(
        tasks_md.contains("issue_type: bug"),
        "bug type in frontmatter"
    );
    assert!(
        tasks_md.contains("issue_type: decision"),
        "decision type in frontmatter"
    );
    assert!(
        tasks_md.contains("issue_type: spike"),
        "spike type in frontmatter"
    );

    // Create a new state, set workspace, and verify rehydrated types
    let state2 = Arc::new(AppState::new(10));
    tools::dispatch(
        state2.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace 2");

    let rehydrated = tools::dispatch(
        state2.clone(),
        "get_ready_work",
        Some(json!({ "issue_type": "bug" })),
    )
    .await
    .expect("rehydrated bug filter");
    let rehydrated_bugs = rehydrated["tasks"].as_array().unwrap();
    assert_eq!(
        rehydrated_bugs.len(),
        1,
        "bug type preserved after rehydration"
    );
}

// ── T065: Defer/Pin integration test ────────────────────────────

#[tokio::test]
async fn t065_defer_pin_ready_work_interaction() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    std::fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let tmem_dir = workspace.path().join(".tmem");
    std::fs::create_dir_all(&tmem_dir).expect("create .tmem");
    std::fs::write(tmem_dir.join("tasks.md"), "# Tasks\n").expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_str().unwrap().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    // Create 3 tasks with different priorities
    let t1 = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "High priority task" })),
    )
    .await
    .expect("create t1");
    let t1_id = t1["task_id"].as_str().unwrap().to_string();

    let t2 = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Medium priority task" })),
    )
    .await
    .expect("create t2");
    let t2_id = t2["task_id"].as_str().unwrap().to_string();

    let t3 = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": "Low priority task" })),
    )
    .await
    .expect("create t3");
    let t3_id = t3["task_id"].as_str().unwrap().to_string();

    // All 3 should appear initially
    let ready = tools::dispatch(state.clone(), "get_ready_work", Some(json!({})))
        .await
        .expect("get_ready_work initial");
    assert_eq!(
        ready["tasks"].as_array().unwrap().len(),
        3,
        "all 3 tasks visible"
    );

    // Defer t1 to the far future: should be excluded
    let future_date = "2099-12-31T23:59:59Z";
    tools::dispatch(
        state.clone(),
        "defer_task",
        Some(json!({ "task_id": t1_id, "until": future_date })),
    )
    .await
    .expect("defer_task t1");

    let ready_after_defer = tools::dispatch(state.clone(), "get_ready_work", Some(json!({})))
        .await
        .expect("get_ready_work after defer");
    let tasks_after_defer = ready_after_defer["tasks"].as_array().unwrap();
    assert_eq!(tasks_after_defer.len(), 2, "deferred task excluded");
    let ids: Vec<&str> = tasks_after_defer
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert!(
        !ids.iter().any(|id| id.ends_with(&t1_id)),
        "t1 should not appear while deferred"
    );

    // Undefer t1: should reappear
    tools::dispatch(
        state.clone(),
        "undefer_task",
        Some(json!({ "task_id": t1_id })),
    )
    .await
    .expect("undefer_task t1");

    let ready_after_undefer = tools::dispatch(state.clone(), "get_ready_work", Some(json!({})))
        .await
        .expect("get_ready_work after undefer");
    assert_eq!(
        ready_after_undefer["tasks"].as_array().unwrap().len(),
        3,
        "undeferred task reappears"
    );

    // Pin t3 (lowest priority): should appear first
    tools::dispatch(state.clone(), "pin_task", Some(json!({ "task_id": t3_id })))
        .await
        .expect("pin_task t3");

    let ready_pinned = tools::dispatch(state.clone(), "get_ready_work", Some(json!({})))
        .await
        .expect("get_ready_work pinned");
    let pinned_tasks = ready_pinned["tasks"].as_array().unwrap();
    assert_eq!(pinned_tasks.len(), 3);
    // Pinned task should be first regardless of priority
    let first_id = pinned_tasks[0]["id"].as_str().unwrap();
    assert!(
        first_id.ends_with(&t3_id),
        "pinned task should be first, got {first_id}"
    );

    // Unpin t3: should return to normal position
    tools::dispatch(
        state.clone(),
        "unpin_task",
        Some(json!({ "task_id": t3_id })),
    )
    .await
    .expect("unpin_task t3");

    let ready_unpinned = tools::dispatch(state.clone(), "get_ready_work", Some(json!({})))
        .await
        .expect("get_ready_work unpinned");
    let unpinned_tasks = ready_unpinned["tasks"].as_array().unwrap();
    // After unpinning, t3 should no longer be forced first
    // (all have same default priority, so order may vary, but pinned=false)
    assert_eq!(unpinned_tasks.len(), 3, "all still present after unpin");

    // Verify flush/rehydrate preserves defer_until and pinned
    tools::dispatch(
        state.clone(),
        "defer_task",
        Some(json!({ "task_id": t2_id, "until": "2099-01-01T00:00:00Z" })),
    )
    .await
    .expect("defer t2 for persistence test");

    tools::dispatch(state.clone(), "pin_task", Some(json!({ "task_id": t3_id })))
        .await
        .expect("pin t3 for persistence test");

    tools::dispatch(state.clone(), "flush_state", Some(json!({})))
        .await
        .expect("flush_state");

    // Verify frontmatter content
    let tasks_md =
        std::fs::read_to_string(tmem_dir.join("tasks.md")).expect("read flushed tasks.md");
    assert!(
        tasks_md.contains("defer_until:"),
        "defer_until in frontmatter"
    );
    assert!(tasks_md.contains("pinned: true"), "pinned in frontmatter");

    // Rehydrate into fresh state
    let state2 = Arc::new(AppState::new(10));
    tools::dispatch(
        state2.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace 2");

    let rehydrated = tools::dispatch(state2.clone(), "get_ready_work", Some(json!({})))
        .await
        .expect("rehydrated get_ready_work");
    let rehydrated_tasks = rehydrated["tasks"].as_array().unwrap();

    // t2 deferred to 2099 → excluded; t1 and t3 visible; t3 pinned → first
    assert_eq!(
        rehydrated_tasks.len(),
        2,
        "deferred t2 excluded after rehydration"
    );
    let first_rehydrated = rehydrated_tasks[0]["id"].as_str().unwrap();
    assert!(
        first_rehydrated.ends_with(&t3_id),
        "pinned t3 first after rehydration"
    );
}

// ── T066: Edge case — defer_until in the past ───────────────────

#[tokio::test]
async fn t066_past_defer_until_immediately_eligible() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    std::fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let tmem_dir = workspace.path().join(".tmem");
    std::fs::create_dir_all(&tmem_dir).expect("create .tmem");

    // Seed a task with defer_until in the past via markdown
    std::fs::write(
        tmem_dir.join("tasks.md"),
        r"# Tasks

## task:past_defer

---
id: task:past_defer
title: Past deferred task
status: todo
priority: p2
issue_type: task
pinned: false
defer_until: 2020-01-01T00:00:00+00:00
created_at: 2026-02-05T10:00:00+00:00
updated_at: 2026-02-05T10:00:00+00:00
---

This task was deferred to a date that has already passed.
",
    )
    .expect("write tasks.md with past defer_until");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_str().unwrap().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    // The task with past defer_until should be immediately eligible
    let ready = tools::dispatch(state.clone(), "get_ready_work", Some(json!({})))
        .await
        .expect("get_ready_work");
    let tasks = ready["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 1, "past-deferred task immediately eligible");
    assert!(
        tasks[0]["id"].as_str().unwrap().contains("past_defer"),
        "correct task returned"
    );
}
