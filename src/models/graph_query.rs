//! Graph query types for the structured `query_graph` MCP tool.

use serde::{Deserialize, Serialize};

/// Direction of edge traversal in graph queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraversalDirection {
    /// Follow edges where the current node is the source (forward traversal).
    Outgoing,
    /// Follow edges where the current node is the target (reverse traversal).
    Incoming,
    /// Follow edges in both directions (bidirectional traversal).
    #[default]
    Both,
}
