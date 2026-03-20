/// Harness for agent-engram-gsa Phase 1: Remove Tool Dispatch and Tool Functions.
///
/// These tests verify that all 26 task management tools (24 task management +
/// 2 task-code bridge tools) have been removed from the dispatch table and return
/// a "not implemented" error rather than a workspace or task error.
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::server::state::AppState;
use engram::tools;

const TASK_MANAGEMENT_TOOLS: &[&str] = &[
    "create_task",
    "update_task",
    "claim_task",
    "release_task",
    "batch_update_tasks",
    "defer_task",
    "undefer_task",
    "pin_task",
    "unpin_task",
    "add_blocker",
    "add_dependency",
    "get_task_graph",
    "add_label",
    "remove_label",
    "add_comment",
    "check_status",
    "get_ready_work",
    "get_compaction_candidates",
    "get_active_context",
    "get_event_history",
    "register_decision",
    "rollback_to_event",
    "apply_compaction",
    "create_collection",
    "add_to_collection",
    "remove_from_collection",
    "get_collection_context",
    // task-code bridge tools (agent-engram-gsa.1.2)
    "link_task_to_code",
    "unlink_task_from_code",
];

/// Verifies that every task management tool returns a "not implemented" error,
/// confirming it has been removed from the dispatch table.
///
/// Before Phase 1 implementation: these tools exist and return `WORKSPACE_NOT_SET`.
/// After Phase 1 implementation: these tools hit `_` and return "not implemented".
#[test]
async fn task_management_tools_not_in_dispatch() {
    let state = Arc::new(AppState::new(10));

    for tool in TASK_MANAGEMENT_TOOLS {
        let result = tools::dispatch(state.clone(), tool, Some(json!({}))).await;
        let err = result.expect_err(&format!("{tool} succeeded — should not be in dispatch"));

        let response = err.to_response();
        assert!(
            response.error.message.contains("not implemented"),
            "{tool} returned unexpected error (expected 'not implemented'): {}",
            response.error.message
        );
    }
}
