//! Contract tests for atomic workspace+config snapshot at dispatch entry (TASK-018).
//!
//! Validates the three acceptance criteria:
//! 1. Workspace binding and config are snapshotted atomically at dispatch entry
//! 2. A concurrent `set_workspace_config` call cannot change policy mid-dispatch
//! 3. Policy-denied calls are recorded in metrics with `outcome=denied`

use std::fs;
use std::sync::Arc;

use serde_json::json;
use serial_test::serial;
use tokio::test;

use engram::errors::codes::POLICY_DENIED;
use engram::models::config::WorkspaceConfig;
use engram::models::policy::{PolicyConfig, UnmatchedPolicy};
use engram::server::state::AppState;
use engram::services::{metrics, policy};
use engram::tools;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Create a temp git workspace, bind it to state, and inject a policy config.
///
/// Returns both the `Arc<AppState>` and the `TempDir` handle.  The caller
/// MUST hold the `TempDir` for the duration of the test — dropping it
/// deletes the workspace.
async fn setup_workspace_with_policy(policy: PolicyConfig) -> (Arc<AppState>, tempfile::TempDir) {
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

    let config = WorkspaceConfig {
        policy,
        ..WorkspaceConfig::default()
    };
    state.set_workspace_config(Some(config)).await;

    (state, workspace)
}

/// Build a deny-all `PolicyConfig`: policy enabled, unmatched=Deny, no rules.
fn deny_all_policy() -> PolicyConfig {
    PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Deny,
        rules: vec![],
    }
}

// ── AC1: Atomic snapshot captures workspace + config ─────────────────────────

/// C018-01: `snapshot_dispatch_context` returns both workspace and config
/// in one atomic `DispatchSnapshot`.
///
/// RED: panics with `unimplemented!` — `snapshot_dispatch_context` is a stub.
#[test]
async fn c018_01_snapshot_captures_workspace_and_config_atomically() {
    // GIVEN a workspace is bound and a deny-all policy config is set
    let (state, _workspace) = setup_workspace_with_policy(deny_all_policy()).await;

    // WHEN snapshot_dispatch_context is called
    let snapshot = state.snapshot_dispatch_context().await;

    // THEN the returned DispatchSnapshot is Some and contains both pieces
    let snap = snapshot.expect("snapshot must be Some when workspace is bound");
    assert!(
        snap.config.policy.enabled,
        "snapshot config must carry the deny-all policy"
    );
    assert_eq!(
        snap.config.policy.unmatched,
        UnmatchedPolicy::Deny,
        "unmatched must be Deny in snapshot"
    );
    assert!(
        !snap.workspace.path.is_empty(),
        "snapshot workspace path must be populated"
    );
}

/// C018-02: `snapshot_dispatch_context` returns `None` when no workspace
/// is bound, even if a config has been loaded.
///
/// RED: panics with `unimplemented!` — `snapshot_dispatch_context` is a stub.
#[test]
async fn c018_02_snapshot_returns_none_without_workspace() {
    // GIVEN an AppState with no workspace bound but config set
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace_config(Some(WorkspaceConfig {
            policy: deny_all_policy(),
            ..WorkspaceConfig::default()
        }))
        .await;

    // WHEN snapshot_dispatch_context is called
    let snapshot = state.snapshot_dispatch_context().await;

    // THEN it returns None because there is no workspace
    assert!(
        snapshot.is_none(),
        "snapshot must be None when no workspace is bound"
    );
}

/// C018-03: When a workspace is bound but no config was loaded, the
/// snapshot defaults to `WorkspaceConfig::default()` (policy disabled).
///
/// RED: panics with `unimplemented!` — `snapshot_dispatch_context` is a stub.
#[test]
async fn c018_03_snapshot_uses_default_config_when_none_set() {
    // GIVEN a workspace is bound with no explicit config
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_string_lossy().to_string() })),
    )
    .await
    .expect("set_workspace must succeed");

    // No set_workspace_config call — config remains None

    // WHEN snapshot_dispatch_context is called
    let snapshot = state.snapshot_dispatch_context().await;

    // THEN the snapshot exists with default (disabled) policy
    let snap = snapshot.expect("snapshot must be Some when workspace is bound");
    assert!(
        !snap.config.policy.enabled,
        "default policy must be disabled"
    );
    assert_eq!(
        snap.config.policy.unmatched,
        UnmatchedPolicy::Deny,
        "default unmatched must be Deny"
    );
}

// ── AC2: Concurrent config change cannot affect mid-dispatch policy ──────────

/// C018-04: A snapshot taken at dispatch entry is immune to concurrent
/// `set_workspace_config` calls.  After the snapshot is created, changing
/// the live config does NOT alter the snapshot's policy.
///
/// RED: panics with `unimplemented!` — `snapshot_dispatch_context` is a stub.
#[test]
async fn c018_04_snapshot_is_immutable_after_config_change() {
    // GIVEN a workspace with deny-all policy, snapshot taken
    let (state, _workspace) = setup_workspace_with_policy(deny_all_policy()).await;
    let snapshot = state.snapshot_dispatch_context().await.expect("snapshot");

    // WHEN the live config is flipped to allow-all AFTER the snapshot
    state
        .set_workspace_config(Some(WorkspaceConfig {
            policy: PolicyConfig {
                enabled: false,
                ..PolicyConfig::default()
            },
            ..WorkspaceConfig::default()
        }))
        .await;

    // THEN evaluating policy against the SNAPSHOT still denies (it is frozen)
    let eval = policy::evaluate(
        &snapshot.config.policy,
        Some("unknown-agent"),
        "list_symbols",
    );
    assert!(
        eval.is_err(),
        "snapshot policy must still deny after live config was changed"
    );

    // AND the live config now permits the same call
    let live = state
        .workspace_config()
        .await
        .expect("live config must exist");
    let live_eval = policy::evaluate(&live.policy, Some("unknown-agent"), "list_symbols");
    assert!(
        live_eval.is_ok(),
        "live config must now allow (policy disabled)"
    );
}

