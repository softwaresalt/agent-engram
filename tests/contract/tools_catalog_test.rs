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

/// Verifies the `impact_analysis` summary advertises both code and Power BI graph
/// roots so MCP clients can select it for either workflow.
#[test]
fn impact_analysis_summary_documents_code_and_powerbi() {
    let tools = tools_catalog::all_tools();
    let impact = tools
        .iter()
        .find(|tool| tool.name.as_ref() == "impact_analysis")
        .expect("impact_analysis must be present in catalog");
    let summary = impact.description.as_deref().unwrap_or_default();

    assert!(
        summary.contains("code call graph"),
        "summary must mention the code call graph: {summary}"
    );
    assert!(
        summary.contains("Power BI"),
        "summary must mention Power BI graph support: {summary}"
    );
    assert!(
        summary.contains("powerbi_node_id"),
        "summary must reference the Power BI selector: {summary}"
    );
}

/// Verifies the `impact_analysis` depth parameter describes both traversal
/// surfaces and the Power BI selector remains documented in the schema.
#[test]
fn impact_analysis_params_document_powerbi_selector_and_depth() {
    let tools = tools_catalog::all_tools();
    let impact = tools
        .iter()
        .find(|tool| tool.name.as_ref() == "impact_analysis")
        .expect("impact_analysis must be present in catalog");
    let props = impact
        .input_schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .expect("impact_analysis schema must have properties");
    let depth = props
        .get("depth")
        .and_then(|value| value.get("description"))
        .and_then(serde_json::Value::as_str)
        .expect("depth must have a description");
    let powerbi_node_id = props
        .get("powerbi_node_id")
        .and_then(|value| value.get("description"))
        .and_then(serde_json::Value::as_str)
        .expect("powerbi_node_id must have a description");

    assert!(
        depth.contains("code call graph") && depth.contains("Power BI dependency graph"),
        "depth description must cover both graph surfaces: {depth}"
    );
    assert!(
        powerbi_node_id.contains("Power BI node id"),
        "powerbi_node_id description must document the selector: {powerbi_node_id}"
    );
}

/// 095-F U8 (cycle-5 F6 / AR-26): the `query_graph` tool description must
/// advertise the `lineage_derives_from` edge type and enumerate the full
/// traversable namespace (code / backlog / powerbi / lineage) so agents can
/// discover the notebook data-lineage subgraph.
#[test]
fn query_graph_advertises_lineage_and_full_edge_namespace() {
    let tools = tools_catalog::all_tools();
    let query_graph = tools
        .iter()
        .find(|tool| tool.name.as_ref() == "query_graph")
        .expect("query_graph must be present in catalog");
    let desc = query_graph.description.as_deref().unwrap_or_default();

    assert!(
        desc.contains("lineage_derives_from"),
        "description must advertise the lineage_derives_from edge type: {desc}"
    );
    for namespace in ["code", "backlog", "powerbi", "lineage"] {
        assert!(
            desc.contains(namespace),
            "description must enumerate the '{namespace}' edge namespace: {desc}"
        );
    }
}
