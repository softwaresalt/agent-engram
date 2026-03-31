//! Unit tests for WorkspaceConfig policy integration (TASK-016.01.01).
//!
//! Covers:
//! - WorkspaceConfig deserializes with a `[policy]` section
//! - WorkspaceConfig deserializes without a `[policy]` section (defaults to disabled)
//! - `AppState::policy_config()` returns `None` when no workspace config is loaded
//! - `AppState::policy_config()` returns the cached `PolicyConfig` when workspace config is set
//! - `parse_config` falls back gracefully when the `[policy]` section has invalid values

use std::sync::Arc;

use engram::models::config::WorkspaceConfig;
use engram::models::policy::{PolicyConfig, PolicyRule, UnmatchedPolicy};
use engram::server::state::AppState;
use engram::services::config::parse_config;

// ── WorkspaceConfig TOML deserialization ─────────────────────────────────────

/// GIVEN a `.engram/config.toml` with a fully populated `[policy]` section
/// WHEN deserialized into `WorkspaceConfig`
/// THEN `policy.enabled`, `policy.unmatched`, and `policy.rules` match the file.
#[test]
fn workspace_config_policy_section_deserializes() {
    let toml_str = r#"
[policy]
enabled = true
unmatched = "deny"

[[policy.rules]]
agent_role = "doc-ops"
allow = ["query_memory", "unified_search", "list_symbols", "map_code"]
deny = ["index_workspace", "sync_workspace", "flush_state"]
"#;

    let config: WorkspaceConfig =
        toml::from_str(toml_str).unwrap_or_else(|e| panic!("toml parse failed: {e}"));

    assert!(config.policy.enabled, "policy should be enabled");
    assert_eq!(
        config.policy.unmatched,
        UnmatchedPolicy::Deny,
        "unmatched should be Deny"
    );
    assert_eq!(config.policy.rules.len(), 1, "one rule expected");

    let rule = &config.policy.rules[0];
    assert_eq!(rule.agent_role, "doc-ops");
    assert_eq!(
        rule.allow,
        vec!["query_memory", "unified_search", "list_symbols", "map_code"]
    );
    assert_eq!(
        rule.deny,
        vec!["index_workspace", "sync_workspace", "flush_state"]
    );
}

/// GIVEN a `.engram/config.toml` with no `[policy]` section
/// WHEN deserialized into `WorkspaceConfig`
/// THEN `policy` defaults to disabled with `unmatched = Deny` and no rules.
#[test]
fn workspace_config_without_policy_defaults_to_disabled() {
    let config: WorkspaceConfig =
        toml::from_str("").unwrap_or_else(|e| panic!("toml parse failed: {e}"));

    assert!(!config.policy.enabled, "policy should be disabled by default");
    assert_eq!(
        config.policy.unmatched,
        UnmatchedPolicy::Deny,
        "unmatched should default to Deny"
    );
    assert!(
        config.policy.rules.is_empty(),
        "rules should default to empty"
    );
}

/// GIVEN a config.toml with multiple policy rules
/// WHEN deserialized
/// THEN all rules are present and preserve order.
#[test]
fn workspace_config_policy_multiple_rules() {
    let toml_str = r#"
[policy]
enabled = true
unmatched = "allow"

[[policy.rules]]
agent_role = "doc-ops"
allow = ["list_symbols"]
deny = []

[[policy.rules]]
agent_role = "rust-engineer"
allow = []
deny = ["set_workspace"]
"#;

    let config: WorkspaceConfig =
        toml::from_str(toml_str).unwrap_or_else(|e| panic!("toml parse failed: {e}"));

    assert_eq!(config.policy.rules.len(), 2);
    assert_eq!(config.policy.rules[0].agent_role, "doc-ops");
    assert_eq!(config.policy.rules[1].agent_role, "rust-engineer");
    assert_eq!(config.policy.rules[1].deny, vec!["set_workspace"]);
}

// ── parse_config fallback (review F6) ────────────────────────────────────────

