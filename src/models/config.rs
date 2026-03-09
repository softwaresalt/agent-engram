//! Workspace configuration models.
//!
//! Defines [`WorkspaceConfig`], [`CompactionConfig`], [`BatchConfig`], and
//! [`PluginConfig`] for user-customizable workspace and daemon behavior read
//! from `.engram/config.toml`.

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
    /// Maximum number of events retained in the event ledger.
    ///
    /// When the ledger exceeds this count, the oldest events are pruned.
    /// Corresponds to `ENGRAM_EVENT_LEDGER_MAX` in the global CLI config.
    #[serde(default = "default_event_ledger_max")]
    pub event_ledger_max: usize,
    /// Whether MCP clients are permitted to invoke `rollback_to_event`.
    ///
    /// Disabled by default for safety; enable via `.engram/config.toml`
    /// or the `ENGRAM_ALLOW_AGENT_ROLLBACK` environment variable.
    #[serde(default)]
    pub allow_agent_rollback: bool,
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
            event_ledger_max: default_event_ledger_max(),
            allow_agent_rollback: false,
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

const fn default_event_ledger_max() -> usize {
    500
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

// ── PluginConfig ──────────────────────────────────────────────────────────────

/// User-configurable settings loaded from `.engram/config.toml` at daemon startup.
///
/// Unknown fields are silently ignored (serde default behaviour — `deny_unknown_fields`
/// is intentionally omitted). Missing fields receive their declared defaults.
///
/// # Examples
///
/// ```toml
/// idle_timeout_minutes = 30
/// debounce_ms = 250
/// exclude_patterns = [".engram/", ".git/", "target/"]
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Minutes of inactivity before daemon self-terminates (0 = never).
    #[serde(default = "default_idle_timeout_minutes")]
    pub idle_timeout_minutes: u64,
    /// Milliseconds to debounce file-system events.
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    /// Glob patterns for files to watch.
    #[serde(default = "default_watch_patterns")]
    pub watch_patterns: Vec<String>,
    /// Glob patterns for files to exclude from watching.
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
    /// Daemon log verbosity (e.g. `"info"`, `"debug"`, `"warn"`).
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Log output format (`"pretty"` or `"json"`).
    #[serde(default = "default_log_format")]
    pub log_format: String,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            idle_timeout_minutes: default_idle_timeout_minutes(),
            debounce_ms: default_debounce_ms(),
            watch_patterns: default_watch_patterns(),
            exclude_patterns: default_exclude_patterns(),
            log_level: default_log_level(),
            log_format: default_log_format(),
        }
    }
}

impl PluginConfig {
    /// Convert `idle_timeout_minutes` to a [`std::time::Duration`].
    ///
    /// Returns [`std::time::Duration::ZERO`] when `idle_timeout_minutes` is 0,
    /// which the daemon interprets as "run forever".
    pub fn idle_timeout(&self) -> std::time::Duration {
        if self.idle_timeout_minutes == 0 {
            std::time::Duration::ZERO
        } else {
            std::time::Duration::from_secs(self.idle_timeout_minutes * 60)
        }
    }

    /// Load config from `.engram/config.toml` inside `workspace`.
    ///
    /// Falls back to [`PluginConfig::default`] when the file is absent or
    /// contains invalid TOML; a `warn`-level trace event is emitted in the
    /// latter case so the operator can diagnose the problem.
    pub fn load(workspace: &std::path::Path) -> Self {
        let config_path = workspace.join(".engram").join("config.toml");
        match std::fs::read_to_string(&config_path) {
            Err(_) => {
                tracing::debug!(
                    "no config.toml found at {config_path}; using defaults",
                    config_path = config_path.display()
                );
                Self::default()
            }
            Ok(content) => match toml::from_str::<Self>(&content) {
                Ok(cfg) => {
                    tracing::info!(
                        path = %config_path.display(),
                        "loaded plugin config from config.toml"
                    );
                    cfg
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        path = %config_path.display(),
                        "failed to parse config.toml; using defaults"
                    );
                    Self::default()
                }
            },
        }
    }
}

const fn default_idle_timeout_minutes() -> u64 {
    240 // 4 hours
}

const fn default_debounce_ms() -> u64 {
    500
}

fn default_watch_patterns() -> Vec<String> {
    vec!["**/*".to_owned()]
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        ".engram/".to_owned(),
        ".git/".to_owned(),
        "node_modules/".to_owned(),
        "target/".to_owned(),
        ".env*".to_owned(),
    ]
}

fn default_log_level() -> String {
    "info".to_owned()
}

fn default_log_format() -> String {
    "pretty".to_owned()
}
