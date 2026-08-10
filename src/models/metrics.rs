//! Data models for the metrics subsystem.
//!
//! Provides usage event recording, summary aggregation, and configuration
//! types for measuring engram's token delivery to AI coding assistants.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Current usage-telemetry record schema version (pinned public contract).
///
/// Autoharness parses `.engram/metrics/{branch}/usage.jsonl` against this
/// version. New fields are additive-only; existing fields are never renamed
/// or removed so older records keep deserializing.
pub const USAGE_SCHEMA_VERSION: u32 = 2;

/// Maximum accepted length (in characters) for a caller-supplied correlation id.
pub const CORRELATION_ID_MAX_LEN: usize = 128;

fn default_outcome() -> String {
    "success".to_string()
}

fn default_schema_version() -> u32 {
    USAGE_SCHEMA_VERSION
}

/// Deterministic, non-cryptographic hex hash (FNV-1a, 64-bit) of `input`.
///
/// Stable across processes and platforms so a persisted `query_hash` is a
/// usable bucket key for autoharness. Not for security use.
#[must_use]
pub fn stable_hash_hex(input: &str) -> String {
    // FNV-1a, 64-bit.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Sanitize a caller-supplied correlation id for safe persistence in a JSONL line.
///
/// Strips control characters (including `\n`/`\r`/`\t`) that could forge or split
/// a JSONL record, then truncates to [`CORRELATION_ID_MAX_LEN`] characters. Returns
/// `None` when nothing usable remains (empty or all-control input).
///
/// This is the **envelope** policy (MCP `_meta.correlation_id`): never fail a live
/// tool call — sanitize-and-truncate instead.
#[must_use]
pub fn sanitize_correlation_id(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !c.is_control())
        .take(CORRELATION_ID_MAX_LEN)
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

/// Validate a caller-supplied correlation id, rejecting invalid input.
///
/// This is the **CLI/direct** policy: fail fast for a human-driven surface.
/// An empty id yields `Ok(None)` (treated as "not supplied").
///
/// # Errors
///
/// Returns `Err(String)` describing why the id was rejected when it contains
/// control characters (JSONL line-integrity risk) or exceeds
/// [`CORRELATION_ID_MAX_LEN`] characters.
pub fn validate_correlation_id(raw: &str) -> Result<Option<String>, String> {
    if raw.chars().count() > CORRELATION_ID_MAX_LEN {
        return Err(format!(
            "correlation id exceeds {CORRELATION_ID_MAX_LEN} characters"
        ));
    }
    if raw.chars().any(char::is_control) {
        return Err("correlation id contains control characters".to_owned());
    }
    if raw.is_empty() {
        return Ok(None);
    }
    Ok(Some(raw.to_owned()))
}

fn default_response_shape_counts() -> BTreeMap<String, u32> {
    BTreeMap::new()
}

/// A single tool call usage measurement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageEvent {
    /// MCP tool method name (e.g., `"map_code"`, `"unified_search"`).
    pub tool_name: String,
    /// RFC 3339 timestamp of the tool call.
    pub timestamp: String,
    /// Serialized request payload size in bytes.
    #[serde(default)]
    pub request_bytes: u64,
    /// Estimated request token count (`request_bytes / 4`).
    #[serde(default)]
    pub estimated_input_tokens: u64,
    /// Response payload size in bytes.
    pub response_bytes: u64,
    /// Estimated response token count (`response_bytes / 4`).
    #[serde(default)]
    pub estimated_output_tokens: u64,
    /// Estimated token count (`response_bytes / 4`).
    ///
    /// Retained as a compatibility alias for output-side token counting.
    pub estimated_tokens: u64,
    /// Canonical number of result items returned by the tool.
    #[serde(default)]
    pub result_count: u32,
    /// Deterministic response-shape counters keyed by semantic bucket.
    #[serde(
        default = "default_response_shape_counts",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub response_shape_counts: BTreeMap<String, u32>,
    /// Number of symbols returned (tool-specific extraction).
    pub symbols_returned: u32,
    /// Number of result items returned.
    pub results_returned: u32,
    /// Active Git branch (already sanitized by `resolve_git_branch`).
    pub branch: String,
    /// SSE connection UUID, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    /// Agent role identity from `_meta.agent_role`, if provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_role: Option<String>,
    /// Outcome of the tool call (e.g., `"success"`, `"error"`).
    #[serde(default = "default_outcome")]
    pub outcome: String,
    /// Runtime-attributed prompt tokens when available from a higher layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_attributed: Option<u64>,
    /// Runtime-attributed completion tokens when available from a higher layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_attributed: Option<u64>,
    /// Runtime-attributed cached tokens when available from a higher layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens_attributed: Option<u64>,
    /// Telemetry record schema version (pinned; autoharness contract).
    ///
    /// Defaults to [`USAGE_SCHEMA_VERSION`] for records written before this
    /// field existed (back-compat deserialization).
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// Caller-supplied correlation id (dual-source: MCP `_meta.correlation_id`
    /// or CLI `--correlation-id`/`ENGRAM_CORRELATION_ID`). Omitted when neither
    /// source supplied one. Validated (control-char strip, 128-char cap) before
    /// persistence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Dispatch latency in milliseconds (measures the tool call, not emission).
    #[serde(default)]
    pub latency_ms: u64,
    /// Workspace root path (already resolved).
    #[serde(default)]
    pub workspace: String,
    /// Coarse, privacy-preserving parameter summary (never raw query text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params_summary: Option<CoarseParams>,
}

