//! File hash model — a stored content hash for a tracked workspace file.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A stored file hash record from the `file_hash` table.
///
/// Maps a workspace-relative file path to its last-known SHA-256 content hash
/// and size, enabling offline change detection on daemon restart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileHashRecord {
    /// Workspace-relative file path (e.g., `src/main.rs`).
    pub file_path: String,
    /// SHA-256 hex digest of the file contents at record time.
    pub content_hash: String,
    /// File size in bytes at record time.
    pub size_bytes: u64,
    /// Timestamp when the hash was last recorded.
    pub recorded_at: DateTime<Utc>,
}
