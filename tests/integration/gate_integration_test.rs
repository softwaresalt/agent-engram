//! Integration tests for dependency gate enforcement (User Story 1).
//!
//! Tests use an embedded `SurrealDB` instance to verify the BFS gate logic with
//! real database state. Scenarios: S010 (multiple blockers) and S012 (perf).

use std::time::Instant;

use engram::db::queries::Queries;
use engram::db::schema;
use engram::models::graph::DependencyType;
use engram::models::task::{Task, TaskStatus};
use engram::services::gate;

/// Creates a minimal Task value with the given id and status.
fn make_task(id: &str, status: TaskStatus) -> Task {
    let now = chrono::Utc::now();
    Task {
        id: id.to_string(),
        title: format!("Gate test task {id}"),
        status,
        work_item_id: None,
        description: String::new(),
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
        created_at: now,
        updated_at: now,
    }
}

/// Sets up a throwaway embedded `SurrealDB` instance for gate tests.
async fn setup_db(dir: &std::path::Path) -> Queries {
    let db =
        surrealdb::Surreal::new::<surrealdb::engine::local::SurrealKv>(dir.to_str().expect("path"))
            .await
            .expect("embedded db");
    db.use_ns("engram")
        .use_db("gate_test")
        .await
        .expect("ns/db");
    db.query(schema::DEFINE_TASK).await.expect("schema task");
    db.query(schema::DEFINE_RELATIONSHIPS)
        .await
        .expect("schema relationships");
    Queries::new(db)
}

/// S010: Multiple direct `hard_blocker` prerequisites all surface as blockers.
///
/// Graph: `target` → `blocker_1`, `blocker_2`, `blocker_3` (all `hard_blocker` edges).
///
/// All three blockers are `todo` so `check_blockers` must return 3 entries.
#[tokio::test]
async fn t019_multiple_direct_blockers_all_returned() {
    let dir = tempfile::tempdir().expect("tempdir");
    let q = setup_db(dir.path()).await;

    q.upsert_task(&make_task("target", TaskStatus::Todo))
        .await
        .expect("insert target");
    for i in 1..=3 {
        q.upsert_task(&make_task(&format!("blocker-{i}"), TaskStatus::Todo))
            .await
            .expect("insert blocker");
        q.create_dependency(
            "target",
            &format!("blocker-{i}"),
            DependencyType::HardBlocker,
        )
        .await
        .expect("create dependency");
    }

    let blockers = q
        .check_blockers("target")
        .await
        .expect("check_blockers must succeed");

    assert_eq!(
        blockers.len(),
        3,
        "all 3 direct hard_blocker prerequisites must be returned"
    );
    for b in &blockers {
        assert_eq!(
            b["dependency_type"], "hard_blocker",
            "each blocker must have dependency_type=hard_blocker"
        );
        assert_eq!(
            b["transitively_blocks"],
            serde_json::json!(false),
            "direct blockers must have transitively_blocks=false"
        );
    }
}

/// S011: When both hard_blocker and soft_dependency edges exist, the hard failure
/// takes precedence and soft warnings are suppressed.
///
/// Graph: `target` → `hard-dep` (hard_blocker, todo), `target` → `soft-dep` (soft_dependency, todo).
///
/// The gate must return blockers for `hard-dep` and an empty warnings list.
#[tokio::test]
async fn t021_mixed_hard_soft_hard_takes_precedence() {
    let dir = tempfile::tempdir().expect("tempdir");
    let q = setup_db(dir.path()).await;

    q.upsert_task(&make_task("target", TaskStatus::Todo))
        .await
        .expect("insert target");
    q.upsert_task(&make_task("hard-dep", TaskStatus::Todo))
        .await
        .expect("insert hard-dep");
    q.upsert_task(&make_task("soft-dep", TaskStatus::Todo))
        .await
        .expect("insert soft-dep");

    q.create_dependency("target", "hard-dep", DependencyType::HardBlocker)
        .await
        .expect("create hard dependency");
    q.create_dependency("target", "soft-dep", DependencyType::SoftDependency)
        .await
        .expect("create soft dependency");

    let result = gate::evaluate("target", &q)
        .await
        .expect("gate::evaluate must succeed");

    assert!(
        result.is_blocked(),
        "gate must be blocked when hard_blocker is incomplete"
    );
    assert_eq!(
        result.blockers.len(),
        1,
        "exactly one hard blocker expected"
    );
    assert!(
        result.warnings.is_empty(),
        "soft warnings must be suppressed when hard blockers exist (S011)"
    );
}

/// S012: Gate evaluation for a 100-task chain completes within the latency budget.
///
/// Chain: `chain-0` → `chain-1` → … → `chain-99` (all `hard_blocker` edges).
/// All tasks are `todo`. `gate::evaluate("chain-0", …)` must return 99 blockers
/// and complete within 50 ms (release) / 30 000 ms (debug — `SurrealKv` overhead).
#[tokio::test]
async fn t020_gate_performance_100_task_chain() {
    let dir = tempfile::tempdir().expect("tempdir");
    let q = setup_db(dir.path()).await;

    // Build a 100-task linear chain.
    for i in 0..100_u32 {
        q.upsert_task(&make_task(&format!("chain-{i}"), TaskStatus::Todo))
            .await
            .expect("insert chain task");
    }
    for i in 0..99_u32 {
        q.create_dependency(
            &format!("chain-{i}"),
            &format!("chain-{}", i + 1),
            DependencyType::HardBlocker,
        )
        .await
        .expect("create chain dependency");
    }

    let start = Instant::now();
    let gate_result = gate::evaluate("chain-0", &q)
        .await
        .expect("gate::evaluate must succeed");
    let elapsed = start.elapsed();

    // Threshold: generous for debug builds due to SurrealKv embedded overhead.
    let threshold_ms: u128 = if cfg!(debug_assertions) { 30_000 } else { 50 };

    println!(
        "T020 gate 100-task chain: {:?} ({} blockers, target <{threshold_ms}ms)",
        elapsed,
        gate_result.blockers.len()
    );

    assert!(
        gate_result.is_blocked(),
        "gate must be blocked when chain-1..99 are all todo"
    );
    assert_eq!(
        gate_result.blockers.len(),
        99,
        "BFS must collect all 99 transitive hard_blocker prerequisites"
    );
    assert!(
        elapsed.as_millis() < threshold_ms,
        "gate evaluation took {}ms, target <{threshold_ms}ms",
        elapsed.as_millis()
    );
}
