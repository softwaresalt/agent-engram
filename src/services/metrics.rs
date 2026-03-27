//! Metrics collection service for tracking tool call token usage.
//!
//! Provides non-blocking event recording via a `tokio::sync::mpsc` channel
//! and summary computation from persisted JSONL files.

use std::path::Path;

use crate::errors::EngramError;
use crate::models::metrics::{MetricsSummary, UsageEvent};

/// Record a usage event to the metrics channel (non-blocking).
///
/// If the channel is full, the event is dropped with a `tracing::trace!`
/// log. This ensures zero latency impact on tool call responses.
pub fn record(_event: UsageEvent) {
    unimplemented!(
        "Worker: Wrap event in MetricsMessage::Event and try_send on the \
         global OnceLock<mpsc::Sender<MetricsMessage>>. If the channel is \
         full, emit tracing::trace!(\"metrics_event_dropped\") and return."
    )
}

/// Compute a `MetricsSummary` from the `usage.jsonl` file on disk.
///
/// Reads `{workspace_path}/.engram/metrics/{branch}/usage.jsonl` line by
/// line, deserializes each line as a `UsageEvent`, and aggregates into a
/// `MetricsSummary`. Silently discards the final line if it fails to parse
/// (concurrent-append tolerance).
pub fn compute_summary(
    _workspace_path: &Path,
    _branch: &str,
) -> Result<MetricsSummary, EngramError> {
    unimplemented!(
        "Worker: Read workspace_path/.engram/metrics/branch/usage.jsonl \
         line-by-line, parse each line as UsageEvent via serde_json::from_str, \
         discard the final line if parse fails, call MetricsSummary::from_events \
         on the collected events. Return MetricsError::NotFound if the file \
         does not exist."
    )
}

/// Compute and atomically write `summary.json` for a branch.
///
/// Calls [`compute_summary`] then writes the result using
/// `dehydration::atomic_write`.
pub async fn compute_and_write_summary(
    _workspace_path: &Path,
    _branch: &str,
) -> Result<(), EngramError> {
    unimplemented!(
        "Worker: Call compute_summary(workspace_path, branch), serialize the \
         MetricsSummary to JSON via serde_json::to_string_pretty, then call \
         dehydration::atomic_write to \
         workspace_path/.engram/metrics/branch/summary.json"
    )
}
