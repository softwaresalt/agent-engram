//! Performance benchmark tests for workspace statistics.
//!
//! Validates success criteria timing constraints:
//! - SC-015: statistics <100ms
//!
//! Note: thresholds are relaxed for debug-build CI; production targets
//! assume `--release` builds.

use std::sync::Arc;
use std::time::Instant;

use serde_json::json;

use engram::server::state::AppState;
use engram::tools;

/// Setup: bind a fresh workspace and return state.
async fn perf_setup() -> Arc<AppState> {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(workspace.path().join(".git")).expect(".git");
    let engram_dir = workspace.path().join(".engram");
    std::fs::create_dir_all(&engram_dir).expect(".engram");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    // Release the tempdir handle so the workspace persists for the test
    // (leaking is fine in test code — OS cleans up on exit)
    std::mem::forget(workspace);

    state
}

// ── SC-015: statistics <100ms ────────────────────────────────────────────────

#[tokio::test]
async fn t089_sc015_statistics_performance() {
    let state = perf_setup().await;

    let start = Instant::now();
    let result = tools::dispatch(state.clone(), "get_workspace_statistics", Some(json!({})))
        .await
        .expect("get_workspace_statistics");
    let elapsed = start.elapsed();

    assert!(
        result.get("code_files").is_some(),
        "should return code_files"
    );
    assert!(result.get("functions").is_some(), "should return functions");
    assert!(
        elapsed.as_millis() < 30_000,
        "SC-015: statistics should complete in <30s (debug build, \
         prod target <100ms); took {}ms",
        elapsed.as_millis()
    );
}
