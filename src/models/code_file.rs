//! Code file model — a source file tracked in the code graph.

use serde::{Deserialize, Serialize};

/// A source file tracked in the code graph.
///
/// Serves as the containment root for function/class/interface nodes.
/// The ID is derived from the SHA-256 hash of the workspace-relative path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeFile {
    /// SurrealDB record ID (format: `code_file:<hex>`).
    pub id: String,
    /// Workspace-relative file path (e.g., `src/billing.rs`).
    pub path: String,
    /// Language identifier (e.g., `"rust"`).
    pub language: String,
    /// File size at last index.
    pub size_bytes: u64,
    /// SHA-256 hex digest of file contents.
    pub content_hash: String,
    /// Timestamp of last successful index.
    pub last_indexed_at: String,
}
