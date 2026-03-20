use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::errors::codes::{QUERY_EMPTY, QUERY_TOO_LONG, WORKSPACE_NOT_SET};
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools;

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

// ── T073: query_memory contract tests ────────────────────────────

#[test]
async fn contract_query_memory_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "query": "user authentication",
    }));

    let err = tools::dispatch(state, "query_memory", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_query_memory_rejects_long_query() {
    // Build a state with workspace set so we get past the workspace check.
    let state = Arc::new(AppState::new(10));
    let snapshot = engram::server::state::WorkspaceSnapshot {
        workspace_id: "test_ws".to_string(),
        path: "/tmp/test-repo".to_string(),
        task_count: 0,
        context_count: 0,
        last_flush: None,
        stale_files: false,
        connection_count: 1,
        file_mtimes: std::collections::HashMap::new(),
    };
    state.set_workspace(snapshot).await.expect("set workspace");

    // Query exceeding 2000 chars ≈ 500+ tokens
    let long_query = "a ".repeat(1500);
    let params = Some(json!({
        "query": long_query,
    }));

    let err = tools::dispatch(state, "query_memory", params)
        .await
        .expect_err("expected query too long error");

    let code = err.to_response().error.code;
    assert_eq!(code, QUERY_TOO_LONG);
}

#[test]
async fn contract_query_memory_returns_results_array() {
    // With an active workspace (even empty), query_memory should return
    // a JSON object with a `results` array.
    let state = Arc::new(AppState::new(10));
    let snapshot = engram::server::state::WorkspaceSnapshot {
        workspace_id: "test_ws_results".to_string(),
        path: "/tmp/test-repo-results".to_string(),
        task_count: 0,
        context_count: 0,
        last_flush: None,
        stale_files: false,
        connection_count: 1,
        file_mtimes: std::collections::HashMap::new(),
    };
    state.set_workspace(snapshot).await.expect("set workspace");

    let params = Some(json!({
        "query": "user login",
    }));

    let result = tools::dispatch(state, "query_memory", params).await;
    // May succeed with empty results or fail with ModelNotLoaded on keyword-only.
    // Either way, it should not return WorkspaceNotSet.
    match result {
        Ok(val) => {
            assert!(
                val.get("results").is_some(),
                "response must contain `results` key"
            );
            assert!(val["results"].is_array(), "`results` must be an array");
        }
        Err(e) => {
            // Acceptable: ModelNotLoaded or DatabaseError (no real DB in unit test)
            let code = e.to_response().error.code;
            assert_ne!(code, WORKSPACE_NOT_SET, "must not be WorkspaceNotSet");
        }
    }
}

// ── Statistics contract tests (T067) ─────────────────────────────────────────

#[test]
async fn contract_get_workspace_statistics_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({}));

    let err = tools::dispatch(state, "get_workspace_statistics", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

// ── T036: map_code contract tests ────────────────────────────────────

#[test]
async fn contract_map_code_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "symbol_name": "my_function",
    }));

    let err = tools::dispatch(state, "map_code", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_map_code_empty_graph_uses_fallback() {
    // With an active workspace but no indexed code, map_code should
    // fall back to vector search and return an empty result set (not an error).
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("map_code_empty"))
        .await
        .expect("set workspace");

    let params = Some(json!({
        "symbol_name": "nonexistent_function",
    }));

    let result = tools::dispatch(state, "map_code", params).await;
    match result {
        Ok(val) => {
            // Should have fallback_used = true and empty matches
            assert_eq!(val["fallback_used"].as_bool(), Some(true));
            assert_eq!(val["truncated"].as_bool(), Some(false));
        }
        Err(e) => {
            // Acceptable: ModelNotLoaded or DatabaseError (no real embedding model in unit test)
            let code = e.to_response().error.code;
            assert_ne!(code, WORKSPACE_NOT_SET, "must not be WorkspaceNotSet");
        }
    }
}

// ── T037: list_symbols contract tests ────────────────────────────────

#[test]
async fn contract_list_symbols_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({}));

    let err = tools::dispatch(state, "list_symbols", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_list_symbols_empty_graph_returns_error() {
    use engram::errors::codes::SYMBOL_NOT_FOUND;

    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("list_symbols_empty"))
        .await
        .expect("set workspace");

    let params = Some(json!({"name_prefix": "nonexistent"}));

    let err = tools::dispatch(state, "list_symbols", params)
        .await
        .expect_err("expected symbol not found error for filtered empty graph");

    let code = err.to_response().error.code;
    assert_eq!(code, SYMBOL_NOT_FOUND);
}

// ─── Phase 7: Unified Semantic Search ───────────────────────────────────────

#[test]
async fn contract_unified_search_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "query": "billing logic" }));

    let err = tools::dispatch(state, "unified_search", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_unified_search_rejects_empty_query() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("unified_search_empty"))
        .await
        .expect("set workspace");

    // Empty string
    let params = Some(json!({ "query": "" }));
    let err = tools::dispatch(state.clone(), "unified_search", params)
        .await
        .expect_err("expected empty query error");
    let code = err.to_response().error.code;
    assert_eq!(code, QUERY_EMPTY, "empty query should return 4004");

    // Whitespace-only string
    let params = Some(json!({ "query": "   " }));
    let err = tools::dispatch(state, "unified_search", params)
        .await
        .expect_err("expected empty query error for whitespace");
    let code = err.to_response().error.code;
    assert_eq!(
        code, QUERY_EMPTY,
        "whitespace-only query should return 4004"
    );
}

// ─── Phase 8: Impact Analysis Queries ───────────────────────────────────────

#[test]
async fn contract_impact_analysis_requires_workspace() {
    use engram::errors::codes::WORKSPACE_NOT_SET;

    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "symbol_name": "EngramError" }));

    let err = tools::dispatch(state, "impact_analysis", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_impact_analysis_symbol_not_found() {
    use engram::errors::codes::SYMBOL_NOT_FOUND;

    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("impact_analysis_not_found"))
        .await
        .expect("set workspace");

    let params = Some(json!({ "symbol_name": "NonExistentSymbol" }));

    let err = tools::dispatch(state, "impact_analysis", params)
        .await
        .expect_err("expected symbol not found error");

    let code = err.to_response().error.code;
    assert_eq!(code, SYMBOL_NOT_FOUND);
}
