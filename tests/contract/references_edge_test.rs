//! Contract tests for the `references` relation edge — 033.004-T and 035-F.
//!
//! Verifies that `create_references_edge` creates a persisted edge in the
//! `SurrealDB` `references` table and can be queried back. Also verifies
//! schema tuning, batch-resolution optimization, DRY helper, and heuristics
//! introduced by feature 035-F (SQL Reference Resolution Hardening).

use std::collections::HashSet;
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
/// After implementation, creates an edge from a `code_file` node to a class node
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
    let row = &rows[0];
    assert_eq!(
        row.get("source").and_then(|v| v.as_str()),
        Some("code_file:abc"),
        "source field must match the supplied source_id"
    );
    assert_eq!(
        row.get("target").and_then(|v| v.as_str()),
        Some("class:def"),
        "target field must match the supplied target_id"
    );
    assert_eq!(
        row.get("qualified_name").and_then(|v| v.as_str()),
        Some("users"),
        "qualified_name field must match the supplied qualified_name"
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
        qn.is_some_and(|v| v.as_str() == Some("nonexistent_table")),
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
        .query("SELECT source, target, qualified_name FROM `references` LIMIT 10")
        .await
        .expect("query references table");
    let rows: Vec<serde_json::Value> = resp.take(0).expect("deserialize");
    assert!(
        rows.is_empty(),
        "references table must be empty after delete_edges_from_file"
    );
}

// ── 035.001-T: references_target index ──────────────────────────────────

/// 035.001-T: schema must define a `references_target` index on the `references` table.
///
/// Connecting to a fresh embedded DB applies the schema automatically.
/// `INFO FOR TABLE \`references\`` must report the `references_target` index.
#[test]
async fn contract_references_target_index_exists() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (data_dir, branch) = test_db_params(tmp.path());
    let db = engram::db::connect_db(&data_dir, &branch)
        .await
        .expect("db connect");

    let mut resp = db
        .query("INFO FOR TABLE `references`")
        .await
        .expect("INFO FOR TABLE `references`");
    let info: Vec<serde_json::Value> = resp.take(0).expect("deserialize info");
    let info_str = serde_json::to_string(&info).expect("serialize");
    assert!(
        info_str.contains("references_target"),
        "035.001-T: `references` table must have a `references_target` index; \
         got: {info_str}"
    );
}

// ── 035.002-T: batch-UPDATE optimization ────────────────────────────────

/// 035.002-T: `reresolve_references_edges` must batch class lookups.
///
/// Creates 3 self-loop edges across 2 distinct sources but with 2 distinct
/// `qualified_names` for class nodes that exist, so `unique_names.len() == 2`.
/// A fourth edge uses a name with no matching class (stays unresolved).
///
/// Red phase: fails until `reresolve_references_edges` returns `ReresolveResult`
/// and `lookups <= unique_names_count` (batch) rather than one lookup per edge.
#[test]
async fn contract_reresolve_batch_optimization() {
    use engram::db::queries::ReresolveResult;
    use engram::models::Class;

    let tmp = tempfile::tempdir().expect("tempdir");
    let (data_dir, branch) = test_db_params(tmp.path());
    let db = engram::db::connect_db(&data_dir, &branch)
        .await
        .expect("db connect");
    let q = engram::db::queries::CodeGraphQueries::new(db.clone());

    // Insert Class nodes for "users" and "accounts" via upsert_class so that
    // schema validation is properly checked.
    q.upsert_class(&Class {
        id: "class:batch_users".to_owned(),
        name: "users".to_owned(),
        file_path: "schema.sql".to_owned(),
        line_start: 1,
        line_end: 1,
        docstring: None,
        body: String::new(),
        body_hash: "h1".to_owned(),
        token_count: 1,
        embed_type: "explicit_code".to_owned(),
        embedding: vec![0.0_f32; 384],
        summary: String::new(),
    })
    .await
    .expect("create users class");
    q.upsert_class(&Class {
        id: "class:batch_accounts".to_owned(),
        name: "accounts".to_owned(),
        file_path: "schema.sql".to_owned(),
        line_start: 2,
        line_end: 2,
        docstring: None,
        body: String::new(),
        body_hash: "h2".to_owned(),
        token_count: 1,
        embed_type: "explicit_code".to_owned(),
        embedding: vec![0.0_f32; 384],
        summary: String::new(),
    })
    .await
    .expect("create accounts class");

    // Insert 4 self-loop edges:
    //  - file1 → users (resolvable, first edge for this name)
    //  - file2 → users (resolvable, second edge for same class name)
    //  - file3 → accounts (resolvable)
    //  - file4 → nonexistent (unresolvable)
    for (src, qn) in [
        ("code_file:file1", "users"),
        ("code_file:file2", "users"),
        ("code_file:file3", "accounts"),
        ("code_file:file4", "nonexistent"),
    ] {
        q.create_references_edge(src, src, Some(qn))
            .await
            .expect("create self-loop edge");
    }

    let result: ReresolveResult = q
        .reresolve_references_edges()
        .await
        .expect("reresolve_references_edges");

    assert_eq!(
        result.resolved, 3,
        "035.002-T: 3 edges should be resolved (users×2 + accounts×1); got {}",
        result.resolved
    );

    // Unique qualified_names that have matching classes: {"users", "accounts"} = 2.
    // Batch must issue ≤ 2 class-lookup round-trips (unique names count).
    // N+1 implementation would issue 4 lookups (one per edge), which fails this check.
    let unique_resolvable: HashSet<&str> = ["users", "accounts"].into_iter().collect();
    assert!(
        result.lookups <= unique_resolvable.len(),
        "035.002-T: batch lookup must issue ≤ {} class lookups (unique names count); \
         got {} (N+1 anti-pattern)",
        unique_resolvable.len(),
        result.lookups
    );
}

