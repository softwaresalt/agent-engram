//! Content record model for ingested workspace content.
//!
//! Provides [`ContentRecord`] — an ingested piece of content stored
//! in SurrealDB, partitioned by content type for type-filtered search.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An ingested piece of content from a registered workspace source.
///
/// Each record represents a single file's content, partitioned by
/// the content type declared in the registry. Records are keyed by
/// `file_path` within a workspace database (unique constraint).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentRecord {
    /// SurrealDB record identifier (stringified `Thing`).
    pub id: String,

    /// Content type from the source registry entry (e.g. `"spec"`, `"code"`).
    pub content_type: String,

    /// Relative file path from workspace root.
    pub file_path: String,

    /// SHA-256 hash of file content for change detection.
    pub content_hash: String,

    /// Full text content of the file.
    pub content: String,

    /// Vector embedding (when the `embeddings` feature is enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,

    /// Registry source path this record belongs to.
    pub source_path: String,

    /// File size at ingestion time in bytes.
    pub file_size_bytes: u64,

    /// Timestamp of last ingestion.
    pub ingested_at: DateTime<Utc>,
}
