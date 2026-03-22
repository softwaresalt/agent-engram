//! In-memory query timing statistics, aggregated per query type.
//!
//! Provides a lightweight fixed-size circular buffer per query type
//! for computing average and p95 latency, plus slow-query counting.
//! A global singleton is reset on each workspace change so stats
//! always reflect the current workspace baseline.

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

/// Maximum latency samples retained per query type.
const MAX_SAMPLES: usize = 1_000;

/// Queries at or above this threshold (ms) are counted as slow.
const SLOW_QUERY_MS: u64 = 100;

/// Per-query-type timing statistics.
#[derive(Debug, Default)]
pub struct QueryTypeStats {
    /// Circular window of recent latency samples (milliseconds).
    pub latencies_ms: Vec<u64>,
    /// Total queries recorded since last reset.
    pub total_count: u64,
    /// Queries whose elapsed time met or exceeded [`SLOW_QUERY_MS`].
    pub slow_count: u64,
}

/// Aggregated query timing statistics across all query types.
#[derive(Debug, Default)]
pub struct QueryTimingStats {
    /// Map of query-type label → per-type stats.
    pub by_type: HashMap<String, QueryTypeStats>,
}

impl QueryTimingStats {
    /// Creates an empty stats collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one query sample.
    pub fn record(&mut self, query_type: &str, elapsed_ms: u64) {
        let entry = self.by_type.entry(query_type.to_owned()).or_default();
        entry.total_count += 1;
        if elapsed_ms >= SLOW_QUERY_MS {
            entry.slow_count += 1;
        }
        if entry.latencies_ms.len() >= MAX_SAMPLES {
            entry.latencies_ms.remove(0);
        }
        entry.latencies_ms.push(elapsed_ms);
    }

    /// Clears all accumulated statistics.
    pub fn reset(&mut self) {
        self.by_type.clear();
    }

    /// Returns the arithmetic mean latency (ms) for a query type, or `None`
    /// if no samples exist.
    #[must_use]
    pub fn avg_latency_ms(&self, query_type: &str) -> Option<f64> {
        let entry = self.by_type.get(query_type)?;
        if entry.latencies_ms.is_empty() {
            return None;
        }
        let sum: u64 = entry.latencies_ms.iter().sum();
        #[allow(clippy::cast_precision_loss)]
        Some(sum as f64 / entry.latencies_ms.len() as f64)
    }

    /// Returns the p95 latency (ms) for a query type, or `None` if no
    /// samples exist.
    #[must_use]
    pub fn p95_latency_ms(&self, query_type: &str) -> Option<u64> {
        let entry = self.by_type.get(query_type)?;
        if entry.latencies_ms.is_empty() {
            return None;
        }
        let mut sorted = entry.latencies_ms.clone();
        sorted.sort_unstable();
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation
        )]
        let idx = (sorted.len() as f64 * 0.95).ceil() as usize;
        Some(sorted[idx.saturating_sub(1).min(sorted.len() - 1)])
    }

    /// Serializes the stats to a JSON-compatible structure suitable for
    /// embedding in the health report.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::json;
        let mut map = serde_json::Map::new();
        for (qt, entry) in &self.by_type {
            map.insert(
                qt.clone(),
                json!({
                    "total": entry.total_count,
                    "slow_count": entry.slow_count,
                    "avg_ms": self.avg_latency_ms(qt),
                    "p95_ms": self.p95_latency_ms(qt),
                }),
            );
        }
        serde_json::Value::Object(map)
    }
}

// ---------------------------------------------------------------------------
// Global singleton (process-wide, reset on workspace change)
// ---------------------------------------------------------------------------

static QUERY_TIMING: OnceLock<Mutex<QueryTimingStats>> = OnceLock::new();

fn global_stats() -> &'static Mutex<QueryTimingStats> {
    QUERY_TIMING.get_or_init(|| Mutex::new(QueryTimingStats::new()))
}

/// Records one query timing sample into the global stats.
pub fn record_timing(query_type: &str, elapsed_ms: u64) {
    if let Ok(mut stats) = global_stats().lock() {
        stats.record(query_type, elapsed_ms);
    }
}

/// Resets the global stats.  Call this on every workspace change so stats
/// reflect only the current workspace baseline.
pub fn reset_timing() {
    if let Ok(mut stats) = global_stats().lock() {
        stats.reset();
    }
}

/// Returns a JSON snapshot of the current global stats (zero-copy-friendly).
#[must_use]
pub fn timing_snapshot() -> serde_json::Value {
    global_stats()
        .lock()
        .map_or(serde_json::Value::Null, |s| s.to_json())
}
