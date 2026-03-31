//! Policy data models for the MCP sandbox policy engine (TASK-016.01).
//!
//! Provides [`PolicyRule`], [`PolicyConfig`], and [`UnmatchedPolicy`]
//! for per-agent tool access control.

use serde::{Deserialize, Serialize};

/// Behavior when no policy rule matches the requesting agent's role.
///
/// The default is [`Deny`][UnmatchedPolicy::Deny], which is the safe choice
/// when `enabled = true`.  Because [`PolicyConfig::enabled`] defaults to
/// `false`, operators who have not opted in to policy enforcement are
/// unaffected by this default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnmatchedPolicy {
    /// Allow tool calls from unrecognized agent roles.
    Allow,
    /// Deny tool calls from unrecognized agent roles (default).
    #[default]
    Deny,
}

/// A single policy rule mapping an agent role to tool permissions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Agent role identifier (e.g., `"doc-ops"`, `"rust-engineer"`).
    pub agent_role: String,
    /// Tools this role is explicitly allowed to call. Empty means allow-all.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Tools this role is explicitly denied from calling. Evaluated after allow.
    #[serde(default)]
    pub deny: Vec<String>,
}

/// Workspace-level policy configuration loaded from `.engram/engram.toml`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// When `false`, all tool calls are allowed regardless of rules (backward-compatible default).
    /// Set to `true` to enforce role-based allow/deny rules from `.engram/engram.toml`.
    #[serde(default)]
    pub enabled: bool,
    /// Default behavior when no rule matches the agent role.
    #[serde(default)]
    pub unmatched: UnmatchedPolicy,
    /// Per-agent-role rules.
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
}
