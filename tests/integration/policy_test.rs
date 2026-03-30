//! BDD harness for MCP sandbox policy engine (TASK-016).
//!
//! Tests cover: policy model serde, policy evaluation logic,
//! agent-role extraction from JSON-RPC params, error responses,
//! and dispatch integration with policy enforcement.
//!
//! Run: `cargo test --test policy_test`

use engram::errors::codes::{POLICY_CONFIG_INVALID, POLICY_DENIED};
use engram::errors::{EngramError, PolicyError};
use engram::models::policy::{PolicyConfig, PolicyRule, UnmatchedPolicy};
use engram::services::policy::{ToolCallContext, evaluate, extract_agent_role};
use serde_json::json;

// ── Section 1: Model serde round-trips (TASK-016.01.02) ─────────────

/// GIVEN a fully populated [`PolicyConfig`]
/// WHEN serialized to JSON and deserialized back
/// THEN the round-tripped value equals the original.
#[test]
fn t016_01_02_policy_config_serde_round_trip() {
    let config = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Deny,
        rules: vec![
            PolicyRule {
                agent_role: "doc-ops".to_string(),
                allow: vec!["list_symbols".to_string(), "unified_search".to_string()],
                deny: vec![],
            },
            PolicyRule {
                agent_role: "rust-engineer".to_string(),
                allow: vec![],
                deny: vec!["set_workspace".to_string()],
            },
        ],
    };

    let json_str =
        serde_json::to_string(&config).unwrap_or_else(|e| panic!("serialize failed: {e}"));
    let round_tripped: PolicyConfig =
        serde_json::from_str(&json_str).unwrap_or_else(|e| panic!("deserialize failed: {e}"));

    assert_eq!(config, round_tripped);
}

/// GIVEN a default [`PolicyConfig`]
/// WHEN checked
/// THEN `enabled` is false and `unmatched` is `Allow`.
#[test]
fn t016_01_02_policy_config_defaults() {
    let config = PolicyConfig::default();

    assert!(!config.enabled, "default policy should be disabled");
    assert_eq!(
        config.unmatched,
        UnmatchedPolicy::Allow,
        "default unmatched policy should be Allow"
    );
    assert!(config.rules.is_empty(), "default rules should be empty");
}

/// GIVEN a [`PolicyRule`] with only `allow` list
/// WHEN serialized
/// THEN `deny` field defaults to empty vec.
#[test]
fn t016_01_02_policy_rule_optional_deny_defaults() {
    let json_str = r#"{"agent_role":"tester","allow":["map_code"]}"#;
    let rule: PolicyRule =
        serde_json::from_str(json_str).unwrap_or_else(|e| panic!("deserialize failed: {e}"));

    assert_eq!(rule.agent_role, "tester");
    assert_eq!(rule.allow, vec!["map_code"]);
    assert!(rule.deny.is_empty(), "deny should default to empty");
}