// ── 035.003-T: resolve_reference_target helper ──────────────────────────

/// 035.003-T: `resolve_reference_target` helper must resolve names to Class IDs.
///
/// Red phase: method does not exist yet — this test will not compile until
/// `resolve_reference_target` is added to `CodeGraphQueries`.
#[test]
async fn contract_resolve_reference_target_helper() {
    use engram::models::Class;

    let tmp = tempfile::tempdir().expect("tempdir");
    let (data_dir, branch) = test_db_params(tmp.path());
    let db = engram::db::connect_db(&data_dir, &branch)
        .await
        .expect("db connect");
    let q = engram::db::queries::CodeGraphQueries::new(db.clone());

    // Insert a Class node named "users" via upsert_class for proper error checking.
    q.upsert_class(&Class {
        id: "class:helper_users".to_owned(),
        name: "users".to_owned(),
        file_path: "schema.sql".to_owned(),
        line_start: 1,
        line_end: 1,
        docstring: None,
        body: String::new(),
        body_hash: "h1".to_owned(),
        token_count: 1,
        embed_type: "explicit_code".to_owned(),
        embedding: vec![0.0_f32; 384],
        summary: String::new(),
    })
    .await
    .expect("create users class");

    // Exact match.
    let id = q
        .resolve_reference_target("users")
        .await
        .expect("resolve exact")
        .expect("must resolve to Some");
    assert!(
        id.starts_with("class:"),
        "035.003-T: resolved id must be a class: prefixed string; got {id}"
    );

    // Schema-qualified fallback: "public.users" → "users".
    let fallback_id = q
        .resolve_reference_target("public.users")
        .await
        .expect("resolve qualified")
        .expect("qualified fallback must resolve");
    assert_eq!(
        id, fallback_id,
        "035.003-T: schema-qualified fallback must resolve to same class as exact match"
    );

    // Unresolvable name returns None.
    let none_result = q
        .resolve_reference_target("nonexistent_table")
        .await
        .expect("resolve nonexistent");
    assert!(
        none_result.is_none(),
        "035.003-T: nonexistent name must return None"
    );
}

// ── 035.004-T: case-insensitive and quote-stripping heuristics ──────────

