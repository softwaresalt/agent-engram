/// Harness for agent-engram-gsa.1.5: Update MCP tool registration in `list_tools`.
///
/// Verifies that all 29 removed task management tools are absent from the shim's
/// `all_tools()` catalog, and that only the retained code-intelligence tools remain.
///
/// Before implementation: `all_tools()` returns 43 tools including task management.
/// After implementation: `all_tools()` returns only the ~14-16 retained tools.
use engram::shim::tools_catalog;

const REMOVED_TOOLS: &[&str] = &[
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
    "link_task_to_code",
    "unlink_task_from_code",
];

const RETAINED_TOOLS: &[&str] = &[
    "set_workspace",
    "get_daemon_status",
    "get_workspace_status",
    "flush_state",
    "query_memory",
    "get_workspace_statistics",
    "index_workspace",
    "sync_workspace",
    "map_code",
    "list_symbols",
    "unified_search",
    "impact_analysis",
    "get_health_report",
    "query_graph",
];

/// Verifies no task management tools appear in the catalog.
#[test]
fn task_management_tools_absent_from_catalog() {
    let tools = tools_catalog::all_tools();
    let names: std::collections::HashSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    for tool in REMOVED_TOOLS {
        assert!(
            !names.contains(tool),
            "task management tool '{tool}' must not appear in all_tools() catalog"
        );
    }
}

/// Verifies all retained code-intelligence tools are present in the catalog.
#[test]
fn retained_tools_present_in_catalog() {
    let tools = tools_catalog::all_tools();
    let names: std::collections::HashSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    for tool in RETAINED_TOOLS {
        assert!(
            names.contains(tool),
            "retained tool '{tool}' must be present in all_tools() catalog"
        );
    }
}

/// Verifies `TOOL_COUNT` matches the actual catalog length.
#[test]
fn tool_count_constant_matches_catalog() {
    let tools = tools_catalog::all_tools();
    assert_eq!(
        tools.len(),
        tools_catalog::TOOL_COUNT,
        "TOOL_COUNT must equal all_tools().len() (got {} tools, constant is {})",
        tools.len(),
        tools_catalog::TOOL_COUNT
    );
}
