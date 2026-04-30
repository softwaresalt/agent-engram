//! Integration tests for `CozoDB` edge CRUD and BFS graph traversal
//! (Tasks U3.2, U3.3, U3.4, U3.5).
//!
//! Covers:
//!  - `create_calls_edge`, `create_imports_edge`, `create_defines_edge`,
//!    `create_concerns_edge`, `create_inherits_edge`
//!  - `concerns_edge_exists`, `delete_concerns_edges_for_symbol`,
//!    `list_concerns_for_task`, `find_tasks_for_symbols`
//!  - `bfs_neighborhood`, `graph_neighborhood`, `count_code_edges`
//!
//! Requires the `cozo-backend` feature:
//!   `cargo test --no-default-features --features cozo-backend --test integration_cozo_edge`

use tempfile::TempDir;

use engram::models::Function;

async fn make_db() -> (TempDir, engram::db::Db) {
    let tmp = TempDir::new().expect("tempdir");
    let db = engram::db::connect_db(tmp.path(), "test-edge")
        .await
        .expect("connect_db must succeed");
    (tmp, db)
}

fn make_function(id: &str, file_path: &str) -> Function {
    let name = id.split(':').next_back().unwrap_or(id).to_owned();
    Function {
        id: id.to_owned(),
        name: name.clone(),
        file_path: file_path.to_owned(),
        line_start: 1,
        line_end: 10,
        signature: format!("fn {name}()"),
        docstring: None,
        body: String::new(),
        body_hash: format!("hash_{id}"),
        token_count: 0,
        embed_type: "explicit_code".to_owned(),
        summary: String::new(),
        embedding: vec![0.1_f32; 384],
    }
}

// ── calls_edge CRUD ──────────────────────────────────────────────────────────

#[tokio::test]
async fn create_calls_edge_succeeds() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_a", "src/a.rs"))
        .await
        .expect("upsert fn_a");
    q.upsert_function(&make_function("function:fn_b", "src/b.rs"))
        .await
        .expect("upsert fn_b");
    q.create_calls_edge("function:fn_a", "function:fn_b")
        .await
        .expect("create_calls_edge should succeed");
}

#[tokio::test]
async fn create_calls_edge_is_idempotent() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_idem_a", "src/a.rs"))
        .await
        .expect("upsert");
    q.upsert_function(&make_function("function:fn_idem_b", "src/b.rs"))
        .await
        .expect("upsert");
    q.create_calls_edge("function:fn_idem_a", "function:fn_idem_b")
        .await
        .expect("first create");
    q.create_calls_edge("function:fn_idem_a", "function:fn_idem_b")
        .await
        .expect("idempotent second create should not fail");
}

// ── defines_edge CRUD ────────────────────────────────────────────────────────

#[tokio::test]
async fn create_defines_edge_succeeds() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_x", "src/x.rs"))
        .await
        .expect("upsert");
    q.create_defines_edge("code_file:src_x.rs", "function", "function:fn_x")
        .await
        .expect("create_defines_edge should succeed");
}

// ── imports_edge CRUD ────────────────────────────────────────────────────────

#[tokio::test]
async fn create_imports_edge_succeeds() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.create_imports_edge(
        "code_file:src_a.rs",
        "code_file:src_b.rs",
        "crate::module_b",
    )
    .await
    .expect("create_imports_edge should succeed");
}

// ── concerns_edge CRUD ───────────────────────────────────────────────────────

#[tokio::test]
async fn create_concerns_edge_succeeds() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_y", "src/y.rs"))
        .await
        .expect("upsert");
    q.create_concerns_edge("task:t1", "function", "function:fn_y", "agent-x")
        .await
        .expect("create_concerns_edge should succeed");
}

#[tokio::test]
async fn concerns_edge_exists_after_create() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_z", "src/z.rs"))
        .await
        .expect("upsert");
    q.create_concerns_edge("task:t2", "function", "function:fn_z", "agent-x")
        .await
        .expect("create");
    let exists = q
        .concerns_edge_exists("task:t2", "function", "function:fn_z")
        .await
        .expect("concerns_edge_exists");
    assert!(exists, "edge should exist after create");
}

#[tokio::test]
async fn concerns_edge_absent_without_create() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let exists = q
        .concerns_edge_exists("task:nonexistent", "function", "function:missing")
        .await
        .expect("concerns_edge_exists on empty DB");
    assert!(!exists, "edge must not exist without prior create");
}

// ── delete_concerns_edges_for_symbol ─────────────────────────────────────────

#[tokio::test]
async fn delete_concerns_edges_for_symbol_returns_count() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_del", "src/del.rs"))
        .await
        .expect("upsert");
    q.create_concerns_edge("task:ta", "function", "function:fn_del", "agent")
        .await
        .expect("create a");
    q.create_concerns_edge("task:tb", "function", "function:fn_del", "agent")
        .await
        .expect("create b");

    let deleted = q
        .delete_concerns_edges_for_symbol("function", "function:fn_del")
        .await
        .expect("delete_concerns_edges_for_symbol");
    assert_eq!(deleted, 2, "should delete exactly 2 concerns edges");

    let after = q
        .concerns_edge_exists("task:ta", "function", "function:fn_del")
        .await
        .expect("exists check after delete");
    assert!(!after, "edge should be gone after deletion");
}

