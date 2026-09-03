//! Workspace configuration models.
//!
//! Defines [`WorkspaceConfig`], [`BatchConfig`], and [`PluginConfig`] for
//! user-customizable workspace and daemon behavior read from
//! `.engram/config.toml`.

use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use thiserror::Error;

use crate::models::evaluation::EvaluationConfig;
use crate::models::lineage::LineageAuthorityContext;
use crate::models::metrics::MetricsConfig;
use crate::models::policy::PolicyConfig;
use crate::models::retrieval_eval::RetrievalEvalConfig;

/// Top-level workspace configuration read from `.engram/config.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Batch operation settings.
    #[serde(default)]
    pub batch: BatchConfig,
    /// Code graph indexing and traversal settings.
    #[serde(default)]
    pub code_graph: CodeGraphConfig,
    /// Metrics collection and persistence settings.
    #[serde(default)]
    pub metrics: MetricsConfig,
    /// Per-agent tool access policy.
    #[serde(default)]
    pub policy: PolicyConfig,
    /// Agent efficiency evaluation configuration.
    #[serde(default)]
    pub evaluation: EvaluationConfig,
    /// Portable retrieval + graph-recall evaluation configuration.
    ///
    /// Distinct from the agent-efficiency [`EvaluationConfig`]; disabled by
    /// default. Read from the `[retrieval_eval]` section.
    #[serde(default)]
    pub retrieval_eval: RetrievalEvalConfig,
    /// Timeout in milliseconds for sandboxed graph queries (`query_graph` tool).
    ///
    /// Queries that exceed this limit are cancelled with a `QUERY_TIMEOUT` error.
    #[serde(default = "default_query_timeout_ms")]
    pub query_timeout_ms: u64,
    /// Maximum number of rows returned by a single sandboxed graph query.
    ///
    /// Results beyond this limit are truncated and the response sets
    /// `"truncated": true`.
    #[serde(default = "default_query_row_limit")]
    pub query_row_limit: usize,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            batch: BatchConfig::default(),
            code_graph: CodeGraphConfig::default(),
            metrics: MetricsConfig::default(),
            policy: PolicyConfig::default(),
            evaluation: EvaluationConfig::default(),
            retrieval_eval: RetrievalEvalConfig::default(),
            query_timeout_ms: default_query_timeout_ms(),
            query_row_limit: default_query_row_limit(),
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

const fn default_query_timeout_ms() -> u64 {
    5_000
}

const fn default_query_row_limit() -> usize {
    1_000
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
    vec![
        "rust".to_owned(),
        "python".to_owned(),
        "typescript".to_owned(),
        "tsx".to_owned(),
        "javascript".to_owned(),
        "go".to_owned(),
        "csharp".to_owned(),
        "hcl".to_owned(),
    ]
}

const fn default_token_limit() -> usize {
    512
}

// ── DaemonMode ────────────────────────────────────────────────────────────────

/// Strict daemon operating mode (142-F).
///
/// [`DaemonMode::resolve`] is the single shared mode resolver for the whole
/// daemon; no other parsing logic for this setting exists anywhere else. The
/// managed default applies only when the setting is absent (`None`). A
/// present-but-unrecognized value is a typed [`DaemonModeParseError`] — never
/// a silent fallback to managed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonMode {
    /// Legacy indexing-and-serving daemon (today's only behavior). The
    /// resolved default when no mode setting is present.
    Managed,
    /// Read-only generation-serving daemon that never indexes and serves only
    /// immutable, atomically published generations (142-F).
    ReadServer,
}