/// 035.004-T: `resolve_reference_target` must handle quoted identifiers and
/// case-insensitive names.
///
/// Red phase: `get_class_by_name_ci` and `strip_sql_quotes` do not exist yet.
#[test]
async fn contract_resolve_reference_target_heuristics() {
    use engram::models::Class;

    let tmp = tempfile::tempdir().expect("tempdir");
    let (data_dir, branch) = test_db_params(tmp.path());
    let db = engram::db::connect_db(&data_dir, &branch)
        .await
        .expect("db connect");
    let q = engram::db::queries::CodeGraphQueries::new(db.clone());

    // Insert a Class node named "Users" (PascalCase) via upsert_class.
    q.upsert_class(&Class {
        id: "class:heuristic_users".to_owned(),
        name: "Users".to_owned(),
        file_path: "schema.sql".to_owned(),
        line_start: 1,
        line_end: 1,
        docstring: None,
        body: String::new(),
        body_hash: "h1".to_owned(),
        token_count: 1,
        embed_type: "explicit_code".to_owned(),
        embedding: vec![0.0_f32; 384],
        summary: String::new(),
    })
    .await
    .expect("create Users class");

    // Case-insensitive: "users" (lowercase) → should resolve to "Users" node.
    let id_ci = q
        .resolve_reference_target("users")
        .await
        .expect("resolve lowercase")
        .expect("035.004-T: case-insensitive fallback must resolve 'users' to 'Users' node");
    assert!(
        id_ci.starts_with("class:"),
        "035.004-T: case-insensitive resolved id must be class:-prefixed; got {id_ci}"
    );

    // Double-quoted identifier: `"Users"` → strip quotes → "Users".
    let id_dq = q
        .resolve_reference_target(r#""Users""#)
        .await
        .expect("resolve double-quoted")
        .expect("035.004-T: double-quoted identifier must resolve after stripping quotes");
    assert_eq!(
        id_ci, id_dq,
        "035.004-T: double-quoted identifier must resolve to same class as plain name"
    );

    // Bracket-quoted: "[Users]" → strip quotes → "Users".
    let id_bracket = q
        .resolve_reference_target("[Users]")
        .await
        .expect("resolve bracket-quoted")
        .expect("035.004-T: bracket-quoted identifier must resolve after stripping quotes");
    assert_eq!(
        id_ci, id_bracket,
        "035.004-T: bracket-quoted identifier must resolve to same class as plain name"
    );
}

/// 035.004-T: contract test verifying case-insensitive edge re-resolution.
///
/// Creates a Class named "Users" (`PascalCase`) and a self-loop reference with
/// `qualified_name` = "users" (lowercase). After `reresolve_references_edges`,
/// the edge must point to the "Users" class node.
#[test]
async fn contract_reresolve_case_insensitive_resolution() {
    use engram::models::Class;

    let tmp = tempfile::tempdir().expect("tempdir");
    let (data_dir, branch) = test_db_params(tmp.path());
    let db = engram::db::connect_db(&data_dir, &branch)
        .await
        .expect("db connect");
    let q = engram::db::queries::CodeGraphQueries::new(db.clone());

    // Class "Users" (PascalCase) via upsert_class for proper error checking.
    q.upsert_class(&Class {
        id: "class:ci_users".to_owned(),
        name: "Users".to_owned(),
        file_path: "schema.sql".to_owned(),
        line_start: 1,
        line_end: 1,
        docstring: None,
        body: String::new(),
        body_hash: "h1".to_owned(),
        token_count: 1,
        embed_type: "explicit_code".to_owned(),
        embedding: vec![0.0_f32; 384],
        summary: String::new(),
    })
    .await
    .expect("create Users class");

    // Self-loop edge with lowercase qualified_name.
    q.create_references_edge("code_file:f1", "code_file:f1", Some("users"))
        .await
        .expect("create self-loop edge");

    let result = q
        .reresolve_references_edges()
        .await
        .expect("reresolve_references_edges");

    assert_eq!(
        result.resolved, 1,
        "035.004-T: case-insensitive re-resolution must resolve 1 edge; got {}",
        result.resolved
    );

    // Verify the edge now points to the Users class (not the source file).
    let mut resp = db
        .query(
            "SELECT target FROM `references` WHERE source = 'code_file:f1' \
             AND qualified_name = 'users' LIMIT 1",
        )
        .await
        .expect("query resolved edge");
    let rows: Vec<serde_json::Value> = resp.take(0).expect("deserialize");
    assert!(!rows.is_empty(), "edge must exist after re-resolution");
    let target = rows[0].get("target").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        target.starts_with("class:"),
        "035.004-T: edge target must be a class node after case-insensitive re-resolution; \
         got {target:?}"
    );
}