/// Coarse, privacy-preserving summary of request parameters.
///
/// Never stores raw query text — only a stable hash, the query length, and any
/// caller-supplied result limit — so autoharness can bucket queries without
/// leaking source content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CoarseParams {
    /// Stable non-cryptographic hash of the query text (hex), when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_hash: Option<String>,
    /// Length in characters of the query text, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_len: Option<u64>,
    /// Caller-supplied result limit, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}

impl CoarseParams {
    /// Build a coarse summary from an optional query string and result limit.
    ///
    /// Returns `None` when neither a query nor a limit is present, so the
    /// enclosing `params_summary` field is omitted.
    #[must_use]
    pub fn from_parts(query: Option<&str>, limit: Option<u64>) -> Option<Self> {
        if query.is_none() && limit.is_none() {
            return None;
        }
        Some(Self {
            query_hash: query.map(stable_hash_hex),
            query_len: query.map(|q| u64::try_from(q.chars().count()).unwrap_or(u64::MAX)),
            limit,
        })
    }
}

impl Default for UsageEvent {
    fn default() -> Self {
        Self {
            tool_name: String::new(),
            timestamp: String::new(),
            request_bytes: 0,
            estimated_input_tokens: 0,
            response_bytes: 0,
            estimated_output_tokens: 0,
            estimated_tokens: 0,
            result_count: 0,
            response_shape_counts: BTreeMap::new(),
            symbols_returned: 0,
            results_returned: 0,
            branch: String::new(),
            connection_id: None,
            agent_role: None,
            outcome: default_outcome(),
            prompt_tokens_attributed: None,
            completion_tokens_attributed: None,
            cached_tokens_attributed: None,
            schema_version: default_schema_version(),
            correlation_id: None,
            latency_ms: 0,
            workspace: String::new(),
            params_summary: None,
        }
    }
}

