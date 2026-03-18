//! Git commit graph models for change tracking.
//!
//! Provides [`CommitNode`], [`ChangeRecord`], and [`ChangeType`] —
//! the graph representations of git commits and their per-file
//! changes stored in SurrealDB for history-aware querying.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of file change within a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// File was added in this commit.
    Add,
    /// File was modified in this commit.
    Modify,
    /// File was deleted in this commit.
    Delete,
    /// File was renamed in this commit.
    Rename,
}

impl ChangeType {
    /// Return the canonical snake_case string for this change type.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Modify => "modify",
            Self::Delete => "delete",
            Self::Rename => "rename",
        }
    }
}

/// A per-file diff within a commit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeRecord {
    /// Relative file path affected.
    pub file_path: String,

    /// Type of change.
    pub change_type: ChangeType,

    /// Diff text with context lines.
    pub diff_snippet: String,

    /// Starting line in old file (for modifications).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_line_start: Option<u32>,

    /// Starting line in new file (for additions/modifications).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_line_start: Option<u32>,

    /// Count of added lines.
    pub lines_added: u32,

    /// Count of removed lines.
    pub lines_removed: u32,
}

/// A git commit in the graph, stored in SurrealDB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitNode {
    /// SurrealDB record identifier (stringified `Thing`).
    pub id: String,

    /// Full 40-character git commit hash.
    pub hash: String,

    /// 7-character abbreviated hash.
    pub short_hash: String,

    /// Commit author name.
    pub author_name: String,

    /// Commit author email.
    pub author_email: String,

    /// Commit timestamp (author date).
    pub timestamp: DateTime<Utc>,

    /// Full commit message.
    pub message: String,

    /// Parent commit hashes (empty for root, 2+ for merges).
    #[serde(default)]
    pub parent_hashes: Vec<String>,

    /// Per-file changes in this commit.
    #[serde(default)]
    pub changes: Vec<ChangeRecord>,
}
