//! Data models for the metrics subsystem.
//!
//! Provides usage event recording, summary aggregation, and configuration
//! types for measuring engram's token delivery to AI coding assistants.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Message types for the metrics background writer channel.
#[derive(Debug)]
pub enum MetricsMessage {
    /// A usage event to record.
    Event(UsageEvent),
    /// Switch the active branch output path.
    SwitchBranch(String),
    /// Drain buffered events and shut down.
    Shutdown,
}

/// A single tool call usage measurement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageEvent {
    /// MCP tool method name (e.g., `"map_code"`, `"unified_search"`).
    pub tool_name: String,
    /// RFC 3339 timestamp of the tool call.
    pub timestamp: String,
    /// Response payload size in bytes.
    pub response_bytes: u64,
    /// Estimated token count (`response_bytes / 4`).
    pub estimated_tokens: u64,
    /// Number of symbols returned (tool-specific extraction).
    pub symbols_returned: u32,
    /// Number of result items returned.
    pub results_returned: u32,
    /// Active Git branch (already sanitized by `resolve_git_branch`).
    pub branch: String,
    /// SSE connection UUID, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
}

/// Aggregated metrics for a branch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsSummary {
    /// Total tool calls recorded.
    pub total_tool_calls: u64,
    /// Total estimated tokens delivered to agents.
    pub total_tokens: u64,
    /// Per-tool breakdown (deterministic ordering via `BTreeMap`).
    pub by_tool: BTreeMap<String, ToolMetrics>,
    /// Top queried symbols by frequency.
    pub top_symbols: Vec<SymbolCount>,
    /// Time range covered by this summary.
    pub time_range: TimeRange,
    /// Distinct session count.
    pub session_count: u32,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeRange {
    /// RFC 3339 start timestamp.
    pub start: String,
    /// RFC 3339 end timestamp.
    pub end: String,
}

/// Configuration for the metrics subsystem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether metrics collection is enabled.
    #[serde(default = "default_metrics_enabled")]
    pub enabled: bool,
    /// Bounded channel buffer size for the background writer.
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
}

fn default_metrics_enabled() -> bool {
    true
}

fn default_buffer_size() -> usize {
    1024
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_metrics_enabled(),
            buffer_size: default_buffer_size(),
        }
    }
}

impl MetricsSummary {
    /// Compute an aggregated summary from a list of usage events.
    #[allow(clippy::cast_precision_loss)]
    pub fn from_events(_events: &[UsageEvent]) -> Self {
        unimplemented!(
            "Worker: Aggregate events into MetricsSummary — group by tool_name \
             into BTreeMap<String, ToolMetrics>, compute avg_tokens as \
             total_tokens / call_count, collect top 10 symbols from \
             tool_name frequencies, extract time_range from first/last \
             event timestamps, count unique connection_ids for session_count"
        )
    }
}
