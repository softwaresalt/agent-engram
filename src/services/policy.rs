//! Policy evaluation service for the MCP sandbox engine (TASK-016.02).
//!
//! Provides [`evaluate`] to check whether an agent role is permitted to
//! invoke a given MCP tool, and [`extract_agent_role`] to pull identity
//! from JSON-RPC `_meta` params.

use serde_json::Value;

use crate::errors::PolicyError;
use crate::models::policy::{PolicyConfig, UnmatchedPolicy};

/// Context carried through the dispatch pipeline per tool call.
#[derive(Debug, Default, Clone)]
pub struct ToolCallContext {
    /// Agent role extracted from `_meta.agent_role`, if present.
    pub agent_role: Option<String>,
}

/// Extract agent role from the `_meta.agent_role` field in JSON-RPC params.
#[must_use]
pub fn extract_agent_role(params: &Option<Value>) -> Option<String> {
    let params = params.as_ref()?;
    params
        .get("_meta")
        .and_then(|m| m.get("agent_role"))
        .and_then(Value::as_str)
        .map(String::from)
}

/// Evaluate whether an agent role is permitted to call a tool.
///
/// Returns `Ok(())` if allowed, `Err(PolicyError::Denied)` if blocked.
///
/// # Errors
///
/// Returns [`PolicyError::Denied`] when the policy explicitly forbids the
/// agent role from calling the specified tool.
pub fn evaluate(
    config: &PolicyConfig,
    agent_role: Option<&str>,
    tool_name: &str,
) -> Result<(), PolicyError> {
    if !config.enabled {
        return Ok(());
    }

    let rule = agent_role.and_then(|role| config.rules.iter().find(|r| r.agent_role == role));

    let Some(rule) = rule else {
        return match config.unmatched {
            UnmatchedPolicy::Allow => Ok(()),
            UnmatchedPolicy::Deny => Err(PolicyError::Denied {
                agent_role: agent_role.unwrap_or("<anonymous>").to_string(),
                tool_name: tool_name.to_string(),
            }),
        };
    };

    // Deny list takes precedence.
    if rule.deny.iter().any(|t| t == tool_name) {
        return Err(PolicyError::Denied {
            agent_role: agent_role.unwrap_or("<anonymous>").to_string(),
            tool_name: tool_name.to_string(),
        });
    }

    // Non-empty allow list acts as allowlist; not in list → denied.
    if !rule.allow.is_empty() && !rule.allow.iter().any(|t| t == tool_name) {
        return Err(PolicyError::Denied {
            agent_role: agent_role.unwrap_or("<anonymous>").to_string(),
            tool_name: tool_name.to_string(),
        });
    }

    Ok(())
}
