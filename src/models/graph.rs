//! Graph relationship types for task dependency edges.

use serde::{Deserialize, Serialize};

/// Type of dependency between two tasks in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    /// Task cannot progress until the blocker is done.
    HardBlocker,
    /// Blocker provides useful context but does not prevent progress.
    SoftDependency,
}
