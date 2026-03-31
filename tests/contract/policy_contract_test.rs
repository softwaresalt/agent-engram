//! Contract tests for MCP sandbox policy gate (TASK-016.03).
//!
//! Validates that the policy gate is correctly wired into the MCP dispatch
//! pipeline: denied calls return error code 14001, allowed calls proceed,
//! and disabled policy permits everything.

use std::fs;
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::errors::codes::POLICY_DENIED;
use engram::models::config::WorkspaceConfig;
use engram::models::policy::{PolicyConfig, PolicyRule, UnmatchedPolicy};
use engram::server::state::AppState;
use engram::tools;

/// Helper: create a temp git workspace, bind it to state, and inject a policy config.
///
/// Returns both the `Arc<AppState>` and the `TempDir` handle. The caller MUST hold
/// the `TempDir` for the duration of the test — dropping it deletes the workspace.
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

    // Inject policy config into workspace config.
    let config = WorkspaceConfig {
        policy,
        ..WorkspaceConfig::default()
    };
    state.set_workspace_config(Some(config)).await;

    (state, workspace)
}

// ── C016-01: Policy disabled ─────────────────────────────────────────────────

/// C016-01: When policy is disabled (default), all tool calls proceed.
#[test]
async fn c016_01_disabled_policy_allows_all_tools() {
    let (state, _workspace) = setup_workspace_with_policy(PolicyConfig {
        enabled: false,
        ..PolicyConfig::default()
    })
    .await;

    // WHEN an agent with no role calls list_symbols
    let result = tools::dispatch(
        state.clone(),
        "list_symbols",
        Some(json!({ "file_path": "src/lib.rs" })),
    )
    .await;

    // THEN the call succeeds (or fails for non-policy reasons)
    assert!(
        !matches!(&result, Err(e) if e.to_response().error.code == POLICY_DENIED),
        "disabled policy must not produce PolicyDenied"
    );
}

// ── C016-02: Policy enabled, unmatched=Deny, no agent role ──────────────────

/// C016-02: When policy is enabled with `unmatched=Deny` and no `agent_role`
/// is present in params, the call is denied with error code 14001.
#[test]
async fn c016_02_unmatched_deny_blocks_anonymous_agent() {
    let (state, _workspace) = setup_workspace_with_policy(PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Deny,
        rules: vec![],
    })
    .await;

    // WHEN params have no _meta.agent_role
    let result = tools::dispatch(
        state.clone(),
        "list_symbols",
        Some(json!({ "file_path": "src/lib.rs" })),
    )
    .await;

    // THEN the call is denied
    assert!(result.is_err(), "anonymous agent should be denied");
    let code = result.unwrap_err().to_response().error.code;
    assert_eq!(code, POLICY_DENIED, "error code must be {POLICY_DENIED}");
}

// ── C016-03: Policy enabled, agent_role present and allowed ─────────────────

/// C016-03: When policy is enabled and the `agent_role` matches an allow rule,
/// the call proceeds (or fails for a non-policy reason).
#[test]
async fn c016_03_matching_allow_rule_permits_call() {
    let (state, _workspace) = setup_workspace_with_policy(PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Deny,
        rules: vec![PolicyRule {
            agent_role: "doc-ops".to_string(),
            allow: vec!["list_symbols".to_string(), "unified_search".to_string()],
            deny: vec![],
        }],
    })
    .await;

    // WHEN params include _meta.agent_role = "doc-ops" calling an allowed tool
    let result = tools::dispatch(
        state.clone(),
        "list_symbols",
        Some(json!({
            "file_path": "src/lib.rs",
            "_meta": { "agent_role": "doc-ops" }
        })),
    )
    .await;

    // THEN the call is not denied by policy
    assert!(
        !matches!(&result, Err(e) if e.to_response().error.code == POLICY_DENIED),
        "allowed tool should not produce PolicyDenied"
    );
}

// ── C016-04: Policy enabled, agent_role present but tool denied ──────────────

/// C016-04: When policy is enabled and the `agent_role` is in the deny list for
/// the requested tool, the call returns 14001 `PolicyDenied`.
#[test]
async fn c016_04_deny_list_blocks_tool_via_dispatch() {
    let (state, _workspace) = setup_workspace_with_policy(PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Allow,
        rules: vec![PolicyRule {
            agent_role: "restricted-agent".to_string(),
            allow: vec![],
            deny: vec!["set_workspace".to_string()],
        }],
    })
    .await;

    // WHEN restricted-agent tries to call set_workspace
    let result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({
            "path": "/tmp",
            "_meta": { "agent_role": "restricted-agent" }
        })),
    )
    .await;

    // THEN the call is denied with code 14001
    assert!(result.is_err(), "denied tool should return error");
    let code = result.unwrap_err().to_response().error.code;
    assert_eq!(code, POLICY_DENIED, "error code must be {POLICY_DENIED}");
}

// ── C016-05: Policy gate error code schema ───────────────────────────────────

/// C016-05: `PolicyDenied` response conforms to the error taxonomy shape:
/// `{ error: { code: 14001, name: "PolicyDenied", message: "...", details: { agent_role, tool_name } } }`.
#[test]
async fn c016_05_policy_denied_error_shape() {
    let (state, _workspace) = setup_workspace_with_policy(PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Deny,
        rules: vec![],
    })
    .await;

    let err = tools::dispatch(
        state.clone(),
        "list_symbols",
        Some(json!({ "file_path": "src/lib.rs" })),
    )
    .await
    .unwrap_err();

    let response = err.to_response();
    let error = &response.error;

    assert_eq!(error.code, POLICY_DENIED);
    assert_eq!(error.name, "PolicyDenied");
    assert!(!error.message.is_empty(), "message must be non-empty");

    let details = error.details.as_ref().expect("details must be present");
    assert!(
        details.get("agent_role").is_some(),
        "details must contain agent_role"
    );
    assert!(
        details.get("tool_name").is_some(),
        "details must contain tool_name"
    );
}
