//! Integration tests for SQL References edge graph wiring — 033.001-T.
//!
//! Indexes temporary SQL workspaces via `code_graph::index_workspace` and
//! verifies that `references` edges are correctly persisted in SurrealDB for:
//!
//! * Resolved edges: SQL file references a class that was indexed from a CREATE TABLE
//! * Unresolved edges: SQL file references a table that has no corresponding Class node
//! * Qualified-name fallback: `public.users` resolves via last-segment match to `users`
//!
//! # Harness status
//!
//! These tests are in the **red phase** until 033.001-T is implemented.
//! The `code_graph::index_workspace` path currently has a no-op arm for
//! `ExtractedEdge::References { .. }`, so no edges are written and the
//! assertions below fail with "no references edges found".

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::test;

use engram::db::connect_db;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;

/// Helper: write a file into a workspace directory, creating parents as needed.
fn write_file(dir: &Path, rel: &str, content: &str) {
    let full = dir.join(rel);
    if let Some(p) = full.parent() {
        fs::create_dir_all(p).expect("create_dir_all");
    }
    fs::write(full, content).expect("write_file");
}

/// Helper: derive a deterministic (data_dir, branch) pair from a workspace path.
fn test_db_params(path: &Path) -> (std::path::PathBuf, String) {
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("sql-refs-{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

/// Helper: build a `CodeGraphConfig` with SQL enabled.
fn sql_graph_config() -> CodeGraphConfig {
    CodeGraphConfig {
        supported_languages: vec!["sql".to_owned()],
        ..CodeGraphConfig::default()
    }
}

/// 033.001-T: indexing a SQL file that references a class defined in another SQL
/// file must create a resolved `references` edge pointing to the Class node.
///
/// **Red phase**: fails until `code_graph::index_workspace` no-op arm for
/// `ExtractedEdge::References` is replaced with the real persistence call.
#[test]
async fn sql_references_resolved_to_class_node() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    // File 1: defines the `users` class.
    write_file(ws, "schema.sql", "CREATE TABLE users (id INT, name VARCHAR(255));");

    // File 2: references `users` via SELECT FROM.
    write_file(ws, "queries.sql", "SELECT id, name FROM users WHERE active = 1;");

    let config = sql_graph_config();
    let (data_dir, branch) = test_db_params(ws);

    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index_workspace must succeed");

    assert!(result.errors.is_empty(), "no indexing errors expected; got: {:?}", result.errors);

    // Query the `references` table for any edge.
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    let mut resp = db
        .query("SELECT * FROM `references` LIMIT 20")
        .await
        .expect("query references table");
    let rows: Vec<serde_json::Value> = resp.take(0).expect("deserialize");

    assert!(
        !rows.is_empty(),
        "033.001-T: expected at least one references edge after indexing SQL workspace; \
         no edges found — the no-op arm in index_workspace must be replaced (red phase)"
    );
}

/// 033.001-T: indexing a SQL file that references a table with no corresponding
/// Class node must create an unresolved `references` edge (self-referencing file
/// with `qualified_name` set to the raw target string).
///
/// **Red phase**: fails until `code_graph::index_workspace` persists unresolved edges.
#[test]
async fn sql_references_unresolved_stays_in_graph() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    // Only a query file — no CREATE TABLE for `nonexistent_table`.
    write_file(
        ws,
        "orphan_query.sql",
        "SELECT id FROM nonexistent_table WHERE status = 'active';",
    );

    let config = sql_graph_config();
    let (data_dir, branch) = test_db_params(ws);

    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index_workspace must succeed");

    assert!(result.errors.is_empty(), "no indexing errors expected; got: {:?}", result.errors);

    // The unresolved edge must still be persisted with qualified_name set.
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    let mut resp = db
        .query("SELECT qualified_name FROM `references` LIMIT 10")
        .await
        .expect("query references table");
    let rows: Vec<serde_json::Value> = resp.take(0).expect("deserialize");

    assert!(
        !rows.is_empty(),
        "033.001-T: unresolved reference must produce an edge in the references table (red phase)"
    );
    let qn = rows[0].get("qualified_name").and_then(|v| v.as_str());
    assert!(
        qn == Some("nonexistent_table"),
        "qualified_name must be 'nonexistent_table' for an unresolved reference; got: {qn:?}"
    );
}

/// 033.001-T: `public.users` (schema-qualified name) must resolve via last-segment
/// fallback when a `users` Class node exists but `public.users` does not.
///
/// This test exercises the deliberation Finding 2 acceptance criterion:
/// when `get_class_by_name("public.users")` returns `None`, retry with last
/// segment `"users"` before declaring the reference unresolved.
///
/// **Red phase**: fails until both 033.001-T (graph wiring) AND 033.002-T (parser
/// emitting `"public.users"` as target) are implemented.
#[test]
async fn sql_references_qualified_name_fallback() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    // File 1: defines the `users` class under unqualified name.
    write_file(ws, "schema.sql", "CREATE TABLE users (id INT, name VARCHAR(255));");

    // File 2: references `public.users` — should fall back to `users` class.
    write_file(
        ws,
        "qualified_query.sql",
        "SELECT id, name FROM public.users WHERE active = 1;",
    );

    let config = sql_graph_config();
    let (data_dir, branch) = test_db_params(ws);

    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index_workspace must succeed");

    assert!(result.errors.is_empty(), "no indexing errors expected; got: {:?}", result.errors);

    // After implementation: verify a resolved edge exists (pointing to the users class).
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    let mut resp = db
        .query("SELECT * FROM `references` LIMIT 10")
        .await
        .expect("query references table");
    let rows: Vec<serde_json::Value> = resp.take(0).expect("deserialize");

    assert!(
        !rows.is_empty(),
        "033.001-T + 033.002-T: qualified-name fallback must produce a references edge (red phase)"
    );

    // The edge should point to a class node, not a code_file (resolved).
    let in_val = rows[0].get("in").and_then(|v| v.as_str()).unwrap_or("");
    let out_val = rows[0].get("out").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        out_val.starts_with("class:"),
        "qualified-name fallback: resolved edge must point to class node; got in={in_val:?} out={out_val:?}"
    );
}
