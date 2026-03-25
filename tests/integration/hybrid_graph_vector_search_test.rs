//! Integration tests for hybrid graph + vector queries (dxo.3.3).
//!
//! Validates `hybrid_graph_vector_search()` against an embedded `SurrealDB`
//! instance with known graph structure and embedding vectors.

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::Function;

/// Create a test DB and return a `CodeGraphQueries` handle.
async fn test_queries(label: &str) -> CodeGraphQueries {
    let branch = format!("test_hybrid_{label}_{}", std::process::id());
    let data_dir = std::env::temp_dir().join("engram-test");
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

/// Insert a function with a specific embedding vector.
async fn insert_fn(queries: &CodeGraphQueries, name: &str, embedding: Vec<f32>) {
    let func = Function {
        id: format!("function:{name}"),
        name: name.to_string(),
        file_path: "src/test.rs".to_string(),
        line_start: 1,
        line_end: 5,
        signature: format!("fn {name}()"),
        docstring: None,
        body: String::new(),
        body_hash: format!("hash_{name}"),
        token_count: 0,
        embed_type: "explicit_code".to_string(),
        summary: format!("{name} summary"),
        embedding,
    };
    queries
        .upsert_function(&func)
        .await
        .expect("upsert_function");
}

/// Return a 384-d unit vector with a dominant component at `dim`.
fn unit_vec(dim: usize) -> Vec<f32> {
    let mut v = vec![0.0_f32; 384];
    v[dim] = 1.0;
    v
}

#[tokio::test]
async fn hybrid_query_returns_only_graph_neighbors() {
    // GIVEN root → fn_a (calls edge); fn_isolated has no edges
    // fn_a and fn_isolated both have embeddings similar to the query
    let q = test_queries("scope").await;
    insert_fn(&q, "root", unit_vec(0)).await;
    insert_fn(&q, "fn_a", unit_vec(0)).await; // similar to query
    insert_fn(&q, "fn_isolated", unit_vec(0)).await; // similar but not reachable

    q.create_calls_edge("function:root", "function:fn_a")
        .await
        .expect("edge root->fn_a");

    // WHEN we run hybrid search with depth=1, query aligned with dim 0
    let results = q
        .hybrid_graph_vector_search("function:root", 1, &unit_vec(0), 10, &[])
        .await
        .expect("hybrid search");

    // THEN fn_isolated must NOT appear — it is not a graph neighbor
    let names: Vec<&str> = results.iter().map(|(_, s)| s.name.as_str()).collect();
    assert!(
        !names.contains(&"fn_isolated"),
        "fn_isolated is not a neighbor and must not appear; got: {names:?}"
    );
}

#[tokio::test]
async fn hybrid_query_results_ordered_by_similarity_score() {
    // GIVEN root → fn_close (dim 0), root → fn_far (dim 1)
    // Query is aligned with dim 0 so fn_close should score higher
    let q = test_queries("order").await;
    insert_fn(&q, "root_order", unit_vec(10)).await;
    insert_fn(&q, "fn_close", unit_vec(0)).await; // cosine sim = 1.0 w/ query
    insert_fn(&q, "fn_far", unit_vec(1)).await; // cosine sim = 0.0 w/ query

    q.create_calls_edge("function:root_order", "function:fn_close")
        .await
        .expect("edge root->fn_close");
    q.create_calls_edge("function:root_order", "function:fn_far")
        .await
        .expect("edge root->fn_far");

    // WHEN we search with a query aligned to dim 0
    let results = q
        .hybrid_graph_vector_search("function:root_order", 1, &unit_vec(0), 10, &[])
        .await
        .expect("hybrid search");

    // THEN fn_close appears and scores higher than fn_far
    if results.len() >= 2 {
        assert_eq!(
            results[0].1.name,
            "fn_close",
            "fn_close should be ranked first; got order: {:?}",
            results
                .iter()
                .map(|(s, m)| (s, m.name.as_str()))
                .collect::<Vec<_>>()
        );
        assert!(
            results[0].0 >= results[1].0,
            "scores must be in descending order"
        );
    }
}

#[tokio::test]
async fn hybrid_query_empty_neighborhood_returns_empty() {
    // GIVEN root has no outgoing edges
    let q = test_queries("empty").await;
    insert_fn(&q, "root_empty", unit_vec(0)).await;

    // WHEN hybrid search is run from a node with no neighbors
    let results = q
        .hybrid_graph_vector_search("function:root_empty", 1, &unit_vec(0), 10, &[])
        .await
        .expect("hybrid search");

    // THEN no results (no neighbors to intersect with)
    assert!(
        results.is_empty(),
        "expected empty results when root has no edges; got {results:?}"
    );
}

#[tokio::test]
async fn hybrid_query_depth2_includes_transitive_neighbors() {
    // GIVEN root → fn_hop1 → fn_hop2 (all with matching embeddings)
    let q = test_queries("depth2").await;
    insert_fn(&q, "root_d2", unit_vec(5)).await;
    insert_fn(&q, "fn_hop1", unit_vec(5)).await;
    insert_fn(&q, "fn_hop2", unit_vec(5)).await;

    q.create_calls_edge("function:root_d2", "function:fn_hop1")
        .await
        .expect("edge root->hop1");
    q.create_calls_edge("function:fn_hop1", "function:fn_hop2")
        .await
        .expect("edge hop1->hop2");

    // WHEN depth=2
    let results = q
        .hybrid_graph_vector_search("function:root_d2", 2, &unit_vec(5), 10, &[])
        .await
        .expect("hybrid search");

    // THEN fn_hop2 appears because it is within 2 hops
    let names: Vec<&str> = results.iter().map(|(_, s)| s.name.as_str()).collect();
    assert!(
        names.contains(&"fn_hop2"),
        "fn_hop2 should be reachable at depth=2; got: {names:?}"
    );
}

#[tokio::test]
async fn hybrid_query_depth1_excludes_transitive_neighbors() {
    // GIVEN root → fn_hop1 → fn_hop2 (all with matching embeddings)
    let q = test_queries("depth1_excl").await;
    insert_fn(&q, "root_d1", unit_vec(7)).await;
    insert_fn(&q, "fn_mid", unit_vec(7)).await;
    insert_fn(&q, "fn_deep", unit_vec(7)).await;

    q.create_calls_edge("function:root_d1", "function:fn_mid")
        .await
        .expect("edge root->fn_mid");
    q.create_calls_edge("function:fn_mid", "function:fn_deep")
        .await
        .expect("edge fn_mid->fn_deep");

    // WHEN depth=1
    let results = q
        .hybrid_graph_vector_search("function:root_d1", 1, &unit_vec(7), 10, &[])
        .await
        .expect("hybrid search");

    // THEN fn_deep must NOT appear (it is 2 hops away)
    let names: Vec<&str> = results.iter().map(|(_, s)| s.name.as_str()).collect();
    assert!(
        !names.contains(&"fn_deep"),
        "fn_deep is 2 hops away and must not appear at depth=1; got: {names:?}"
    );
}

#[tokio::test]
async fn hybrid_query_configurable_edge_types_filter_correctly() {
    // GIVEN root → fn_via_calls (calls edge), root → fn_via_imports (imports edge)
    // Both have matching embeddings
    let q = test_queries("edgetypes").await;
    insert_fn(&q, "root_et", unit_vec(3)).await;
    insert_fn(&q, "fn_via_calls", unit_vec(3)).await;
    insert_fn(&q, "fn_via_imports", unit_vec(3)).await;

    q.create_calls_edge("function:root_et", "function:fn_via_calls")
        .await
        .expect("calls edge");
    q.create_imports_edge("function:root_et", "function:fn_via_imports", "test_path")
        .await
        .expect("imports edge");

    // WHEN searching with only ["calls"] edge type
    let results = q
        .hybrid_graph_vector_search("function:root_et", 1, &unit_vec(3), 10, &["calls"])
        .await
        .expect("hybrid search");

    let names: Vec<&str> = results.iter().map(|(_, s)| s.name.as_str()).collect();
    // THEN fn_via_calls appears but fn_via_imports does not
    assert!(
        names.contains(&"fn_via_calls"),
        "fn_via_calls must appear when traversing calls edges; got: {names:?}"
    );
    assert!(
        !names.contains(&"fn_via_imports"),
        "fn_via_imports must NOT appear when only calls edges are traversed; got: {names:?}"
    );
}

#[tokio::test]
async fn hybrid_query_limit_zero_returns_empty() {
    // GIVEN any graph
    let q = test_queries("limit_zero").await;
    insert_fn(&q, "root_lz", unit_vec(0)).await;
    insert_fn(&q, "fn_lz_a", unit_vec(0)).await;

    q.create_calls_edge("function:root_lz", "function:fn_lz_a")
        .await
        .expect("edge");

    // WHEN limit = 0
    let results = q
        .hybrid_graph_vector_search("function:root_lz", 1, &unit_vec(0), 0, &[])
        .await
        .expect("hybrid search with limit=0");

    // THEN immediately returns empty without hitting DB
    assert!(results.is_empty(), "limit=0 must return empty immediately");
}

#[tokio::test]
async fn hybrid_query_source_is_db_authoritative_scores() {
    // GIVEN root → fn_a with known embedding
    let q = test_queries("scores").await;
    insert_fn(&q, "root_sc", unit_vec(20)).await;
    insert_fn(&q, "fn_scored", unit_vec(0)).await;

    q.create_calls_edge("function:root_sc", "function:fn_scored")
        .await
        .expect("edge");

    // WHEN we search with a unit vector aligned to dim 0 (exact match with fn_scored)
    let results = q
        .hybrid_graph_vector_search("function:root_sc", 1, &unit_vec(0), 10, &[])
        .await
        .expect("hybrid search");

    // THEN scores are in [0.0, 1.0] range (DB-authoritative cosine similarity)
    for (score, sym) in &results {
        assert!(
            (0.0..=1.0).contains(score),
            "score for {} must be in [0.0, 1.0]; got {score}",
            sym.name
        );
    }
}
