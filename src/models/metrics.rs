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
    pub fn from_events(events: &[UsageEvent]) -> Self {
        let mut by_tool: BTreeMap<String, ToolMetrics> = BTreeMap::new();
        let mut symbol_counts: BTreeMap<String, u32> = BTreeMap::new();
        let mut total_tokens = 0_u64;
        let mut session_ids = std::collections::BTreeSet::new();

        for event in events {
            total_tokens += event.estimated_tokens;
            let entry = by_tool
                .entry(event.tool_name.clone())
                .or_insert_with(|| ToolMetrics {
                    call_count: 0,
                    total_tokens: 0,
                    avg_tokens: 0.0,
                });
            entry.call_count += 1;
            entry.total_tokens += event.estimated_tokens;

            *symbol_counts.entry(event.tool_name.clone()).or_insert(0) += 1;

            if let Some(connection_id) = &event.connection_id {
                session_ids.insert(connection_id.clone());
            }
        }

        for metrics in by_tool.values_mut() {
            metrics.avg_tokens = if metrics.call_count == 0 {
                0.0
            } else {
                metrics.total_tokens as f64 / metrics.call_count as f64
            };
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

        let time_range = if let (Some(first), Some(last)) = (events.first(), events.last()) {
            TimeRange {
                start: first.timestamp.clone(),
                end: last.timestamp.clone(),
            }
        } else {
            TimeRange {
                start: String::new(),
                end: String::new(),
            }
        };

        Self {
            total_tool_calls: u64::try_from(events.len()).unwrap_or(u64::MAX),
            total_tokens,
            by_tool,
            top_symbols,
            time_range,
            session_count: u32::try_from(session_ids.len()).unwrap_or(u32::MAX),
        }
    }
}
