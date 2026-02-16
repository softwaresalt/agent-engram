//! Graph relationship types for task dependency edges.

use serde::{Deserialize, Serialize};

/// Type of dependency between two tasks in the graph.
///
/// Eight variants covering blocking, hierarchy, and informational links.
/// `hard_blocker`, `blocked_by`, and `duplicate_of` affect ready-work
/// eligibility; the remaining types are informational.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    /// Task cannot progress until the blocker is done.
    HardBlocker,
    /// Blocker provides useful context but does not prevent progress.
    SoftDependency,
    /// Source is a subtask of target (parent-child hierarchy).
    ChildOf,
    /// Source is blocked by target (directional blocking).
    BlockedBy,
    /// Source is a duplicate of target; excluded from ready-work.
    DuplicateOf,
    /// Informational linkage, no blocking semantics.
    RelatedTo,
    /// Target should be done before source starts (ordering hint).
    Predecessor,
    /// Source should be done before target starts (inverse of predecessor).
    Successor,
}

impl DependencyType {
    /// All valid variant names as they appear in serialized form.
    pub const ALL: &[&str] = &[
        "hard_blocker",
        "soft_dependency",
        "child_of",
        "blocked_by",
        "duplicate_of",
        "related_to",
        "predecessor",
        "successor",
    ];
}