/// Aggregated metrics for a branch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsSummary {
    /// Total tool calls recorded.
    pub total_tool_calls: u64,
    /// Total estimated tokens delivered to agents.
    pub total_tokens: u64,
    /// Total serialized request bytes recorded across events.
    pub total_request_bytes: u64,
    /// Total serialized response bytes recorded across events.
    pub total_response_bytes: u64,
    /// Total estimated request tokens across events.
    pub total_input_tokens: u64,
    /// Total estimated response tokens across events.
    pub total_output_tokens: u64,
    /// Total result items across events.
    pub total_result_count: u64,
    /// Per-tool breakdown (deterministic ordering via `BTreeMap`).
    pub by_tool: BTreeMap<String, ToolMetrics>,
    /// Top queried symbols by frequency.
    pub top_symbols: Vec<SymbolCount>,
    /// Time range covered by this summary.
    pub time_range: TimeRange,
    /// Distinct session count.
    pub session_count: u32,
    /// Count of distinct tools exercised across all recorded events.
    ///
    /// Adoption-breadth signal: how many of engram's tool surfaces the harness
    /// actually touched. Equals `by_tool.len()`. Defaults to `0` for summaries
    /// deserialized from records written before this field existed. Cheap
    /// scalar — safe to surface on every metrics-bearing tool.
    #[serde(default)]
    pub unique_tools_exercised: u32,
    /// Count of distinct non-empty correlation ids observed.
    ///
    /// Adoption-reach signal: how many distinct harness tasks/sessions
    /// (identified by `correlation_id`) invoked engram at least once. Events
    /// without a correlation id do not contribute to this count. Cheap scalar —
    /// safe to surface on every metrics-bearing tool. The full per-correlation
    /// breakdown ([`correlation_metrics`]) is intentionally kept off this shared
    /// struct so it does not bloat frequently-polled tools like
    /// `get_health_report`; it is surfaced only by `get_token_savings_report`.
    #[serde(default)]
    pub distinct_correlation_ids: u32,
}

/// Per-correlation-id usage metrics.
///
/// Answers "how much did a single harness task/session use engram?" for one
/// `correlation_id`. Used to quantify the extent of engram adoption per unit of
/// harness work.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CorrelationMetrics {
    /// Number of tool calls recorded for this correlation id.
    pub call_count: u64,
    /// Count of distinct tools exercised under this correlation id.
    pub unique_tools: u32,
    /// Time range spanned by this correlation id's events.
    pub time_range: TimeRange,
}

/// Per-tool metrics breakdown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolMetrics {
    /// Number of calls to this tool.
    pub call_count: u64,
    /// Total tokens delivered by this tool.
    pub total_tokens: u64,
    /// Average tokens per call.
    pub avg_tokens: f64,
    /// Total serialized request bytes for this tool.
    pub total_request_bytes: u64,
    /// Total serialized response bytes for this tool.
    pub total_response_bytes: u64,
    /// Total estimated request tokens for this tool.
    pub total_input_tokens: u64,
    /// Total estimated response tokens for this tool.
    pub total_output_tokens: u64,
    /// Total result items for this tool.
    pub total_result_count: u64,
}

/// Symbol with query frequency count.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SymbolCount {
    /// Symbol name.
    pub name: String,
    /// Number of times queried.
    pub count: u32,
}

/// Time range for a metrics collection period.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TimeRange {
    /// RFC 3339 start timestamp.
    pub start: String,
    /// RFC 3339 end timestamp.
    pub end: String,
}

/// Configuration for the metrics subsystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether metrics collection is enabled.
    #[serde(default = "default_metrics_enabled")]
    pub enabled: bool,
    /// Bounded channel buffer size for the background writer.
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
    /// Optional override for the usage.jsonl path.
    ///
    /// May be absolute or relative to the workspace root, but MUST resolve
    /// **within** the workspace root (containment is validated before use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_path_override: Option<String>,
    /// Maximum bytes before usage.jsonl is rotated (`0` = unbounded/no rotation).
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes: u64,
    /// Maximum number of rotated `usage.N.jsonl` files retained.
    #[serde(default = "default_max_rotated_files")]
    pub max_rotated_files: usize,
}

fn default_metrics_enabled() -> bool {
    true
}

fn default_buffer_size() -> usize {
    1024
}

const fn default_max_file_bytes() -> u64 {
    10 * 1024 * 1024
}