/// GIVEN an [`UnmatchedPolicy::Deny`]
/// WHEN serialized to JSON
/// THEN the string is `"deny"`.
#[test]
fn t016_01_02_unmatched_policy_serde_rename() {
    let json_str = serde_json::to_string(&UnmatchedPolicy::Deny)
        .unwrap_or_else(|e| panic!("serialize failed: {e}"));
    assert_eq!(json_str, r#""deny""#);

    let deserialized: UnmatchedPolicy =
        serde_json::from_str(r#""allow""#).unwrap_or_else(|e| panic!("deserialize failed: {e}"));
    assert_eq!(deserialized, UnmatchedPolicy::Allow);
}

// ── Section 2: PolicyError and error codes (TASK-016.01.03) ─────────

/// GIVEN a [`PolicyError::Denied`]
/// WHEN converted to [`EngramError`] and then `to_response`
/// THEN the error code is 14001 and details contain `agent_role` and `tool_name`.
#[test]
fn t016_01_03_policy_denied_error_response() {
    let err = EngramError::from(PolicyError::Denied {
        agent_role: "doc-ops".to_string(),
        tool_name: "set_workspace".to_string(),
    });
    let resp = err.to_response();

    assert_eq!(resp.error.code, POLICY_DENIED);
    assert_eq!(resp.error.name, "PolicyDenied");

    let details = resp
        .error
        .details
        .as_ref()
        .unwrap_or_else(|| panic!("expected details"));
    assert_eq!(details["agent_role"], "doc-ops");
    assert_eq!(details["tool_name"], "set_workspace");
}

/// GIVEN a [`PolicyError::ConfigInvalid`]
/// WHEN converted to [`EngramError`] and then `to_response`
/// THEN the error code is 14002.
#[test]
fn t016_01_03_policy_config_invalid_error_response() {
    let err = EngramError::from(PolicyError::ConfigInvalid {
        reason: "duplicate agent_role".to_string(),
    });
    let resp = err.to_response();

    assert_eq!(resp.error.code, POLICY_CONFIG_INVALID);
    assert_eq!(resp.error.name, "PolicyConfigInvalid");
}

// ── Section 3: extract_agent_role (TASK-016.02.02) ──────────────────

/// GIVEN JSON-RPC params with `_meta.agent_role`
/// WHEN `extract_agent_role` is called
/// THEN it returns `Some(role)`.
#[test]
fn t016_02_02_extract_agent_role_from_meta() {
    let params = Some(json!({
        "query": "test",
        "_meta": { "agent_role": "doc-ops" }
    }));

    let role = extract_agent_role(&params);
    assert_eq!(role, Some("doc-ops".to_string()));
}

/// GIVEN JSON-RPC params without `_meta`
/// WHEN `extract_agent_role` is called
/// THEN it returns `None`.
#[test]
fn t016_02_02_extract_agent_role_missing_meta() {
    let params = Some(json!({ "query": "test" }));

    let role = extract_agent_role(&params);
    assert_eq!(role, None);
}

/// GIVEN `None` params
/// WHEN `extract_agent_role` is called
/// THEN it returns `None`.
#[test]
fn t016_02_02_extract_agent_role_none_params() {
    let role = extract_agent_role(&None);
    assert_eq!(role, None);
}

/// GIVEN `_meta` present but `agent_role` absent
/// WHEN `extract_agent_role` is called
/// THEN it returns `None`.
#[test]
fn t016_02_02_extract_agent_role_meta_without_role() {
    let params = Some(json!({
        "_meta": { "session_id": "abc123" }
    }));

    let role = extract_agent_role(&params);
    assert_eq!(role, None);
}

// ── Section 4: ToolCallContext (TASK-016.02.02) ─────────────────────

/// GIVEN a [`ToolCallContext`]
/// WHEN constructed with default
/// THEN `agent_role` is `None`.
#[test]
fn t016_02_02_tool_call_context_default() {
    let ctx = ToolCallContext::default();
    assert_eq!(ctx.agent_role, None);
}

// ── Section 5: Policy evaluation logic (TASK-016.02.01) ─────────────

/// GIVEN policy is disabled
/// WHEN any agent calls any tool
/// THEN evaluate returns Ok.
#[test]
fn t016_02_01_disabled_policy_allows_everything() {
    let config = PolicyConfig {
        enabled: false,
        ..PolicyConfig::default()
    };

    let result = evaluate(&config, Some("doc-ops"), "set_workspace");
    assert!(result.is_ok(), "disabled policy should allow everything");
}

/// GIVEN policy is enabled with unmatched=Allow
/// WHEN an unknown agent calls a tool
/// THEN evaluate returns Ok.
#[test]
fn t016_02_01_unmatched_allow_permits_unknown_agent() {
    let config = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Allow,
        rules: vec![],
    };

    let result = evaluate(&config, Some("unknown-agent"), "list_symbols");
    assert!(
        result.is_ok(),
        "unmatched=Allow should permit unknown agents"
    );
}

/// GIVEN policy is enabled with unmatched=Deny
/// WHEN an unknown agent calls a tool
/// THEN evaluate returns Err(PolicyDenied).
#[test]
fn t016_02_01_unmatched_deny_blocks_unknown_agent() {
    let config = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Deny,
        rules: vec![],
    };

    let result = evaluate(&config, Some("unknown-agent"), "list_symbols");
    assert!(
        result.is_err(),
        "unmatched=Deny should block unknown agents"
    );
}

