//! Contract tests for the `references` relation edge — 033.004-T.
//!
//! Verifies that `create_references_edge` creates a persisted edge in the
//! SurrealDB `references` table and can be queried back.
//!
//! # Harness status
//!
//! These tests are in the **red phase** until 033.004-T is implemented:
//! `create_references_edge` currently panics with `unimplemented!()`.

use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;

/// Derive test DB parameters from a workspace path.
fn test_db_params(path: &Path) -> (std::path::PathBuf, String) {
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("ref-edge-{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

/// 033.004-T: `create_references_edge` must persist a resolved (file → class) edge.
///
/// After implementation, creates an edge from a code_file node to a class node
/// and verifies it can be queried from the `references` table.
///
/// **Harness**: panics with `unimplemented!` until 033.004-T is complete.
#[test]
async fn contract_create_references_edge_resolved() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (data_dir, branch) = test_db_params(tmp.path());
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db.clone());

    // Stub: panics until 033.004-T implements the method.
    q.create_references_edge("code_file:abc", "class:def", Some("users"))
        .await
        .expect("create_references_edge must succeed for resolved target (033.004-T)");

    // Verify the edge persists in the `references` table.
    // SELECT specific fields to avoid `id: Thing` which serde_json::Value cannot deserialize
    // in SurrealDB 2.6 (Thing uses an Id enum that serde_content presents as visit_enum).
    let mut resp = db
        .query("SELECT source, target, qualified_name FROM `references` LIMIT 10")
        .await
        .expect("query references table");
    let rows: Vec<serde_json::Value> = resp.take(0).expect("deserialize");
    assert!(
        !rows.is_empty(),
        "references table must contain at least one edge after create_references_edge"
    );
}

/// 033.004-T: `create_references_edge` must persist an unresolved (file → file) edge.
///
/// When a SQL reference target cannot be resolved to a Class node, the edge
/// should point back to the source file with `qualified_name` set to the raw
/// target string.
///
/// **Harness**: panics with `unimplemented!` until 033.004-T is complete.
#[test]
async fn contract_create_references_edge_unresolved() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (data_dir, branch) = test_db_params(tmp.path());
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db.clone());

    // Stub: panics until 033.004-T implements the method.
    q.create_references_edge("code_file:abc", "code_file:abc", Some("nonexistent_table"))
        .await
        .expect("create_references_edge must succeed for unresolved target (033.004-T)");

    // After implementation: verify qualified_name is stored on the edge.
    let mut resp = db
        .query("SELECT qualified_name FROM `references` LIMIT 10")
        .await
        .expect("query references table");
    let rows: Vec<serde_json::Value> = resp.take(0).expect("deserialize");
    assert!(
        !rows.is_empty(),
        "references table must have the unresolved edge"
    );
    let qn = rows[0].get("qualified_name");
    assert!(
        qn.map_or(false, |v| v.as_str() == Some("nonexistent_table")),
        "qualified_name must be 'nonexistent_table'; got: {qn:?}"
    );
}

/// 033.004-T: `delete_edges_from_file` must clean up `references` edges.
///
/// Verifies that calling `delete_edges_from_file("references", file_id)` removes
/// all `references` edges originating from that file.
///
/// **Harness**: panics with `unimplemented!` until 033.004-T is complete.
#[test]
async fn contract_delete_references_edges_from_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (data_dir, branch) = test_db_params(tmp.path());
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db.clone());

    // Create an edge first (panics until 033.004-T is implemented).
    q.create_references_edge("code_file:file1", "class:users", Some("users"))
        .await
        .expect("setup: create_references_edge");

    // Delete edges for file1.
    q.delete_edges_from_file("references", "code_file:file1")
        .await
        .expect("delete_edges_from_file must succeed for 'references'");

    // After deletion, the edge must not be present.
    let mut resp = db
        .query("SELECT * FROM `references` LIMIT 10")
        .await
        .expect("query references table");
    let rows: Vec<serde_json::Value> = resp.take(0).expect("deserialize");
    assert!(
        rows.is_empty(),
        "references table must be empty after delete_edges_from_file"
    );
}
