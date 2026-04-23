//! Contract tests for background offline-change scan (029-F WS-6).
//!
//! Verifies the contract for `set_workspace` / `get_workspace_status` after
//! background scan is implemented:
//! - `WorkspaceBinding` includes `pending_scan` field
//! - `set_workspace` returns within 500 ms (bind latency only; heavy work is async)
//! - `get_workspace_status` includes `scan_status` field
//!
//! **Red phase**: `pending_scan` serialises as `false` and `scan_status`
//! serialises as `null` — both assertions that check for active scan state
//! fail. Latency and field-presence tests may pass early.

use std::fs;
use std::sync::Arc;
use std::time::Instant;

use serde_json::{Value, json};

use engram::server::state::AppState;
use engram::tools;

/// `set_workspace` response must include a `pending_scan` field.
///
/// Red phase: field is present (serialised as `false`) — this test may pass in
/// red phase but the _value_ test below confirms it stays red until WS-6 ships.
#[tokio::test]
async fn contract_set_workspace_returns_pending_scan_field() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    let result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    assert!(
        result.get("pending_scan").is_some(),
        "set_workspace response must include pending_scan field"
    );
}

/// After `set_workspace` a background scan should be queued: `pending_scan`
/// must be `true` when offline changes exist.
///
/// Red phase: stub always returns `false` → assertion fails.
#[tokio::test]
async fn contract_set_workspace_pending_scan_true_when_offline_changes_exist() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    // Write a file that constitutes an offline change
    fs::write(workspace.path().join("main.rs"), b"fn main() {}").expect("write file");
    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    let result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let pending_scan = result
        .get("pending_scan")
        .and_then(Value::as_bool)
        .expect("pending_scan field must be a bool");

    assert!(
        pending_scan,
        "pending_scan must be true when offline changes exist after workspace binding"
    );
}

/// `set_workspace` must return within 500 ms (bind latency SLA).
/// Heavy post-bind work (DB connect, hydration, scan) runs asynchronously.
///
/// Red phase: timing passes (the stub is fast), but this is the standing SLA
/// that must be maintained through all green-phase implementation.
#[tokio::test]
async fn contract_set_workspace_returns_within_500ms() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    let start = Instant::now();
    let _result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");
    let elapsed_ms = start.elapsed().as_millis();

    assert!(
        elapsed_ms < 500,
        "set_workspace must return within 500 ms (bind SLA); took {elapsed_ms} ms"
    );
}

/// `get_workspace_status` response must include a `scan_status` field.
///
/// Red phase: field serialises as `null` — this checks presence, so it passes
/// only if the JSON key is present. Because `Option<ScanProgress>` serialises
/// to the JSON key with value `null`, the key IS present and this test passes
/// in red phase. The next test checks for an _active_ scan value and fails.
#[tokio::test]
async fn contract_get_workspace_status_includes_scan_status_field() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let status = tools::dispatch(
        state.clone(),
        "get_workspace_status",
        Some(json!({ "path": path })),
    )
    .await
    .expect("get_workspace_status should succeed");

    assert!(
        status.get("scan_status").is_some(),
        "get_workspace_status response must include scan_status field"
    );
}

/// After binding with offline changes, `get_workspace_status.scan_status`
/// must reflect an active or recently queued scan.
///
/// Red phase: `scan_status` is `null` — assertion fails.
#[tokio::test]
async fn contract_get_workspace_status_scan_status_reflects_queued_scan() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    fs::write(workspace.path().join("main.rs"), b"fn main() {}").expect("write file");
    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let status = tools::dispatch(
        state.clone(),
        "get_workspace_status",
        Some(json!({ "path": path })),
    )
    .await
    .expect("get_workspace_status should succeed");

    let scan_status = status
        .get("scan_status")
        .expect("scan_status field present");

    assert!(
        !scan_status.is_null(),
        "scan_status must be a ScanProgress object when a scan was queued, not null"
    );
}