impl DaemonMode {
    /// Canonical setting string for this mode.
    ///
    /// Round-trips through [`DaemonMode::resolve`]:
    /// `DaemonMode::resolve(Some(mode.as_str())) == Ok(mode)` for both
    /// variants.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::ReadServer => "read_server",
        }
    }

    /// Strictly resolve the effective mode from an optional raw setting.
    ///
    /// * `None` (the setting is absent) resolves to [`DaemonMode::Managed`],
    ///   the managed default.
    /// * `Some("managed")` resolves to [`DaemonMode::Managed`].
    /// * `Some("read_server")` resolves to [`DaemonMode::ReadServer`].
    /// * Any other `Some(_)` value is a hard parse error
    ///   ([`DaemonModeParseError::Unrecognized`]) — a present-but-invalid
    ///   value never silently falls back to managed.
    ///
    /// # Errors
    ///
    /// Returns [`DaemonModeParseError::Unrecognized`] when `raw` is `Some`
    /// and does not exactly match `"managed"` or `"read_server"`.
    pub fn resolve(raw: Option<&str>) -> Result<Self, DaemonModeParseError> {
        match raw {
            None => Ok(Self::Managed),
            Some(value) if value == Self::Managed.as_str() => Ok(Self::Managed),
            Some(value) if value == Self::ReadServer.as_str() => Ok(Self::ReadServer),
            Some(other) => Err(DaemonModeParseError::Unrecognized(other.to_owned())),
        }
    }
}

impl std::fmt::Display for DaemonMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned by [`DaemonMode::resolve`] for a present-but-unrecognized
/// daemon-mode setting.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DaemonModeParseError {
    /// The raw setting was present but did not match a known mode.
    #[error("unrecognized daemon mode setting {0:?}; expected \"managed\" or \"read_server\"")]
    Unrecognized(String),
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
    /// Raw daemon-mode setting (`mode = "managed"` / `"read_server"` in
    /// `.engram/config.toml`). `None` when unset.
    ///
    /// This field intentionally stays a raw, permissive string rather than a
    /// typed [`DaemonMode`] so a malformed value does not abort parsing of
    /// the rest of the config file. Resolve it through
    /// [`DaemonMode::resolve`] — the single shared mode resolver — which
    /// returns a hard [`DaemonModeParseError`] for a present-but-unrecognized
    /// value rather than silently falling back to managed.
    #[serde(default)]
    pub mode: Option<String>,
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
            mode: None,
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
        ".copilot-tracking/".to_owned(),
        ".copilot/".to_owned(),
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

/// Trusted-authority configuration for notebook data-lineage extraction (095-F).
///
/// Read from the `lineage:` section of the ingestion registry
/// (`.engram/registry.yaml`). This is the config surface that closes cycle-5 F1:
/// without it the live indexer has no trusted metastore/storage authority, so no
/// `catalog.schema.table` or path literal can ever bind and lineage is
/// unreachable in production. See `docs/architecture.md` (*Enabling lineage
/// (operator configuration)*) for a worked YAML example.
///
/// Fail-closed (013-D / AR-01): an absent or empty section yields an **empty**
/// [`LineageAuthorityContext`] via [`LineageConfig::to_authority_context`], so
/// every dataset identity stays unresolved and **no edge** is emitted — never a
/// bare-name guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LineageConfig {
    /// Stable id of the trusted metastore/catalog authority.
    ///
    /// Embedded in every canonical `dataset_node` id so two metastores that
    /// share a `catalog.schema.table` never collide (AR-01). An **empty** id
    /// disables the table side entirely (every catalog is unmapped).
    pub metastore_authority_id: String,
    /// Catalog name → trusted metastore authority id.
    ///
    /// A catalog absent from this map is **unmapped** and fails closed. An empty
    /// mapping value inherits [`LineageConfig::metastore_authority_id`], which is
    /// the common single-metastore case.
    pub catalog_authorities: BTreeMap<String, String>,
    /// Default catalog for the metastore, bound to `metastore_authority_id`.
    ///
    /// Carried for future 1-/2-part qualification; v1 still **drops** 1-/2-part
    /// names, so this only adds the default catalog to the trusted set.
    pub default_catalog: Option<String>,
    /// Default schema for the metastore.
    ///
    /// Carried for future qualification; unused by v1 resolution (fail-closed on
    /// 1-/2-part names regardless).
    pub default_schema: Option<String>,
    /// Trusted storage-authority prefixes (e.g. `s3://bucket`,
    /// `abfss://c@a.dfs.core.windows.net`).
    ///
    /// A path whose `scheme://authority` matches none of these fails closed.
    /// Independent of the metastore id — paths resolve on the storage allowlist
    /// alone.
    pub storage_authorities: Vec<String>,
}

