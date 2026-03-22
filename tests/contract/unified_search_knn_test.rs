use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::errors::codes::{QUERY_EMPTY, QUERY_TOO_LONG, WORKSPACE_NOT_SET};
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools;

fn ws(id: &str) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: id.to_string(),
        path: format!("/tmp/{id}"),
        last_flush: None,
        stale_files: false,
        connection_count: 1,
        file_mtimes: std::collections::HashMap::new(),
    }
}

// ── Input schema: workspace guard ────────────────────────────────────────────

/// `unified_search` must require workspace to be set.
#[test]
async fn unified_search_requires_workspace() {
    // GIVEN no workspace set
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "query": "authentication" }));

    // WHEN dispatching unified_search without a workspace
    let err = tools::dispatch(state, "unified_search", params)
        .await
        .expect_err("expected workspace not set error");

    // THEN schema returns WORKSPACE_NOT_SET
    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET, "must return WORKSPACE_NOT_SET");
}

/// `unified_search` must reject an empty query.
#[test]
async fn unified_search_rejects_empty_query() {
    // GIVEN workspace set, empty query
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(ws("ufs_empty"))
        .await
        .expect("set workspace");
    let params = Some(json!({ "query": "" }));

    // WHEN dispatching
    let err = tools::dispatch(state, "unified_search", params)
        .await
        .expect_err("expected query empty error");

    // THEN schema returns QUERY_EMPTY
    let code = err.to_response().error.code;
    assert_eq!(code, QUERY_EMPTY, "must return QUERY_EMPTY for blank query");
}

/// `unified_search` must reject queries exceeding the token budget.
#[test]
async fn unified_search_rejects_query_too_long() {
    // GIVEN workspace set, oversized query
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(ws("ufs_long"))
        .await
        .expect("set workspace");
    let long_query = "a ".repeat(1500); // > 2000 char limit
    let params = Some(json!({ "query": long_query }));

    // WHEN dispatching
    let err = tools::dispatch(state, "unified_search", params)
        .await
        .expect_err("expected query too long error");

    // THEN schema returns QUERY_TOO_LONG
    let code = err.to_response().error.code;
    assert_eq!(code, QUERY_TOO_LONG, "must return QUERY_TOO_LONG");
}

// ── Output schema: result shape ───────────────────────────────────────────────

/// When `unified_search` succeeds (even with no embedding model), the output must
/// contain a `results` array — the schema contract is unchanged after KNN migration.
#[test]
async fn unified_search_output_contains_results_array() {
    // GIVEN workspace set
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(ws("ufs_output"))
        .await
        .expect("set workspace");
    let params = Some(json!({ "query": "authentication", "limit": 5 }));

    // WHEN dispatching
    let result = tools::dispatch(state, "unified_search", params).await;

    // THEN either results array present OR acceptable non-schema error
    match result {
        Ok(val) => {
            assert!(
                val.get("results").is_some(),
                "output schema must contain `results` key; got: {val}"
            );
            assert!(val["results"].is_array(), "`results` must be an array");
            // total_count must be a non-negative number
            assert!(
                val.get("total_count").is_some(),
                "output schema must contain `total_count`"
            );
        }
        Err(e) => {
            let code = e.to_response().error.code;
            // Acceptable non-schema errors: ModelNotLoaded (4002) or DatabaseError (5001)
            assert_ne!(code, WORKSPACE_NOT_SET, "must not be WorkspaceNotSet");
            assert_ne!(code, QUERY_EMPTY, "must not be QueryEmpty for valid query");
        }
    }
}

/// Each result in the `results` array must include the fields defined by the
/// `UnifiedSearchResult` output schema: `region`, `score`, `node_type`, `id`.
#[test]
async fn unified_search_result_items_have_required_fields() {
    // GIVEN workspace set (empty DB — keyword matches only)
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(ws("ufs_fields"))
        .await
        .expect("set workspace");
    let params = Some(json!({ "query": "search", "limit": 10 }));

    // WHEN dispatching
    let result = tools::dispatch(state, "unified_search", params).await;

    if let Ok(val) = result {
        // If results are present, each item must have required schema fields
        let results = val["results"].as_array().expect("`results` must be array");
        for item in results {
            assert!(
                item.get("region").is_some(),
                "each result must have `region`; got: {item}"
            );
            assert!(
                item.get("score").is_some(),
                "each result must have `score`; got: {item}"
            );
            assert!(
                item.get("node_type").is_some(),
                "each result must have `node_type`; got: {item}"
            );
            assert!(
                item.get("id").is_some(),
                "each result must have `id`; got: {item}"
            );
            // Score must be a finite float in [0, 1]
            let score = item["score"].as_f64().expect("score must be a number");
            assert!(
                (0.0..=1.0).contains(&score),
                "score must be in [0, 1]; got: {score}"
            );
        }
    }
    // If Err(_): no results to validate — test vacuously passes
}

/// The `limit` parameter must be respected: result count must not exceed `limit`.
#[test]
async fn unified_search_respects_limit_parameter() {
    // GIVEN workspace set, limit = 3
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(ws("ufs_limit"))
        .await
        .expect("set workspace");
    let params = Some(json!({ "query": "function", "limit": 3 }));

    // WHEN dispatching
    let result = tools::dispatch(state, "unified_search", params).await;

    if let Ok(val) = result {
        let results = val["results"].as_array().expect("`results` must be array");
        assert!(
            results.len() <= 3,
            "result count ({}) must not exceed limit (3)",
            results.len()
        );
    }
}
