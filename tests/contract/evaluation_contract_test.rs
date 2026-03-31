//! Contract tests for the evaluation MCP tool (TASK-017.04.01).
//!
//! Validates that `get_evaluation_report` returns a well-formed
//! [`EvaluationReport`] JSON structure with correct field types.

use std::fs;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::test;

use engram::models::config::WorkspaceConfig;
use engram::models::metrics::UsageEvent;
use engram::server::state::AppState;
use engram::tools;

/// Helper: set up a workspace and write seed usage events for the given branch.
async fn setup_workspace_with_events(events: &[UsageEvent]) -> (Arc<AppState>, tempfile::TempDir) {
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    state
        .set_workspace_config(Some(WorkspaceConfig::default()))
        .await;

    // Write events as NDJSON to the metrics file.
    if !events.is_empty() {
        let metrics_dir = workspace
            .path()
            .join(".engram")
            .join("metrics")
            .join("main");
        fs::create_dir_all(&metrics_dir).expect("create metrics dir");
        let events_path = metrics_dir.join("usage.jsonl");
        let content: String = events
            .iter()
            .map(|e| serde_json::to_string(e).expect("serialize event"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&events_path, content).expect("write events file");
    }

    (state, workspace)
}

fn make_event(tool: &str, tokens: u64, agent_role: Option<&str>) -> UsageEvent {
    UsageEvent {
        tool_name: tool.to_string(),
        timestamp: "2026-03-30T12:00:00Z".to_string(),
        response_bytes: tokens * 4,
        estimated_tokens: tokens,
        symbols_returned: 5,
        results_returned: 5,
        branch: "main".to_string(),
        connection_id: None,
        agent_role: agent_role.map(String::from),
        outcome: "success".to_string(),
    }
}

// ── C017-01: Tool is registered and returns correct JSON shape ───────────────

/// C017-01: `get_evaluation_report` returns a JSON object with all required
/// top-level fields: `branch`, `efficiency_score`, `agents`, `anomalies`,
/// `recommendations`, `evaluated_at`.
#[test]
async fn c017_01_evaluation_report_has_required_fields() {
    let events = vec![
        make_event("list_symbols", 200, Some("doc-ops")),
        make_event("unified_search", 400, Some("doc-ops")),
    ];
    let (state, _workspace) = setup_workspace_with_events(&events).await;

    let result = tools::dispatch(state.clone(), "get_evaluation_report", Some(json!({})))
        .await
        .expect("get_evaluation_report must succeed");

    assert!(result.get("branch").is_some(), "must have 'branch' field");
    assert!(
        result.get("efficiency_score").is_some(),
        "must have 'efficiency_score' field"
    );
    assert!(result.get("agents").is_some(), "must have 'agents' field");
    assert!(
        result.get("anomalies").is_some(),
        "must have 'anomalies' field"
    );
    assert!(
        result.get("recommendations").is_some(),
        "must have 'recommendations' field"
    );
    assert!(
        result.get("evaluated_at").is_some(),
        "must have 'evaluated_at' field"
    );
}

// ── C017-02: efficiency_score is in range 0–100 ──────────────────────────────

/// C017-02: `efficiency_score` is an integer in the range 0–100 inclusive.
#[test]
async fn c017_02_efficiency_score_in_range() {
    let events = vec![
        make_event("list_symbols", 200, Some("doc-ops")),
        make_event("map_code", 300, Some("doc-ops")),
        make_event("unified_search", 400, Some("doc-ops")),
    ];
    let (state, _workspace) = setup_workspace_with_events(&events).await;

    let result = tools::dispatch(state.clone(), "get_evaluation_report", Some(json!({})))
        .await
        .expect("get_evaluation_report must succeed");

    let score = result
        .get("efficiency_score")
        .and_then(Value::as_u64)
        .expect("efficiency_score must be a non-negative integer");

    assert!(score <= 100, "efficiency_score {score} must not exceed 100");
}

// ── C017-03: agents array contains per-agent data ────────────────────────────

/// C017-03: The `agents` array contains entries with `agent_role`,
/// `total_calls`, and `total_tokens` fields.
#[test]
async fn c017_03_agents_have_required_subfields() {
    let events = vec![
        make_event("list_symbols", 200, Some("doc-ops")),
        make_event("map_code", 300, Some("rust-engineer")),
    ];
    let (state, _workspace) = setup_workspace_with_events(&events).await;

    let result = tools::dispatch(state.clone(), "get_evaluation_report", Some(json!({})))
        .await
        .expect("get_evaluation_report must succeed");

    let agents = result
        .get("agents")
        .and_then(Value::as_array)
        .expect("agents must be an array");

    assert_eq!(agents.len(), 2, "should have 2 agent entries");

    for agent in agents {
        assert!(
            agent.get("agent_role").is_some(),
            "agent must have agent_role"
        );
        assert!(
            agent.get("total_calls").is_some(),
            "agent must have total_calls"
        );
        assert!(
            agent.get("total_tokens").is_some(),
            "agent must have total_tokens"
        );
    }
}

// ── C017-04: No workspace returns 1003 ──────────────────────────────────────

/// C017-04: When no workspace is bound, `get_evaluation_report` returns
/// error code 1003 `WorkspaceNotSet`.
#[test]
async fn c017_04_no_workspace_returns_workspace_not_set() {
    let state = Arc::new(AppState::new(10));

    let err = tools::dispatch(state.clone(), "get_evaluation_report", Some(json!({})))
        .await
        .unwrap_err();

    assert_eq!(
        err.to_response().error.code,
        1003,
        "must return 1003 WorkspaceNotSet when no workspace bound"
    );
}

// ── C017-05: Tool catalog discoverability ────────────────────────────────────

/// C017-05: `get_evaluation_report` must be discoverable in the static MCP tool catalog.
///
/// Validates that the tool appears in [`engram::shim::tools_catalog::all_tools`] so that
/// MCP clients calling `tools/list` can discover it. This guards Principle II (MCP
/// Protocol Fidelity): all tools must be unconditionally visible.
#[test]
async fn c017_05_evaluation_report_discoverable_in_tool_catalog() {
    let tools = engram::shim::tools_catalog::all_tools();
    let found = tools
        .iter()
        .any(|t| t.name.as_ref() == "get_evaluation_report");
    assert!(
        found,
        "get_evaluation_report must be listed in all_tools() for MCP discoverability"
    );
}
