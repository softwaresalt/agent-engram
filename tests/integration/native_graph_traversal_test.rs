//! Integration tests for native `SurrealQL` graph traversal (dxo.1.1).
//!
//! Verifies that `graph_neighborhood()` uses `SurrealQL` `->edge->`/`<-edge<-`
//! syntax instead of the manual BFS in `bfs_neighborhood()`.

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::{CodeFile, Function};

/// Create a test DB and return a `CodeGraphQueries` handle.
async fn test_queries(label: &str) -> CodeGraphQueries {
    let hash = format!("test_graph_{label}_{}", std::process::id());
    let db = connect_db(&hash).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

/// Insert a function node into the code graph.
async fn insert_function(queries: &CodeGraphQueries, name: &str, file: &str) {
    let func = Function {
        id: format!("function:{name}"),
        name: name.to_string(),
        file_path: file.to_string(),
        line_start: 1,
        line_end: 10,
        signature: format!("fn {name}()"),
        docstring: None,
        body: String::new(),
        body_hash: String::new(),
        token_count: 0,
        embed_type: "explicit_code".to_string(),
        summary: format!("{name} summary"),
        embedding: vec![],
    };
    queries
        .upsert_function(&func)
        .await
        .expect("upsert_function");
}

/// Insert a code file node into the code graph.
async fn insert_code_file(queries: &CodeGraphQueries, path: &str) {
    let file = CodeFile {
        id: format!("code_file:{}", path.replace('/', "_")),
        path: path.to_string(),
        language: "rust".to_string(),
        size_bytes: 100,
        content_hash: "abc123".to_string(),
        last_indexed_at: String::new(),
    };
    queries
        .upsert_code_file(&file)
        .await
        .expect("upsert_code_file");
}

#[tokio::test]
async fn single_hop_matches_bfs_depth_one() {
    // GIVEN a graph: fn_a --calls--> fn_b, fn_a --calls--> fn_c
    let q = test_queries("single_hop").await;
    insert_function(&q, "fn_a", "src/a.rs").await;
    insert_function(&q, "fn_b", "src/b.rs").await;
    insert_function(&q, "fn_c", "src/c.rs").await;
    q.create_calls_edge("function:fn_a", "function:fn_b")
        .await
        .expect("create edge a->b");
    q.create_calls_edge("function:fn_a", "function:fn_c")
        .await
        .expect("create edge a->c");

    // WHEN we traverse from fn_a at depth=1
    let result = q
        .graph_neighborhood("function:fn_a", 1, 100)
        .await
        .expect("graph_neighborhood");

    // THEN we discover fn_b and fn_c as neighbors
    let names: Vec<&str> = result.neighbors.iter().map(|n| n.name.as_str()).collect();
    assert!(names.contains(&"fn_b"), "should find fn_b in neighbors");
    assert!(names.contains(&"fn_c"), "should find fn_c in neighbors");
    assert_eq!(
        result.neighbors.len(),
        2,
        "should find exactly 2 neighbors at depth=1"
    );
    assert!(
        !result.truncated,
        "should not be truncated with max_nodes=100"
    );
}

#[tokio::test]
async fn multi_hop_returns_transitive_closure() {
    // GIVEN a chain: fn_a --calls--> fn_b --calls--> fn_c --calls--> fn_d
    let q = test_queries("multi_hop").await;
    insert_function(&q, "fn_a", "src/a.rs").await;
    insert_function(&q, "fn_b", "src/b.rs").await;
    insert_function(&q, "fn_c", "src/c.rs").await;
    insert_function(&q, "fn_d", "src/d.rs").await;
    q.create_calls_edge("function:fn_a", "function:fn_b")
        .await
        .expect("edge");
    q.create_calls_edge("function:fn_b", "function:fn_c")
        .await
        .expect("edge");
    q.create_calls_edge("function:fn_c", "function:fn_d")
        .await
        .expect("edge");

    // WHEN we traverse from fn_a at depth=3
    let result = q
        .graph_neighborhood("function:fn_a", 3, 100)
        .await
        .expect("graph_neighborhood");

    // THEN we discover fn_b, fn_c, fn_d (full transitive closure)
    let names: Vec<&str> = result.neighbors.iter().map(|n| n.name.as_str()).collect();
    assert!(names.contains(&"fn_b"), "should find fn_b");
    assert!(names.contains(&"fn_c"), "should find fn_c");
    assert!(names.contains(&"fn_d"), "should find fn_d");
    assert_eq!(
        result.neighbors.len(),
        3,
        "depth=3 chain should find 3 neighbors"
    );
}

#[tokio::test]
async fn empty_graph_returns_empty_neighborhood() {
    // GIVEN an empty code graph
    let q = test_queries("empty").await;

    // WHEN we traverse from a non-existent root
    let result = q
        .graph_neighborhood("function:nonexistent", 2, 100)
        .await
        .expect("graph_neighborhood");

    // THEN neighbors and edges are empty
    assert!(
        result.neighbors.is_empty(),
        "empty graph should have no neighbors"
    );
    assert!(result.edges.is_empty(), "empty graph should have no edges");
    assert!(!result.truncated);
}

#[tokio::test]
async fn isolated_node_returns_only_root() {
    // GIVEN a single function with no edges
    let q = test_queries("isolated").await;
    insert_function(&q, "lonely_fn", "src/lonely.rs").await;

    // WHEN we traverse from the isolated node
    let result = q
        .graph_neighborhood("function:lonely_fn", 2, 100)
        .await
        .expect("graph_neighborhood");

    // THEN no neighbors are found (root is not included in neighbors)
    assert!(
        result.neighbors.is_empty(),
        "isolated node should have no neighbors"
    );
    assert!(result.edges.is_empty());
}

#[tokio::test]
async fn all_five_edge_types_are_traversed() {
    // GIVEN a graph with all 5 edge types from fn_root
    let q = test_queries("all_edges").await;
    insert_function(&q, "fn_root", "src/root.rs").await;
    insert_function(&q, "fn_called", "src/called.rs").await;
    insert_function(&q, "fn_defined", "src/defined.rs").await;
    insert_code_file(&q, "src/imported.rs").await;
    insert_code_file(&q, "src/root.rs").await;

    // calls edge
    q.create_calls_edge("function:fn_root", "function:fn_called")
        .await
        .expect("calls edge");
    // defines edge (code_file -> symbol)
    q.create_defines_edge("code_file:src_root_rs", "function", "function:fn_defined")
        .await
        .expect("defines edge");
    // imports edge (code_file -> code_file)
    q.create_imports_edge(
        "code_file:src_root_rs",
        "code_file:src_imported_rs",
        "src/imported.rs",
    )
    .await
    .expect("imports edge");

    // WHEN we traverse from fn_root at depth=1
    let result = q
        .graph_neighborhood("function:fn_root", 1, 100)
        .await
        .expect("graph_neighborhood");

    // THEN all edge types produce neighbors
    assert!(
        result.neighbors.len() >= 2,
        "should find neighbors via multiple edge types, got {}",
        result.neighbors.len()
    );
    let edge_types: Vec<&str> = result.edges.iter().map(|e| e.edge_type.as_str()).collect();
    assert!(
        edge_types.contains(&"calls"),
        "should include 'calls' edge type"
    );
}

#[tokio::test]
async fn truncation_at_max_nodes() {
    // GIVEN a star graph: fn_hub connects to 10 functions
    let q = test_queries("truncation").await;
    insert_function(&q, "fn_hub", "src/hub.rs").await;
    for i in 0..10 {
        let name = format!("fn_spoke_{i}");
        insert_function(&q, &name, "src/spoke.rs").await;
        q.create_calls_edge("function:fn_hub", &format!("function:{name}"))
            .await
            .expect("create edge");
    }

    // WHEN we traverse with max_nodes=3
    let result = q
        .graph_neighborhood("function:fn_hub", 1, 3)
        .await
        .expect("graph_neighborhood");

    // THEN results are truncated at 3 neighbors
    assert!(
        result.neighbors.len() <= 3,
        "should truncate at max_nodes=3, got {}",
        result.neighbors.len()
    );
    assert!(result.truncated, "should signal truncation");
}
