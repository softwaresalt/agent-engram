//! Policy data models for the MCP sandbox policy engine (TASK-016.01).
//!
//! Provides [`PolicyRule`], [`PolicyConfig`], and [`UnmatchedPolicy`]
//! for per-agent tool access control.

use serde::{Deserialize, Serialize};

/// Behavior when no policy rule matches the requesting agent's role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnmatchedPolicy {
    /// Allow tool calls from unrecognized agent roles (default).
    Allow,
    /// Deny tool calls from unrecognized agent roles.
    Deny,
}

impl Default for UnmatchedPolicy {
    fn default() -> Self {
        Self::Allow
    }
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// When false, policy enforcement is disabled (allow-all).
    #[serde(default)]
    pub enabled: bool,
    /// Default behavior when no rule matches the agent role.
    #[serde(default)]
    pub unmatched: UnmatchedPolicy,
    /// Per-agent-role rules.
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            unmatched: UnmatchedPolicy::default(),
            rules: Vec::new(),
        }
    }
}
