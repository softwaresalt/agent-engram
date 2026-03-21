//! Integration tests for enhanced task management features.
//!
//! Most tests in this file called deleted tools (Phase 1 cleanup).
//! Only config rehydration tests remain.

use std::sync::Arc;

use serde_json::json;

use engram::server::state::AppState;
use engram::tools;

// ── T087: Config rehydration integration test ──────────────────

#[tokio::test]
async fn t087_rehydrate_after_config_change_and_missing_config() {
    // Part 1: Set workspace with config, verify values, then change config
    // and re-bind, verify updated values.
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    std::fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let engram_dir = workspace.path().join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");

    // Initial config: batch.max_size=10
    std::fs::write(
        engram_dir.join("config.toml"),
        r"
[batch]
max_size = 10
",
    )
    .expect("write initial config");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("initial set_workspace");

    let cfg1 = state
        .workspace_config()
        .await
        .expect("config after first bind");
    assert_eq!(
        cfg1.batch.max_size, 10,
        "initial batch max_size should be 10"
    );

    // Update config on disk: batch.max_size=20
    std::fs::write(
        engram_dir.join("config.toml"),
        r"
[batch]
max_size = 20
",
    )
    .expect("write updated config");

    // Re-bind to pick up changes
    let state2 = Arc::new(AppState::new(10));
    tools::dispatch(
        state2.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("re-bind set_workspace");

    let cfg2 = state2
        .workspace_config()
        .await
        .expect("config after re-bind");
    assert_eq!(
        cfg2.batch.max_size, 20,
        "updated batch max_size should be 20"
    );

    // Part 2: Remove config.toml, re-bind, defaults should apply
    std::fs::remove_file(engram_dir.join("config.toml")).expect("remove config.toml");

    let state3 = Arc::new(AppState::new(10));
    tools::dispatch(
        state3.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace without config.toml");

    let cfg3 = state3
        .workspace_config()
        .await
        .expect("config with defaults");
    assert_eq!(cfg3.batch.max_size, 100, "defaults: batch max_size=100");
    assert_eq!(
        cfg3.compaction.threshold_days, 7,
        "defaults: threshold_days=7"
    );
    assert_eq!(cfg3.default_priority, "p2", "defaults: priority=p2");
}