/// GIVEN a `config.toml` with a `[policy]` section containing an invalid type
/// WHEN `parse_config` is called
/// THEN it logs a warning and returns `WorkspaceConfig::default()` (policy disabled),
///      not an error that would block workspace binding.
#[test]
fn parse_config_falls_back_on_invalid_policy_type() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let engram_dir = tmp.path().join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");
    // `enabled` expects a bool; this should cause a toml parse error → fallback.
    std::fs::write(
        engram_dir.join("config.toml"),
        "[policy]\nenabled = \"not-a-bool\"\n",
    )
    .expect("write config.toml");

    let result = parse_config(tmp.path());
    let config = result.unwrap_or_else(|e| panic!("parse_config returned Err: {e:?}"));

    assert!(
        !config.policy.enabled,
        "invalid policy section must fall back to disabled"
    );
}

/// GIVEN a `config.toml` with a missing `[policy]` section (file present, no policy key)
/// WHEN `parse_config` is called
/// THEN policy is disabled and workspace binding succeeds.
#[test]
fn parse_config_no_policy_section_succeeds() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let engram_dir = tmp.path().join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");
    std::fs::write(engram_dir.join("config.toml"), "").expect("write config.toml");

    let config = parse_config(tmp.path()).unwrap_or_else(|e| panic!("parse_config failed: {e:?}"));

    assert!(!config.policy.enabled, "absent [policy] should be disabled");
    assert!(config.policy.rules.is_empty());
}

// ── AppState::policy_config() ─────────────────────────────────────────────────

/// GIVEN no workspace config has been loaded
/// WHEN `AppState::policy_config()` is called
/// THEN it returns `None`.
#[tokio::test]
async fn appstate_policy_config_returns_none_when_no_config() {
    let state = Arc::new(AppState::new(10));

    let result = state.policy_config().await;

    assert!(
        result.is_none(),
        "policy_config() must return None when no workspace config is loaded"
    );
}

/// GIVEN a `WorkspaceConfig` with a custom `PolicyConfig` has been set
/// WHEN `AppState::policy_config()` is called
/// THEN it returns `Some(policy)` matching the loaded config.
#[tokio::test]
async fn appstate_policy_config_returns_cached_config() {
    let state = Arc::new(AppState::new(10));

    let policy = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Deny,
        rules: vec![PolicyRule {
            agent_role: "doc-ops".to_string(),
            allow: vec!["list_symbols".to_string()],
            deny: vec![],
        }],
    };

    let config = WorkspaceConfig {
        policy: policy.clone(),
        ..WorkspaceConfig::default()
    };
    state.set_workspace_config(Some(config)).await;

    let result = state.policy_config().await;

    assert!(
        result.is_some(),
        "policy_config() must return Some when workspace config is loaded"
    );
    assert_eq!(
        result.unwrap(),
        policy,
        "returned policy must match the stored config"
    );
}

/// GIVEN a workspace config was set and then cleared
/// WHEN `AppState::policy_config()` is called after clearing
/// THEN it returns `None`.
#[tokio::test]
async fn appstate_policy_config_returns_none_after_config_cleared() {
    let state = Arc::new(AppState::new(10));

    state
        .set_workspace_config(Some(WorkspaceConfig::default()))
        .await;
    assert!(state.policy_config().await.is_some(), "should have config");

    state.set_workspace_config(None).await;

    assert!(
        state.policy_config().await.is_none(),
        "policy_config() must return None after config is cleared"
    );
}

/// GIVEN a `WorkspaceConfig` with the default (disabled) policy
/// WHEN `AppState::policy_config()` is called
/// THEN it returns `Some(PolicyConfig { enabled: false, ... })`.
#[tokio::test]
async fn appstate_policy_config_returns_default_disabled_policy() {
    let state = Arc::new(AppState::new(10));

    state
        .set_workspace_config(Some(WorkspaceConfig::default()))
        .await;

    let result = state.policy_config().await.expect("should be Some");

    assert!(
        !result.enabled,
        "default WorkspaceConfig must carry a disabled PolicyConfig"
    );
    assert_eq!(result.unmatched, UnmatchedPolicy::Deny);
    assert!(result.rules.is_empty());
}