const fn default_max_rotated_files() -> usize {
    5
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_metrics_enabled(),
            buffer_size: default_buffer_size(),
            usage_path_override: None,
            max_file_bytes: default_max_file_bytes(),
            max_rotated_files: default_max_rotated_files(),
        }
    }
}

impl MetricsSummary {
    /// Compute an aggregated summary from a list of usage events.
    #[allow(clippy::cast_precision_loss)]
    pub fn from_events(events: &[UsageEvent]) -> Self {
        let mut by_tool: BTreeMap<String, ToolMetrics> = BTreeMap::new();
        let mut symbol_counts: BTreeMap<String, u32> = BTreeMap::new();
        let mut total_tokens = 0_u64;
        let mut total_request_bytes = 0_u64;
        let mut total_response_bytes = 0_u64;
        let mut total_input_tokens = 0_u64;
        let mut total_output_tokens = 0_u64;
        let mut total_result_count = 0_u64;
        let mut session_ids = std::collections::BTreeSet::new();
        // Distinct non-empty correlation ids (adoption-reach scalar). The full
        // per-correlation breakdown is computed separately by
        // `correlation_metrics` so it does not bloat the shared summary.
        let mut correlation_ids: BTreeSet<&str> = BTreeSet::new();

        for event in events {
            let output_tokens = event.output_tokens();
            let result_count = event.effective_result_count();

            total_tokens = total_tokens.saturating_add(output_tokens);
            total_request_bytes = total_request_bytes.saturating_add(event.request_bytes);
            total_response_bytes = total_response_bytes.saturating_add(event.response_bytes);
            total_input_tokens = total_input_tokens.saturating_add(event.estimated_input_tokens);
            total_output_tokens = total_output_tokens.saturating_add(output_tokens);
            total_result_count = total_result_count.saturating_add(u64::from(result_count));
            let entry = by_tool
                .entry(event.tool_name.clone())
                .or_insert_with(|| ToolMetrics {
                    call_count: 0,
                    total_tokens: 0,
                    avg_tokens: 0.0,
                    total_request_bytes: 0,
                    total_response_bytes: 0,
                    total_input_tokens: 0,
                    total_output_tokens: 0,
                    total_result_count: 0,
                });
            entry.call_count = entry.call_count.saturating_add(1);
            entry.total_tokens = entry.total_tokens.saturating_add(output_tokens);
            entry.total_request_bytes = entry
                .total_request_bytes
                .saturating_add(event.request_bytes);
            entry.total_response_bytes = entry
                .total_response_bytes
                .saturating_add(event.response_bytes);
            entry.total_input_tokens = entry
                .total_input_tokens
                .saturating_add(event.estimated_input_tokens);
            entry.total_output_tokens = entry.total_output_tokens.saturating_add(output_tokens);
            entry.total_result_count = entry
                .total_result_count
                .saturating_add(u64::from(result_count));

            let sym_count = symbol_counts.entry(event.tool_name.clone()).or_insert(0);
            *sym_count = sym_count.saturating_add(1);

            if let Some(connection_id) = &event.connection_id {
                session_ids.insert(connection_id.clone());
            }

            if let Some(correlation_id) =
                event.correlation_id.as_deref().filter(|id| !id.is_empty())
            {
                correlation_ids.insert(correlation_id);
            }
        }

        for metrics in by_tool.values_mut() {
            #[allow(clippy::cast_precision_loss)]
            let raw = if metrics.call_count == 0 {
                0.0
            } else {
                metrics.total_tokens as f64 / metrics.call_count as f64
            };
            // Round to 2 decimal places for stable JSON serialization round-trips.
            metrics.avg_tokens = (raw * 100.0).round() / 100.0;
        }

        let mut top_symbols: Vec<SymbolCount> = symbol_counts
            .into_iter()
            .map(|(name, count)| SymbolCount { name, count })
            .collect();
        top_symbols.sort_by(|left, right| {
            right
                .count
                .cmp(&left.count)
                .then_with(|| left.name.cmp(&right.name))
        });
        top_symbols.truncate(10);

        let time_range = {
            // Skip empty-string sentinels so a malformed record cannot pin the
            // range start to "". Emitter timestamps are canonical RFC-3339 with
            // a fixed `+00:00` offset, which is lexicographically orderable.
            let min_ts = events
                .iter()
                .map(|e| &e.timestamp)
                .filter(|t| !t.is_empty())
                .min()
                .cloned();
            let max_ts = events
                .iter()
                .map(|e| &e.timestamp)
                .filter(|t| !t.is_empty())
                .max()
                .cloned();
            match (min_ts, max_ts) {
                (Some(start), Some(end)) => TimeRange { start, end },
                _ => TimeRange {
                    start: String::new(),
                    end: String::new(),
                },
            }
        };

        Self {
            total_tool_calls: u64::try_from(events.len()).unwrap_or(u64::MAX),
            total_tokens,
            total_request_bytes,
            total_response_bytes,
            total_input_tokens,
            total_output_tokens,
            total_result_count,
            unique_tools_exercised: u32::try_from(by_tool.len()).unwrap_or(u32::MAX),
            distinct_correlation_ids: u32::try_from(correlation_ids.len()).unwrap_or(u32::MAX),
            by_tool,
            top_symbols,
            time_range,
            session_count: u32::try_from(session_ids.len()).unwrap_or(u32::MAX),
        }
    }
}

