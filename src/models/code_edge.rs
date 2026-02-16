//! Code edge types for the code knowledge graph.

use serde::{Deserialize, Serialize};

/// The type of relationship between code graph nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeEdgeType {
    /// Function-to-function call relationship.
    Calls,
    /// File-to-file import/use relationship.
    Imports,
    /// Class/struct to trait/interface inheritance.
    InheritsFrom,
    /// File to symbol containment.
    Defines,
    /// Cross-region task-to-code relationship.
    Concerns,
}

/// A directed edge in the code knowledge graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeEdge {
    /// Edge type.
    pub edge_type: CodeEdgeType,
    /// Source node ID (e.g., `function:abc123`).
    pub from: String,
    /// Target node ID (e.g., `function:def456`).
    pub to: String,
    /// Import path for `imports` edges (e.g., `crate::billing::process_payment`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_path: Option<String>,
    /// Identity of the client that created a `concerns` edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_by: Option<String>,
    /// When the edge was created.
    pub created_at: String,
}
