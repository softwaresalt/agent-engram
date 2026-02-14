//! Workspace configuration models.
//!
//! Defines [`WorkspaceConfig`], [`CompactionConfig`], and [`BatchConfig`]
//! for user-customizable workspace behavior read from `.tmem/config.toml`.

use serde::{Deserialize, Serialize};

/// Top-level workspace configuration read from `.tmem/config.toml`.
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
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            compaction: CompactionConfig::default(),
            batch: BatchConfig::default(),
            default_priority: default_priority(),
            allowed_labels: Vec::new(),
            allowed_types: Vec::new(),
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