/// GIVEN an agent with an allow-list
/// WHEN the agent calls an allowed tool
/// THEN evaluate returns Ok.
#[test]
fn t016_02_01_allow_list_permits_listed_tool() {
    let config = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Deny,
        rules: vec![PolicyRule {
            agent_role: "doc-ops".to_string(),
            allow: vec!["list_symbols".to_string(), "unified_search".to_string()],
            deny: vec![],
        }],
    };

    let result = evaluate(&config, Some("doc-ops"), "list_symbols");
    assert!(result.is_ok(), "allowed tool should be permitted");
}

/// GIVEN an agent with an allow-list
/// WHEN the agent calls an unlisted tool
/// THEN evaluate returns Err(PolicyDenied).
#[test]
fn t016_02_01_allow_list_blocks_unlisted_tool() {
    let config = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Deny,
        rules: vec![PolicyRule {
            agent_role: "doc-ops".to_string(),
            allow: vec!["list_symbols".to_string()],
            deny: vec![],
        }],
    };

    let result = evaluate(&config, Some("doc-ops"), "set_workspace");
    assert!(
        result.is_err(),
        "unlisted tool should be denied when allow-list is set"
    );
}

/// GIVEN an agent with a deny-list
/// WHEN the agent calls a denied tool
/// THEN evaluate returns Err(PolicyDenied).
#[test]
fn t016_02_01_deny_list_blocks_denied_tool() {
    let config = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Allow,
        rules: vec![PolicyRule {
            agent_role: "rust-engineer".to_string(),
            allow: vec![],
            deny: vec!["set_workspace".to_string()],
        }],
    };

    let result = evaluate(&config, Some("rust-engineer"), "set_workspace");
    assert!(result.is_err(), "denied tool should be blocked");
}

/// GIVEN an agent with a deny-list
/// WHEN the agent calls a non-denied tool
/// THEN evaluate returns Ok.
#[test]
fn t016_02_01_deny_list_permits_unlisted_tool() {
    let config = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Allow,
        rules: vec![PolicyRule {
            agent_role: "rust-engineer".to_string(),
            allow: vec![],
            deny: vec!["set_workspace".to_string()],
        }],
    };

    let result = evaluate(&config, Some("rust-engineer"), "list_symbols");
    assert!(
        result.is_ok(),
        "non-denied tool should be permitted with deny-list"
    );
}

/// GIVEN an agent with both allow and deny lists
/// WHEN the agent calls a tool in both lists
/// THEN deny takes precedence and evaluate returns Err.
#[test]
fn t016_02_01_deny_takes_precedence_over_allow() {
    let config = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Allow,
        rules: vec![PolicyRule {
            agent_role: "tester".to_string(),
            allow: vec!["map_code".to_string()],
            deny: vec!["map_code".to_string()],
        }],
    };

    let result = evaluate(&config, Some("tester"), "map_code");
    assert!(
        result.is_err(),
        "deny should take precedence over allow for same tool"
    );
}

/// GIVEN policy is enabled
/// WHEN `agent_role` is `None` and `unmatched=Allow`
/// THEN evaluate returns Ok.
#[test]
fn t016_02_01_none_agent_role_with_allow_unmatched() {
    let config = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Allow,
        rules: vec![],
    };

    let result = evaluate(&config, None, "list_symbols");
    assert!(
        result.is_ok(),
        "None agent_role with unmatched=Allow should pass"
    );
}

/// GIVEN policy is enabled
/// WHEN `agent_role` is `None` and `unmatched=Deny`
/// THEN evaluate returns Err.
#[test]
fn t016_02_01_none_agent_role_with_deny_unmatched() {
    let config = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Deny,
        rules: vec![],
    };

    let result = evaluate(&config, None, "list_symbols");
    assert!(
        result.is_err(),
        "None agent_role with unmatched=Deny should block"
    );
}

/// GIVEN an agent with an empty allow-list and empty deny-list
/// WHEN the agent calls any tool
/// THEN evaluate returns Ok (empty allow = allow-all for that role).
#[test]
fn t016_02_01_empty_allow_list_means_allow_all() {
    let config = PolicyConfig {
        enabled: true,
        unmatched: UnmatchedPolicy::Deny,
        rules: vec![PolicyRule {
            agent_role: "admin".to_string(),
            allow: vec![],
            deny: vec![],
        }],
    };

    let result = evaluate(&config, Some("admin"), "any_tool");
    assert!(
        result.is_ok(),
        "empty allow+deny lists should mean allow-all for that role"
    );
}