impl LineageConfig {
    /// Build the fail-closed [`LineageAuthorityContext`] this config authorizes.
    ///
    /// The table side is enabled only when `metastore_authority_id` is set: each
    /// configured catalog binds to its mapped authority id (inheriting
    /// `metastore_authority_id` when the mapping value is empty), and
    /// `default_catalog` (if any) also binds to it. The storage allowlist is
    /// always carried through and governs paths independently. An empty config
    /// yields an empty context that resolves nothing.
    #[must_use]
    pub fn to_authority_context(&self) -> LineageAuthorityContext {
        let mut catalog_authority = BTreeMap::new();
        if !self.metastore_authority_id.is_empty() {
            for (catalog, authority) in &self.catalog_authorities {
                if catalog.is_empty() {
                    continue;
                }
                let resolved = if authority.is_empty() {
                    self.metastore_authority_id.clone()
                } else {
                    authority.clone()
                };
                catalog_authority.insert(catalog.clone(), resolved);
            }
            if let Some(default_catalog) = self.default_catalog.as_deref() {
                if !default_catalog.is_empty() {
                    catalog_authority
                        .entry(default_catalog.to_owned())
                        .or_insert_with(|| self.metastore_authority_id.clone());
                }
            }
        }
        LineageAuthorityContext::new(catalog_authority, self.storage_authorities.clone())
    }
}

#[cfg(test)]
mod lineage_config_tests {
    use super::LineageConfig;
    use std::collections::BTreeMap;

    fn cfg_with(metastore: &str, catalogs: &[(&str, &str)], storage: &[&str]) -> LineageConfig {
        let mut catalog_authorities = BTreeMap::new();
        for (catalog, authority) in catalogs {
            catalog_authorities.insert((*catalog).to_owned(), (*authority).to_owned());
        }
        LineageConfig {
            metastore_authority_id: metastore.to_owned(),
            catalog_authorities,
            default_catalog: None,
            default_schema: None,
            storage_authorities: storage.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    // AR-01: the same catalog.schema.table under two DIFFERENT metastore
    // authorities canonicalizes to DISTINCT dataset_node ids (never a merge).
    #[test]
    fn cross_authority_distinctness_yields_distinct_ids() {
        let a = cfg_with("metastore-a", &[("cat", "")], &[]).to_authority_context();
        let b = cfg_with("metastore-b", &[("cat", "")], &[]).to_authority_context();
        let ra = a.resolve_table("cat.sch.t").expect("A resolves");
        let rb = b.resolve_table("cat.sch.t").expect("B resolves");
        assert_ne!(
            ra.id, rb.id,
            "the same 3-part name under two metastores must be distinct nodes"
        );
        assert!(ra.id.contains("metastore-a"));
        assert!(rb.id.contains("metastore-b"));
    }

    // AR-01: a catalog with no authority mapping resolves to NO authority, so
    // NO node/edge is produced (fail-closed).
    #[test]
    fn unmapped_catalog_fails_closed() {
        let ctx = cfg_with("m", &[("known", "")], &[]).to_authority_context();
        assert!(
            ctx.resolve_table("known.sch.t").is_some(),
            "a mapped catalog still resolves"
        );
        assert!(
            ctx.resolve_table("unknown.sch.t").is_none(),
            "an unmapped catalog must fail closed"
        );
    }

    // Fail-closed parity: with NO authority configured (default/empty section)
    // the same reference produces ZERO table/path resolutions.
    #[test]
    fn empty_config_resolves_nothing() {
        let ctx = LineageConfig::default().to_authority_context();
        assert!(ctx.is_empty());
        assert!(ctx.resolve_table("cat.sch.t").is_none());
        assert!(ctx.resolve_path("s3://bucket/p").is_none());
    }

    // An empty metastore id disables the table side entirely even when catalogs
    // are listed, while the storage allowlist still governs paths independently.
    #[test]
    fn absent_metastore_disables_tables_but_storage_is_independent() {
        let ctx = cfg_with("", &[("cat", "auth")], &["s3://bucket"]).to_authority_context();
        assert!(
            ctx.resolve_table("cat.sch.t").is_none(),
            "no metastore id => no trusted table authority"
        );
        assert!(
            ctx.resolve_path("s3://bucket/data").is_some(),
            "the storage allowlist governs paths independently of the metastore id"
        );
    }
}
