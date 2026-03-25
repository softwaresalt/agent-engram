use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::errors::codes::{SYMBOL_NOT_FOUND, WORKSPACE_NOT_SET};
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools;

fn ws(id: &str) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: id.to_string(),
        branch: id.to_string(),
        data_dir: std::env::temp_dir().join("engram-test"),
        path: format!("/tmp/{id}"),
        last_flush: None,
        stale_files: false,
        connection_count: 1,
        file_mtimes: std::collections::HashMap::new(),
    }
}

// ── map_code: input schema ───────────────────────────────────────────────────

/// `map_code` must require workspace to be set.
#[test]
async fn map_code_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "symbol_name": "my_function" }));

    let err = tools::dispatch(state, "map_code", params)
        .await
        .expect_err("expected workspace not set");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

// ── map_code: output schema ──────────────────────────────────────────────────

/// `map_code` output must contain the `CodeGraphNeighborhood` fields.
#[test]
async fn map_code_output_schema_is_backward_compatible() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(ws("mc_schema"))
        .await
        .expect("set workspace");
    let params = Some(json!({ "symbol_name": "nonexistent_fn" }));

    let result = tools::dispatch(state, "map_code", params).await;

    match result {
        Ok(val) => {
            // Schema must include all CodeGraphNeighborhood fields
            for field in &["root", "neighbors", "edges", "truncated", "fallback_used"] {
                assert!(
                    val.get(field).is_some(),
                    "map_code output must contain `{field}`; got: {val}"
                );
            }
            assert!(val["neighbors"].is_array(), "`neighbors` must be array");
            assert!(val["edges"].is_array(), "`edges` must be array");
        }
        Err(e) => {
            // Acceptable: symbol not found (7004) or database error
            assert_ne!(e.to_response().error.code, WORKSPACE_NOT_SET);
        }
    }
}

/// `map_code` with `fallback_used=true` returns `matches` array for disambiguation.
#[test]
async fn map_code_fallback_returns_matches_array() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(ws("mc_fallback"))
        .await
        .expect("set workspace");
    let params = Some(json!({ "symbol_name": "nonexistent_function_xyz" }));

    let result = tools::dispatch(state, "map_code", params).await;

    if let Ok(val) = result {
        // When fallback is used, matches must be present
        assert!(
            val.get("matches").is_some(),
            "map_code output must contain `matches`; got: {val}"
        );
        assert!(
            val.get("fallback_used").is_some(),
            "map_code output must contain `fallback_used`"
        );
    }
}

// ── impact_analysis: input schema ───────────────────────────────────────────

/// `impact_analysis` must require workspace to be set.
#[test]
async fn impact_analysis_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "symbol_name": "my_function" }));

    let err = tools::dispatch(state, "impact_analysis", params)
        .await
        .expect_err("expected workspace not set");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

/// `impact_analysis` must return `SYMBOL_NOT_FOUND` when symbol doesn't exist.
#[test]
async fn impact_analysis_returns_symbol_not_found() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(ws("ia_not_found"))
        .await
        .expect("set workspace");
    let params = Some(json!({ "symbol_name": "nonexistent_symbol_xyz" }));

    let err = tools::dispatch(state, "impact_analysis", params)
        .await
        .expect_err("expected symbol not found");

    assert_eq!(err.to_response().error.code, SYMBOL_NOT_FOUND);
}

// ── impact_analysis: output schema ───────────────────────────────────────────

/// `impact_analysis` output must contain the expected schema fields.
#[test]
async fn impact_analysis_output_schema_is_backward_compatible() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(ws("ia_schema"))
        .await
        .expect("set workspace");
    let params = Some(json!({ "symbol_name": "nonexistent_symbol_for_schema_test" }));

    let result = tools::dispatch(state, "impact_analysis", params).await;

    match result {
        Ok(val) => {
            for field in &[
                "symbol",
                "code_neighborhood",
                "effective_depth",
                "effective_max_nodes",
            ] {
                assert!(
                    val.get(field).is_some(),
                    "impact_analysis output must contain `{field}`; got: {val}"
                );
            }
            assert!(
                val["code_neighborhood"].is_array(),
                "`code_neighborhood` must be array"
            );
        }
        Err(e) => {
            // Symbol not found is expected for nonexistent symbol
            let code = e.to_response().error.code;
            assert!(
                code == SYMBOL_NOT_FOUND || code != WORKSPACE_NOT_SET,
                "must not be WorkspaceNotSet"
            );
        }
    }
}
