//! Content registry models for workspace content source declaration.
//!
//! Provides [`RegistryConfig`] (top-level registry parsed from
//! `.engram/registry.yaml`) and [`ContentSource`] (a single declared
//! content source with type, language, and path).

use serde::{Deserialize, Serialize};

/// Validation status of a [`ContentSource`] after hydration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContentSourceStatus {
    /// Initial state before hydration validation.
    #[default]
    Unknown,
    /// Path exists and is readable.
    Active,
    /// Path does not exist on disk.
    Missing,
    /// Path exists but is not readable or violates workspace boundaries.
    Error,
}

impl ContentSourceStatus {
    /// Return the canonical snake_case string for this status.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Active => "active",
            Self::Missing => "missing",
            Self::Error => "error",
        }
    }
}

/// Built-in content types recognised by the ingestion pipeline.
pub const BUILT_IN_TYPES: &[&str] = &[
    "code",
    "tests",
    "spec",
    "docs",
    "memory",
    "context",
    "instructions",
    "backlog",
];

/// A single declared content source from `.engram/registry.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentSource {
    /// Content type label — built-in (see [`BUILT_IN_TYPES`]) or custom.
    #[serde(rename = "type")]
    pub content_type: String,

    /// Language hint used by the code graph indexer (e.g. `"rust"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Relative path from workspace root to the content directory.
    pub path: String,

    /// Optional glob pattern for filtering files within the source directory.
    ///
    /// When set, only files whose path relative to the source directory matches
    /// the pattern are ingested. Standard glob syntax applies: `*` matches any
    /// sequence of non-separator characters, `**` matches across directory
    /// separators, `?` matches a single character, and `[abc]` matches a
    /// character class. Patterns are matched case-sensitively.
    ///
    /// Examples:
    /// - `"*-research.md"` — files whose name ends with `-research.md`
    /// - `"**/*.md"` — all Markdown files anywhere in the directory tree
    /// - `"tasks/**"` — everything under a `tasks/` subdirectory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Validation status set at hydration time (not serialized in YAML).
    #[serde(skip)]
    pub status: ContentSourceStatus,
}

/// Default maximum file size for ingestion (1 MB).
const DEFAULT_MAX_FILE_SIZE: u64 = 1_048_576;

/// Default batch size for ingestion.
const DEFAULT_BATCH_SIZE: usize = 50;

/// Top-level configuration parsed from `.engram/registry.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// List of declared content sources.
    #[serde(default)]
    pub sources: Vec<ContentSource>,

    /// Maximum file size for ingestion in bytes (default: 1 MB).
    #[serde(default = "default_max_file_size")]
    pub max_file_size_bytes: u64,

    /// Files per ingestion batch (default: 50).
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_max_file_size() -> u64 {
    DEFAULT_MAX_FILE_SIZE
}

fn default_batch_size() -> usize {
    DEFAULT_BATCH_SIZE
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            max_file_size_bytes: DEFAULT_MAX_FILE_SIZE,
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }
}
