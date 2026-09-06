use engram::config::StaleStrategy;
use engram::models::config::DaemonMode;
use std::fs;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::test;

use engram::errors::codes::{CONFIG_INVALID_VALUE, WORKSPACE_LIMIT_REACHED};
use engram::errors::{EngramError, IpcError};
use engram::server::state::AppState;
use engram::shim::version::{ENGRAM_PROTOCOL_VERSION, ensure_protocol_compatible};
use engram::tools;

#[test]
async fn contract_set_workspace_returns_hydrated_flag() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    let path = workspace.path().to_string_lossy().to_string();

    let result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    assert_eq!(result.get("hydrated").and_then(Value::as_bool), Some(true));
    let workspace_id = result
        .get("workspace_id")
        .and_then(Value::as_str)
        .expect("workspace_id present");
    assert!(!workspace_id.is_empty(), "workspace_id must not be empty");
}

#[test]
async fn contract_get_daemon_status_reports_counts() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let status = tools::dispatch(state.clone(), "get_daemon_status", None)
        .await
        .expect("get_daemon_status should succeed");

    let active = status
        .get("active_workspaces")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert_eq!(active, 1);
    assert!(
        status
            .get("active_connections")
            .and_then(Value::as_u64)
            .is_some(),
        "active_connections present"
    );
}

#[test]
async fn version_handshake_returns_typed_mismatch_error() {
    let stale_daemon_version = ENGRAM_PROTOCOL_VERSION.saturating_add(1);
    let wrapped_error = EngramError::from(IpcError::VersionMismatch {
        expected: ENGRAM_PROTOCOL_VERSION,
        actual: stale_daemon_version,
    });

    assert!(matches!(
        wrapped_error,
        EngramError::Ipc(IpcError::VersionMismatch { .. })
    ));
    assert!(
        wrapped_error.to_string().contains("respawn"),
        "version mismatch message should carry the remediation hint"
    );

    assert!(matches!(
        ensure_protocol_compatible(stale_daemon_version),
        Err(EngramError::Ipc(IpcError::VersionMismatch { expected, actual }))
            if expected == ENGRAM_PROTOCOL_VERSION && actual == stale_daemon_version
    ));
}

#[test]
async fn contract_get_workspace_status_reports_state() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    let path = workspace.path().to_string_lossy().to_string();
    let canonical_path = engram::db::workspace::canonicalize_workspace(&path)
        .expect("canonicalize_workspace should succeed")
        .display()
        .to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let status = tools::dispatch(state.clone(), "get_workspace_status", None)
        .await
        .expect("get_workspace_status should succeed");

    assert_eq!(
        status.get("path").and_then(Value::as_str),
        Some(canonical_path.as_str())
    );
    assert_eq!(
        status.get("stale_files").and_then(Value::as_bool),
        Some(false)
    );
}

// ─── T112: Workspace limit (FR-009a) ─────────────────────────────────────────

#[test]
async fn contract_set_workspace_enforces_limit() {
    let first = tempfile::tempdir().expect("first workspace");
    let second = tempfile::tempdir().expect("second workspace");
    fs::create_dir(first.path().join(".git")).expect("create first .git");
    fs::create_dir(second.path().join(".git")).expect("create second .git");

    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        1,
        StaleStrategy::Warn,
        20,
        60,
    ));
    let first_path = first.path().to_string_lossy().to_string();
    let second_path = second.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": first_path })),
    )
    .await
    .expect("first workspace should bind");

    let err = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": second_path })),
    )
    .await
    .expect_err("second workspace should exceed limit");

    let payload = err.to_response();
    assert_eq!(payload.error.code, WORKSPACE_LIMIT_REACHED);
    let limit = payload
        .error
        .details
        .as_ref()
        .and_then(|d| d.get("limit"))
        .and_then(Value::as_u64)
        .expect("limit detail present");
    assert_eq!(limit, 1);
}

// ── T081: Config loading contract tests ─────────────────────────

#[test]
async fn contract_no_config_toml_uses_defaults() {
    // When no .engram/config.toml exists, set_workspace should succeed
    // and the config should use built-in defaults.
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    let path = workspace.path().to_string_lossy().to_string();

    let result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed without config.toml");

    assert_eq!(result.get("hydrated").and_then(Value::as_bool), Some(true));

    // Verify defaults are applied: batch max_size=100 by default
    let config = state.workspace_config().await;
    assert!(config.is_some(), "should have config (defaults)");
    let cfg = config.unwrap();
    assert_eq!(cfg.batch.max_size, 100);
}

#[test]
async fn contract_valid_config_populates_workspace_config() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    // Write a valid config.toml
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram dir");
    fs::write(
        engram_dir.join("config.toml"),
        r"
[batch]
max_size = 50
",
    )
    .expect("write config.toml");

    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed with valid config");

    let config = state.workspace_config().await.expect("config should exist");
    assert_eq!(config.batch.max_size, 50);
}

#[test]
async fn contract_toml_parse_error_uses_defaults_with_warning() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    // Write invalid TOML
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram dir");
    fs::write(engram_dir.join("config.toml"), "{{invalid toml").expect("write bad config");

    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    let path = workspace.path().to_string_lossy().to_string();

    // Should still succeed — falls back to defaults
    let result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed even with bad config.toml");

    assert_eq!(result.get("hydrated").and_then(Value::as_bool), Some(true));

    // Defaults should be applied
    let config = state
        .workspace_config()
        .await
        .expect("config should fallback to defaults");
    assert_eq!(config.batch.max_size, 100);
}

#[test]
async fn contract_invalid_config_value_returns_error() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    // Write config.toml with invalid value: batch.max_size=0
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram dir");
    fs::write(
        engram_dir.join("config.toml"),
        r"
[batch]
max_size = 0
",
    )
    .expect("write config with invalid batch size");

    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    let path = workspace.path().to_string_lossy().to_string();

    // set_workspace should fail with CONFIG_INVALID_VALUE
    let result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await;

    assert!(
        result.is_err(),
        "set_workspace should fail on invalid config"
    );
    let err_response = result.unwrap_err().to_response();
    assert_eq!(
        err_response.error.code, CONFIG_INVALID_VALUE,
        "should return CONFIG_INVALID_VALUE (6002)"
    );
}
