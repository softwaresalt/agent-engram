//! Policy evaluation service for the MCP sandbox engine (TASK-016.02).
//!
//! Provides [`evaluate`] to check whether an agent role is permitted to
//! invoke a given MCP tool, and [`extract_agent_role`] to pull identity
//! from JSON-RPC `_meta` params.

use serde_json::Value;

use crate::errors::PolicyError;
use crate::models::policy::PolicyConfig;

/// Context carried through the dispatch pipeline per tool call.
#[derive(Debug, Default, Clone)]
pub struct ToolCallContext {
    /// Agent role extracted from `_meta.agent_role`, if present.
    pub agent_role: Option<String>,
}

/// Extract agent role from the `_meta.agent_role` field in JSON-RPC params.
#[must_use]
pub fn extract_agent_role(_params: &Option<Value>) -> Option<String> {
    unimplemented!(
        "Worker: extract params._meta.agent_role as Option<String> from the JSON-RPC params value. Return None if _meta or agent_role is absent."
    )
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
    _config: &PolicyConfig,
    _agent_role: Option<&str>,
    _tool_name: &str,
) -> Result<(), PolicyError> {
    unimplemented!(
        "Worker: implement policy evaluation logic. \
         1) If config.enabled is false, return Ok(()). \
         2) If agent_role is None, apply config.unmatched policy. \
         3) Find matching PolicyRule by exact agent_role string. \
         4) If no match, apply config.unmatched. \
         5) If rule.deny contains tool_name, return Err(Denied). \
         6) If rule.allow is non-empty and does not contain tool_name, return Err(Denied). \
         7) Otherwise Ok(())."
    )
}
