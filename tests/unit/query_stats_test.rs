use engram::services::query_stats::QueryTimingStats;

#[test]
fn record_increments_total_count_per_type() {
    let mut stats = QueryTimingStats::new();
    stats.record("graph_traversal", 50);
    stats.record("graph_traversal", 75);
    stats.record("knn_search", 30);

    let gt = stats.by_type.get("graph_traversal").unwrap();
    assert_eq!(gt.total_count, 2);

    let knn = stats.by_type.get("knn_search").unwrap();
    assert_eq!(knn.total_count, 1);
}

#[test]
fn slow_query_at_100ms_threshold_increments_slow_count() {
    let mut stats = QueryTimingStats::new();
    stats.record("knn_search", 99); // not slow
    stats.record("knn_search", 100); // exactly at threshold — slow
    stats.record("knn_search", 150); // slow

    let entry = stats.by_type.get("knn_search").unwrap();
    assert_eq!(entry.slow_count, 2);
}

#[test]
fn p95_latency_returns_correct_percentile() {
    let mut stats = QueryTimingStats::new();
    // 10 samples: 9 at 10ms, 1 at 100ms
    // p95 index for 10 samples: ceil(0.95 * 10) - 1 = 10 - 1 = 9
    // sorted[9] = 100ms
    for _ in 0..9 {
        stats.record("knn_search", 10);
    }
    stats.record("knn_search", 100);

    let p95 = stats.p95_latency_ms("knn_search").unwrap();
    assert_eq!(p95, 100);
}

#[test]
fn avg_latency_computed_correctly() {
    let mut stats = QueryTimingStats::new();
    stats.record("crud", 10);
    stats.record("crud", 20);
    stats.record("crud", 30);

    let avg = stats.avg_latency_ms("crud").unwrap();
    assert!((avg - 20.0).abs() < 0.001, "expected avg=20.0, got {avg}");
}

#[test]
fn reset_clears_all_entries() {
    let mut stats = QueryTimingStats::new();
    stats.record("graph_traversal", 50);
    stats.record("knn_search", 30);
    stats.reset();

    assert!(stats.by_type.is_empty());
}

#[test]
fn p95_returns_none_for_unknown_query_type() {
    let stats = QueryTimingStats::new();
    assert!(stats.p95_latency_ms("nonexistent").is_none());
}

#[test]
fn avg_returns_none_for_unknown_query_type() {
    let stats = QueryTimingStats::new();
    assert!(stats.avg_latency_ms("nonexistent").is_none());
}
