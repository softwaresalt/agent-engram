//! Integration tests for query performance observability (dxo.5.3).
//!
//! Validates that query timing stats are recorded when queries execute,
//! that stats reset on demand, and that the health-report snapshot has
//! the expected JSON shape.
//!
//! All tests that touch the global timing singleton are serialized via
//! `TIMING_LOCK` to prevent cross-test state pollution.

use engram::{
    db::connect_db, db::queries::CodeGraphQueries, models::Function, services::query_stats,
};

/// Serializes tests that read/write the global timing singleton.
/// Uses `tokio::sync::Mutex` so the guard can be held across `.await` points.
static TIMING_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

// ── helpers ───────────────────────────────────────────────────────────────

async fn test_db(label: &str) -> CodeGraphQueries {
    let hash = format!("test_perf_obs_{label}_{}", std::process::id());
    let db = connect_db(&hash).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

async fn insert_fn(q: &CodeGraphQueries, name: &str) {
    let func = Function {
        id: format!("function:{name}"),
        name: name.to_string(),
        file_path: "src/lib.rs".to_string(),
        line_start: 1,
        line_end: 5,
        signature: format!("fn {name}()"),
        docstring: None,
        body: String::new(),
        body_hash: format!("hash_{name}"),
        token_count: 0,
        embed_type: "explicit_code".to_string(),
        summary: format!("{name} summary"),
        embedding: vec![0.0_f32; 384],
    };
    q.upsert_function(&func)
        .await
        .unwrap_or_else(|e| panic!("upsert_function({name}): {e}"));
}

fn dim_vec(dim: usize) -> Vec<f32> {
    let mut v = vec![0.0_f32; 384];
    v[dim] = 1.0;
    v
}

// ── tests ─────────────────────────────────────────────────────────────────

/// After a `graph_neighborhood` query, the `"graph_traversal"` timing bucket
/// must have at least one recorded sample.
#[tokio::test]
async fn graph_traversal_query_records_timing_stat() {
    let _lock = TIMING_LOCK.lock().await;

    // GIVEN a minimal graph with two nodes
    let q = test_db("gt_timing").await;
    insert_fn(&q, "root_gt").await;
    insert_fn(&q, "child_gt").await;
    q.create_calls_edge("function:root_gt", "function:child_gt")
        .await
        .expect("calls edge");

    query_stats::reset_timing();

    // WHEN graph_neighborhood executes
    let _result = q
        .graph_neighborhood("function:root_gt", 2, 100)
        .await
        .expect("graph_neighborhood");

    // THEN "graph_traversal" bucket has at least one sample
    let snap = query_stats::timing_snapshot();
    let obj = snap
        .as_object()
        .expect("timing_snapshot must be JSON object");
    let gt = obj
        .get("graph_traversal")
        .expect("graph_traversal bucket must exist after graph_neighborhood");

    assert!(
        gt["total"].as_u64().unwrap_or(0) >= 1,
        "graph_traversal total must be >= 1, got: {gt}"
    );
}

/// After a `hybrid_graph_vector_search` call, the `"hybrid_search"` bucket
/// must be populated.
#[tokio::test]
async fn hybrid_search_query_records_timing_stat() {
    let _lock = TIMING_LOCK.lock().await;

    // GIVEN a two-node graph with embeddings
    let q = test_db("hs_timing").await;

    let func_root = Function {
        id: "function:root_hs".to_string(),
        name: "root_hs".to_string(),
        file_path: "src/lib.rs".to_string(),
        line_start: 1,
        line_end: 5,
        signature: "fn root_hs()".to_string(),
        docstring: None,
        body: String::new(),
        body_hash: "hash_root_hs".to_string(),
        token_count: 0,
        embed_type: "explicit_code".to_string(),
        summary: "root_hs summary".to_string(),
        embedding: dim_vec(0),
    };
    let func_child = Function {
        id: "function:child_hs".to_string(),
        name: "child_hs".to_string(),
        file_path: "src/lib.rs".to_string(),
        line_start: 6,
        line_end: 10,
        signature: "fn child_hs()".to_string(),
        docstring: None,
        body: String::new(),
        body_hash: "hash_child_hs".to_string(),
        token_count: 0,
        embed_type: "explicit_code".to_string(),
        summary: "child_hs summary".to_string(),
        embedding: dim_vec(0),
    };
    q.upsert_function(&func_root).await.expect("upsert root");
    q.upsert_function(&func_child).await.expect("upsert child");
    q.create_calls_edge("function:root_hs", "function:child_hs")
        .await
        .expect("calls edge");

    query_stats::reset_timing();

    // WHEN hybrid_graph_vector_search executes
    let _results = q
        .hybrid_graph_vector_search("function:root_hs", 1, &dim_vec(0), 10, &[])
        .await
        .expect("hybrid_graph_vector_search");

    // THEN "hybrid_search" bucket is populated
    let snap = query_stats::timing_snapshot();
    let obj = snap.as_object().expect("JSON object");
    let hs = obj
        .get("hybrid_search")
        .expect("hybrid_search bucket must exist");

    assert!(
        hs["total"].as_u64().unwrap_or(0) >= 1,
        "hybrid_search total must be >= 1, got: {hs}"
    );
}

/// `reset_timing` must clear all accumulated stats.
#[tokio::test]
async fn reset_timing_clears_all_accumulated_stats() {
    let _lock = TIMING_LOCK.lock().await;

    // GIVEN some stats exist from direct record_timing calls
    query_stats::record_timing("graph_traversal", 25);
    query_stats::record_timing("knn_search", 40);

    let before = query_stats::timing_snapshot();
    assert!(
        before.as_object().is_some_and(|o| !o.is_empty()),
        "expected stats to be non-empty before reset"
    );

    // WHEN we reset
    query_stats::reset_timing();

    // THEN snapshot is empty
    let after = query_stats::timing_snapshot();
    let obj = after.as_object().expect("JSON object");
    assert!(obj.is_empty(), "stats must be empty after reset_timing()");
}

/// Queries whose recorded elapsed time is `>= 100 ms` must increment `slow_count`.
#[tokio::test]
async fn slow_query_threshold_increments_slow_count_in_snapshot() {
    let _lock = TIMING_LOCK.lock().await;
    query_stats::reset_timing();

    // WHEN we record three samples directly (two above threshold)
    query_stats::record_timing("graph_traversal", 50); // fast
    query_stats::record_timing("graph_traversal", 100); // exactly at threshold → slow
    query_stats::record_timing("graph_traversal", 150); // above threshold → slow

    // THEN slow_count == 2, total == 3
    let snap = query_stats::timing_snapshot();
    let obj = snap.as_object().expect("JSON object");
    let gt = obj
        .get("graph_traversal")
        .expect("graph_traversal bucket must exist");

    assert_eq!(gt["total"].as_u64(), Some(3), "total must be 3, got: {gt}");
    assert_eq!(
        gt["slow_count"].as_u64(),
        Some(2),
        "slow_count must be 2, got: {gt}"
    );
}

/// `timing_snapshot` must return a JSON object where every entry has the
/// required numeric fields for health-report consumers.
#[tokio::test]
async fn timing_snapshot_has_required_json_shape() {
    let _lock = TIMING_LOCK.lock().await;
    query_stats::reset_timing();
    query_stats::record_timing("symbol_lookup", 10);
    query_stats::record_timing("symbol_lookup", 20);

    let snap = query_stats::timing_snapshot();
    let obj = snap
        .as_object()
        .expect("timing_snapshot must be JSON object");

    for (qt, stats) in obj {
        assert!(
            stats.is_object(),
            "entry for '{qt}' must be a JSON object, got: {stats}"
        );
        assert!(
            stats["total"].is_number(),
            "'{qt}'.total must be numeric, got: {stats}"
        );
        assert!(
            stats["slow_count"].is_number(),
            "'{qt}'.slow_count must be numeric, got: {stats}"
        );
        // avg_ms may be null when there are no samples, but must be present
        assert!(
            stats.get("avg_ms").is_some(),
            "'{qt}'.avg_ms key must be present, got: {stats}"
        );
        assert!(
            stats.get("p95_ms").is_some(),
            "'{qt}'.p95_ms key must be present, got: {stats}"
        );
    }
}

/// A CRUD upsert (`find_symbols_by_name` or `upsert_function`) populates the
/// `"crud"` or `"symbol_lookup"` timing bucket.
#[tokio::test]
async fn symbol_lookup_query_records_timing_stat() {
    let _lock = TIMING_LOCK.lock().await;

    let q = test_db("sym_timing").await;
    insert_fn(&q, "lookup_target").await;

    query_stats::reset_timing();

    // WHEN find_symbols_by_name runs
    let _results = q
        .find_symbols_by_name("lookup_target")
        .await
        .expect("find_symbols_by_name");

    // THEN "symbol_lookup" bucket is populated
    let snap = query_stats::timing_snapshot();
    let obj = snap.as_object().expect("JSON object");
    let sl = obj
        .get("symbol_lookup")
        .expect("symbol_lookup bucket must exist after find_symbols_by_name");

    assert!(
        sl["total"].as_u64().unwrap_or(0) >= 1,
        "symbol_lookup total must be >= 1, got: {sl}"
    );
}