// ── list_concerns_for_task ────────────────────────────────────────────────────

#[tokio::test]
async fn list_concerns_for_task_returns_links() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_linked", "src/linked.rs"))
        .await
        .expect("upsert");
    q.create_concerns_edge("task:tq", "function", "function:fn_linked", "agent-link")
        .await
        .expect("create concerns edge");

    let links = q
        .list_concerns_for_task("task:tq")
        .await
        .expect("list_concerns_for_task");
    assert_eq!(links.len(), 1, "should return exactly one concern link");
    assert_eq!(links[0].symbol_id, "function:fn_linked");
    assert_eq!(links[0].linked_by, "agent-link");
}

// ── find_tasks_for_symbols ────────────────────────────────────────────────────

#[tokio::test]
async fn find_tasks_for_symbols_returns_pairs() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_task_sym", "src/ts.rs"))
        .await
        .expect("upsert");
    q.create_concerns_edge("task:task_x", "function", "function:fn_task_sym", "agent")
        .await
        .expect("create");

    let pairs = q
        .find_tasks_for_symbols(&["function:fn_task_sym".to_owned()])
        .await
        .expect("find_tasks_for_symbols");
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].0, "task:task_x");
    assert_eq!(pairs[0].1, "function:fn_task_sym");
}

// ── BFS traversal ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn bfs_single_hop_via_calls_edge() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_root", "src/root.rs"))
        .await
        .expect("upsert root");
    q.upsert_function(&make_function("function:fn_child", "src/child.rs"))
        .await
        .expect("upsert child");
    q.create_calls_edge("function:fn_root", "function:fn_child")
        .await
        .expect("edge");

    let result = q
        .bfs_neighborhood("function:fn_root", 1, 100)
        .await
        .expect("bfs_neighborhood");

    assert_eq!(
        result.neighbors.len(),
        1,
        "should find exactly one neighbor"
    );
    assert_eq!(result.neighbors[0].name, "fn_child");
    assert!(!result.truncated);
}

#[tokio::test]
async fn bfs_empty_graph_returns_empty_result() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);

    let result = q
        .bfs_neighborhood("function:nonexistent", 2, 100)
        .await
        .expect("bfs_neighborhood on empty graph");

    assert!(result.neighbors.is_empty(), "empty graph → no neighbors");
    assert!(result.edges.is_empty(), "empty graph → no edges");
    assert!(!result.truncated);
}

#[tokio::test]
async fn bfs_multi_hop_finds_transitive_neighbors() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_bfs_a", "src/a.rs"))
        .await
        .expect("upsert a");
    q.upsert_function(&make_function("function:fn_bfs_b", "src/b.rs"))
        .await
        .expect("upsert b");
    q.upsert_function(&make_function("function:fn_bfs_c", "src/c.rs"))
        .await
        .expect("upsert c");
    q.create_calls_edge("function:fn_bfs_a", "function:fn_bfs_b")
        .await
        .expect("edge a→b");
    q.create_calls_edge("function:fn_bfs_b", "function:fn_bfs_c")
        .await
        .expect("edge b→c");

    let result = q
        .bfs_neighborhood("function:fn_bfs_a", 2, 100)
        .await
        .expect("bfs_neighborhood depth=2");

    let names: std::collections::HashSet<&str> =
        result.neighbors.iter().map(|n| n.name.as_str()).collect();
    assert!(names.contains("fn_bfs_b"), "depth=2 must find fn_bfs_b");
    assert!(names.contains("fn_bfs_c"), "depth=2 must find fn_bfs_c");
}

// ── graph_neighborhood ────────────────────────────────────────────────────────

#[tokio::test]
async fn graph_neighborhood_matches_bfs_for_calls_edge() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_gn_a", "src/a.rs"))
        .await
        .expect("upsert");
    q.upsert_function(&make_function("function:fn_gn_b", "src/b.rs"))
        .await
        .expect("upsert");
    q.create_calls_edge("function:fn_gn_a", "function:fn_gn_b")
        .await
        .expect("edge");

    let bfs = q
        .bfs_neighborhood("function:fn_gn_a", 1, 100)
        .await
        .expect("bfs");
    let gn = q
        .graph_neighborhood("function:fn_gn_a", 1, 100)
        .await
        .expect("gn");

    let bfs_names: std::collections::HashSet<&str> =
        bfs.neighbors.iter().map(|n| n.name.as_str()).collect();
    let gn_names: std::collections::HashSet<&str> =
        gn.neighbors.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(
        bfs_names, gn_names,
        "bfs_neighborhood and graph_neighborhood must agree on neighbors"
    );
}

// ── count_code_edges ──────────────────────────────────────────────────────────

#[tokio::test]
async fn count_code_edges_increases_after_create() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_function("function:fn_cnt_a", "src/a.rs"))
        .await
        .expect("upsert a");
    q.upsert_function(&make_function("function:fn_cnt_b", "src/b.rs"))
        .await
        .expect("upsert b");

    let before = q.count_code_edges().await.expect("count before");
    q.create_calls_edge("function:fn_cnt_a", "function:fn_cnt_b")
        .await
        .expect("create edge");
    let after = q.count_code_edges().await.expect("count after");

    assert!(
        after > before,
        "edge count should increase from {before} after creating an edge, got {after}"
    );
}
