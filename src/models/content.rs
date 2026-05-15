//! Content record model for ingested workspace content.
//!
//! Provides [`ContentRecord`] — an ingested piece of content stored
//! in SurrealDB, partitioned by content type for type-filtered search.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An ingested piece of content from a registered workspace source.
///
/// Each record represents a single retrieval unit from a workspace source.
/// Most sources emit one file-level record, while structured Markdown may emit
/// multiple chunk records for a single file.
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

    /// Record granularity, such as `"file"` or `"markdown_chunk"`.
    pub record_kind: String,

    /// Stable chunk identifier for structured Markdown retrieval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_id: Option<String>,

    /// One-based chunk ordinal within the file when chunked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<u32>,

    /// Heading provenance for Markdown chunks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heading_path: Vec<String>,

    /// One-based starting line for the retrieval unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<u32>,

    /// One-based ending line for the retrieval unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<u32>,

    /// Explicit reason for file-level fallback when structured chunking is not used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,

    /// Advisory lint summary surfaced to retrieval callers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lint_summary: Option<String>,

    /// Advisory lint suggestions surfaced to retrieval callers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
}