/// C018-05: Under concurrent load, the dispatch snapshot is immune to
/// a racing `set_workspace_config` call.
///
/// This test takes an atomic snapshot, spawns a concurrent config flip,
/// then evaluates policy against the snapshot — proving the snapshot
/// is a frozen point-in-time copy regardless of concurrent mutations.
///
/// RED: panics with `unimplemented!` — `snapshot_dispatch_context` is a stub.
#[test]
async fn c018_05_concurrent_config_flip_does_not_bypass_policy() {
    // GIVEN a workspace with deny-all policy
    let (state, _workspace) = setup_workspace_with_policy(deny_all_policy()).await;

    // Take an atomic snapshot BEFORE any concurrent mutation
    let snapshot = state.snapshot_dispatch_context().await.expect("snapshot");

    // Spawn a concurrent task that flips config to disabled (allow-all)
    let flipper_state = state.clone();
    let flipper = tokio::spawn(async move {
        flipper_state
            .set_workspace_config(Some(WorkspaceConfig {
                policy: PolicyConfig {
                    enabled: false,
                    ..PolicyConfig::default()
                },
                ..WorkspaceConfig::default()
            }))
            .await;
    });
    flipper.await.expect("flipper must complete");

    // WHEN policy is evaluated against the SNAPSHOT (not the live config)
    let eval = policy::evaluate(&snapshot.config.policy, Some("anonymous"), "list_symbols");

    // THEN it is still denied — the snapshot captured deny-all before the flip
    assert!(
        eval.is_err(),
        "snapshot policy must deny even after live config was flipped to allow-all"
    );

    // AND the live config is now disabled (allow-all), proving the flip happened
    let live = state.workspace_config().await.expect("live config");
    assert!(
        !live.policy.enabled,
        "live config must reflect the concurrent flip to disabled"
    );
}

// ── AC3: Policy-denied calls recorded in metrics with outcome=denied ─────────

/// C018-06: A policy-denied dispatch records a `UsageEvent` with
/// `outcome == "denied"` in the metrics subsystem.
///
/// RED: The current dispatch returns early on `PolicyDenied` (line ~129)
/// before reaching the metrics recording block (line ~192), so no event
/// is recorded for denied calls.
///
/// `#[serial]` is required because `metrics::clear_recent_events()` resets a
/// process-global ledger.  Without isolation, a concurrent test's denied event
/// can race with the `clear_recent_events` / `recent_events` assertion window,
/// producing a non-deterministic result.  `c018_07` does not call
/// `clear_recent_events()` and uses a unique three-field predicate
/// (`tool_name + outcome + agent_role`) that self-isolates without serialisation.
#[test]
#[serial]
async fn c018_06_policy_denied_call_records_metrics_with_denied_outcome() {
    // GIVEN a workspace with deny-all policy and clear metrics ledger
    let (state, _workspace) = setup_workspace_with_policy(deny_all_policy()).await;
    metrics::clear_recent_events();

    // WHEN a tool call is denied by policy
    let result = tools::dispatch(
        state.clone(),
        "list_symbols",
        Some(json!({ "file_path": "src/lib.rs" })),
    )
    .await;

    // Confirm the call was indeed denied
    assert!(result.is_err(), "call must be denied");
    assert_eq!(
        result.unwrap_err().to_response().error.code,
        POLICY_DENIED,
        "must be PolicyDenied"
    );

    // THEN a UsageEvent with outcome="denied" is recorded
    let events = metrics::recent_events();
    let denied_event = events
        .iter()
        .find(|e| e.tool_name == "list_symbols" && e.outcome == "denied");
    assert!(
        denied_event.is_some(),
        "a denied tool call must produce a UsageEvent with outcome=\"denied\"; \
         got {} event(s): {events:?}",
        events.len()
    );
}

/// C018-07: A policy-denied dispatch records the correct `agent_role`
/// in the metrics event even though no response payload was generated.
///
/// RED: Same root cause as C018-06 — denied calls skip the metrics block.
///
/// No `#[serial]` or `clear_recent_events()` is needed here: the assertion
/// filters on all three fields (`tool_name + outcome + agent_role`), making it
/// immune to events inserted by concurrent tests.
#[test]
async fn c018_07_denied_metrics_event_carries_agent_role() {
    // GIVEN a workspace with deny-all policy
    let (state, _workspace) = setup_workspace_with_policy(deny_all_policy()).await;

    // WHEN a tool call from "rogue-agent" is denied
    let result = tools::dispatch(
        state.clone(),
        "list_symbols",
        Some(json!({
            "file_path": "src/lib.rs",
            "_meta": { "agent_role": "rogue-agent" }
        })),
    )
    .await;

    assert!(result.is_err(), "call must be denied");

    // THEN the recorded UsageEvent carries agent_role = "rogue-agent".
    // Filter by all three criteria to avoid matching a concurrent test's denied
    // event that shares the same tool_name but has no agent_role.
    let events = metrics::recent_events();
    let denied_event = events.iter().find(|e| {
        e.tool_name == "list_symbols"
            && e.outcome == "denied"
            && e.agent_role.as_deref() == Some("rogue-agent")
    });
    assert!(
        denied_event.is_some(),
        "denied metrics event must carry agent_role=\"rogue-agent\"; \
         got {} event(s): {events:?}",
        events.len()
    );
}
