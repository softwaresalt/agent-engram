//! Workspace configuration models.
//!
//! Defines [`WorkspaceConfig`], [`CompactionConfig`], and [`BatchConfig`]
//! for user-customizable workspace behavior read from `.engram/config.toml`.

use serde::{Deserialize, Serialize};

/// Top-level workspace configuration read from `.engram/config.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Compaction settings.
    #[serde(default)]
    pub compaction: CompactionConfig,
    /// Batch operation settings.
    #[serde(default)]
    pub batch: BatchConfig,
    /// Default priority for new tasks.
    #[serde(default = "default_priority")]
    pub default_priority: String,
    /// Allowed label names (empty means any label is allowed).
    #[serde(default)]
    pub allowed_labels: Vec<String>,
    /// Allowed issue type names (empty means any type is allowed).
    #[serde(default)]
    pub allowed_types: Vec<String>,
    /// Code graph indexing and traversal settings.
    #[serde(default)]
    pub code_graph: CodeGraphConfig,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            compaction: CompactionConfig::default(),
            batch: BatchConfig::default(),
            default_priority: default_priority(),
            allowed_labels: Vec::new(),
            allowed_types: Vec::new(),
            code_graph: CodeGraphConfig::default(),
        }
    }
}

/// Compaction tuning knobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Minimum age in days before a done task is eligible for compaction.
    #[serde(default = "default_threshold_days")]
    pub threshold_days: u32,
    /// Maximum candidates returned per `get_compaction_candidates` call.
    #[serde(default = "default_max_candidates")]
    pub max_candidates: u32,
    /// Maximum character length for rule-based truncation fallback.
    #[serde(default = "default_truncation_length")]
    pub truncation_length: u32,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            threshold_days: default_threshold_days(),
            max_candidates: default_max_candidates(),
            truncation_length: default_truncation_length(),
        }
    }
}

/// Batch operation limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum items per batch_update_tasks call.
    #[serde(default = "default_max_size")]
    pub max_size: u32,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_size: default_max_size(),
        }
    }
}

fn default_priority() -> String {
    "p2".to_owned()
}

const fn default_threshold_days() -> u32 {
    7
}

const fn default_max_candidates() -> u32 {
    50
}

const fn default_truncation_length() -> u32 {
    500
}

const fn default_max_size() -> u32 {
    100
}

/// Code graph indexing and traversal configuration.
///
/// Read from the `[code_graph]` section of `.engram/config.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeGraphConfig {
    /// Glob patterns to exclude from indexing (in addition to `.gitignore`).
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    /// Maximum file size in bytes for indexing (default: 1 MB).
    #[serde(default = "default_max_file_size_bytes")]
    pub max_file_size_bytes: u64,
    /// Number of parallel parsing tasks (0 = auto-detect CPU count).
    #[serde(default)]
    pub parse_concurrency: usize,
    /// Maximum BFS traversal depth for `map_code` and `impact_analysis`.
    #[serde(default = "default_max_traversal_depth")]
    pub max_traversal_depth: usize,
    /// Maximum nodes returned by traversal queries.
    #[serde(default = "default_max_traversal_nodes")]
    pub max_traversal_nodes: usize,
    /// Languages supported for AST parsing.
    #[serde(default = "default_supported_languages")]
    pub supported_languages: Vec<String>,
    /// Embedding-specific settings.
    #[serde(default)]
    pub embedding: EmbeddingConfig,
}

impl Default for CodeGraphConfig {
    fn default() -> Self {
        Self {
            exclude_patterns: Vec::new(),
            max_file_size_bytes: default_max_file_size_bytes(),
            parse_concurrency: 0,
            max_traversal_depth: default_max_traversal_depth(),
            max_traversal_nodes: default_max_traversal_nodes(),
            supported_languages: default_supported_languages(),
            embedding: EmbeddingConfig::default(),
        }
    }
}

/// Embedding behaviour for the code graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Token limit for Tier 1 (explicit_code) embedding.
    ///
    /// Bodies with `token_count` ≤ this limit embed the raw source;
    /// bodies exceeding it use the `summary_pointer` strategy.
    #[serde(default = "default_token_limit")]
    pub token_limit: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            token_limit: default_token_limit(),
        }
    }
}

const fn default_max_file_size_bytes() -> u64 {
    1_048_576
}

const fn default_max_traversal_depth() -> usize {
    5
}

const fn default_max_traversal_nodes() -> usize {
    50
}

fn default_supported_languages() -> Vec<String> {
    vec!["rust".to_owned()]
}

const fn default_token_limit() -> usize {
    512
}