/// Compute the per-correlation-id usage breakdown from raw events.
///
/// Kept separate from [`MetricsSummary`] so this potentially large,
/// history-growing map is surfaced only by `get_token_savings_report` and never
/// bloats frequently-polled tools (`get_health_report`, `get_branch_metrics`) or
/// the persisted `summary.json`.
///
/// Events without a non-empty `correlation_id` are excluded (they still count in
/// the summary totals). Timestamps are assumed canonical RFC-3339 with a fixed
/// `+00:00` offset, so lexicographic min/max is order-preserving.
#[must_use]
pub fn correlation_metrics(events: &[UsageEvent]) -> BTreeMap<String, CorrelationMetrics> {
    // Accumulators: (call_count, distinct tools, min_ts, max_ts).
    let mut acc: BTreeMap<String, (u64, BTreeSet<String>, String, String)> = BTreeMap::new();
    for event in events {
        let Some(correlation_id) = event.correlation_id.as_deref().filter(|id| !id.is_empty())
        else {
            continue;
        };
        let entry = acc.entry(correlation_id.to_owned()).or_insert_with(|| {
            (
                0,
                BTreeSet::new(),
                event.timestamp.clone(),
                event.timestamp.clone(),
            )
        });
        entry.0 = entry.0.saturating_add(1);
        entry.1.insert(event.tool_name.clone());
        if !event.timestamp.is_empty() && (entry.2.is_empty() || event.timestamp < entry.2) {
            entry.2.clone_from(&event.timestamp);
        }
        if event.timestamp > entry.3 {
            entry.3.clone_from(&event.timestamp);
        }
    }
    acc.into_iter()
        .map(|(id, (call_count, tools, start, end))| {
            (
                id,
                CorrelationMetrics {
                    call_count,
                    unique_tools: u32::try_from(tools.len()).unwrap_or(u32::MAX),
                    time_range: TimeRange { start, end },
                },
            )
        })
        .collect()
}

impl UsageEvent {
    /// Return the canonical output-token count, falling back to the legacy field.
    #[must_use]
    pub fn output_tokens(&self) -> u64 {
        if self.estimated_output_tokens == 0 && self.estimated_tokens > 0 {
            self.estimated_tokens
        } else {
            self.estimated_output_tokens
        }
    }

    /// Return the canonical result count, falling back to the legacy field.
    #[must_use]
    pub fn effective_result_count(&self) -> u32 {
        if self.result_count == 0 && self.results_returned > 0 {
            self.results_returned
        } else {
            self.result_count
        }
    }
}
