//! Integration tests for `CozoDB` symbol CRUD parity
//! (Tasks 001.003.003-T through 001.003.009-T — U2.2–U2.9).
//!
//! Tests the full write–read–delete roundtrip for each symbol type
//! (`code_file`, `function`, `class`, `interface`) and the aggregate count
//! queries against a `CozoDB` handle.
//!
//! Requires the `cozo-backend` feature:
//!   `cargo test --no-default-features --features cozo-backend --test integration_cozo_crud`

use tempfile::TempDir;

/// Construct a temporary data directory and connect to the `CozoDB` backend.
///
/// Fails (panics) until Phase 2 implements `connect_db`.
async fn make_db() -> (TempDir, engram::db::Db) {
    let tmp = TempDir::new().expect("tempdir");
    let db = engram::db::connect_db(tmp.path(), "test-branch")
        .await
        .expect("connect_db must succeed for CRUD tests (Phase 2 U2.1)");
    (tmp, db)
}

// ── code_file CRUD (U2.3) ─────────────────────────────────────────────────

#[tokio::test]
async fn upsert_and_get_code_file() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let file = engram::models::CodeFile {
        id: "file:test-001".into(),
        path: "src/lib.rs".into(),
        language: "rust".into(),
        size_bytes: 1024,
        content_hash: "abc123".into(),
        last_indexed_at: "2026-01-01T00:00:00Z".into(),
    };
    q.upsert_code_file(&file).await.expect("upsert_code_file");
    let retrieved = q
        .get_code_file_by_path("src/lib.rs")
        .await
        .expect("get_code_file_by_path");
    assert!(
        retrieved.is_some(),
        "code file must be retrievable after upsert"
    );
    assert_eq!(retrieved.unwrap().path, "src/lib.rs");
}

#[tokio::test]
async fn delete_code_file_removes_it() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let file = engram::models::CodeFile {
        id: "file:delete-test".into(),
        path: "src/to_delete.rs".into(),
        language: "rust".into(),
        size_bytes: 256,
        content_hash: "deadbeef".into(),
        last_indexed_at: "2026-01-01T00:00:00Z".into(),
    };
    q.upsert_code_file(&file).await.expect("upsert");
    q.delete_code_file("src/to_delete.rs")
        .await
        .expect("delete");
    let result = q
        .get_code_file_by_path("src/to_delete.rs")
        .await
        .expect("get after delete");
    assert!(result.is_none(), "deleted file must not be retrievable");
}

// ── function CRUD (U2.4) ───────────────────────────────────────────────────

#[tokio::test]
async fn upsert_and_get_function() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let func = engram::models::Function {
        id: "fn:test-fn-001".into(),
        name: "test_function".into(),
        file_path: "src/lib.rs".into(),
        line_start: 10,
        line_end: 20,
        signature: "fn test_function()".into(),
        docstring: Some("A test function.".into()),
        body: "fn test_function() {}".into(),
        body_hash: "hash001".into(),
        token_count: 42,
        embed_type: "explicit_code".into(),
        embedding: vec![0.1_f32; engram::services::embedding::EMBEDDING_DIM],
        summary: "Test function summary".into(),
    };
    q.upsert_function(&func).await.expect("upsert_function");
    let retrieved = q
        .get_function_by_name("test_function")
        .await
        .expect("get_function_by_name");
    assert!(
        retrieved.is_some(),
        "function must be retrievable after upsert"
    );
    assert_eq!(retrieved.unwrap().name, "test_function");
}

#[tokio::test]
async fn delete_functions_by_file() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let func = engram::models::Function {
        id: "fn:del-fn-001".into(),
        name: "fn_to_delete".into(),
        file_path: "src/old.rs".into(),
        line_start: 1,
        line_end: 5,
        signature: "fn fn_to_delete()".into(),
        docstring: None,
        body: "fn fn_to_delete() {}".into(),
        body_hash: "hash-del".into(),
        token_count: 10,
        embed_type: "explicit_code".into(),
        embedding: vec![0.0_f32; engram::services::embedding::EMBEDDING_DIM],
        summary: String::new(),
    };
    q.upsert_function(&func).await.expect("upsert");
    q.delete_functions_by_file("src/old.rs")
        .await
        .expect("delete_functions_by_file");
    let fns = q
        .get_functions_by_file("src/old.rs")
        .await
        .expect("get_functions_by_file after delete");
    assert!(fns.is_empty(), "functions must be gone after file delete");
}

// ── class CRUD (U2.5) ──────────────────────────────────────────────────────

#[tokio::test]
async fn upsert_and_get_class() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let class = engram::models::Class {
        id: "class:test-cls-001".into(),
        name: "TestClass".into(),
        file_path: "src/lib.rs".into(),
        line_start: 30,
        line_end: 60,
        docstring: Some("A test class.".into()),
        body: "struct TestClass {}".into(),
        body_hash: "classhash001".into(),
        token_count: 80,
        embed_type: "explicit_code".into(),
        embedding: vec![0.2_f32; engram::services::embedding::EMBEDDING_DIM],
        summary: "TestClass summary".into(),
    };
    q.upsert_class(&class).await.expect("upsert_class");
    let retrieved = q
        .get_class_by_name("TestClass")
        .await
        .expect("get_class_by_name");
    assert!(
        retrieved.is_some(),
        "class must be retrievable after upsert"
    );
    assert_eq!(retrieved.unwrap().name, "TestClass");
}

// ── interface CRUD (U2.6) ──────────────────────────────────────────────────

#[tokio::test]
async fn upsert_and_get_interface() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let iface = engram::models::Interface {
        id: "iface:test-iface-001".into(),
        name: "TestInterface".into(),
        file_path: "src/traits.rs".into(),
        line_start: 5,
        line_end: 15,
        docstring: Some("A test interface.".into()),
        body: "trait TestInterface {}".into(),
        body_hash: "ifacehash001".into(),
        token_count: 30,
        embed_type: "explicit_code".into(),
        embedding: vec![0.3_f32; engram::services::embedding::EMBEDDING_DIM],
        summary: "TestInterface summary".into(),
    };
    q.upsert_interface(&iface).await.expect("upsert_interface");
    let retrieved = q
        .get_interface_by_name("TestInterface")
        .await
        .expect("get_interface_by_name");
    assert!(
        retrieved.is_some(),
        "interface must be retrievable after upsert"
    );
    assert_eq!(retrieved.unwrap().name, "TestInterface");
}

// ── 3-table write fan-out (U2.2) ──────────────────────────────────────────

#[tokio::test]
async fn function_upsert_writes_meta_code_and_embedding_tables() {
    // U2.2: each upsert fans out to three tables — meta, code, embedding.
    // After upsert, count_functions must reflect the new row.
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let count_before = q.count_functions().await.expect("count_functions before");
    let func = engram::models::Function {
        id: "fn:fanout-test".into(),
        name: "fanout_fn".into(),
        file_path: "src/fanout.rs".into(),
        line_start: 1,
        line_end: 10,
        signature: "fn fanout_fn()".into(),
        docstring: None,
        body: "fn fanout_fn() {}".into(),
        body_hash: "fanout-hash".into(),
        token_count: 20,
        embed_type: "explicit_code".into(),
        embedding: vec![0.5_f32; engram::services::embedding::EMBEDDING_DIM],
        summary: String::new(),
    };
    q.upsert_function(&func)
        .await
        .expect("upsert for fan-out test");
    let count_after = q.count_functions().await.expect("count_functions after");
    assert_eq!(
        count_after,
        count_before + 1,
        "count_functions must increment after upsert"
    );
}

// ── Aggregate counts (U2.9) ───────────────────────────────────────────────

#[tokio::test]
async fn count_code_files_returns_zero_on_empty_db() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let count = q.count_code_files().await.expect("count_code_files");
    assert_eq!(count, 0, "fresh DB must have zero code files");
}

#[tokio::test]
async fn count_functions_returns_zero_on_empty_db() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let count = q.count_functions().await.expect("count_functions");
    assert_eq!(count, 0, "fresh DB must have zero functions");
}

#[tokio::test]
async fn count_classes_returns_zero_on_empty_db() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let count = q.count_classes().await.expect("count_classes");
    assert_eq!(count, 0, "fresh DB must have zero classes");
}

#[tokio::test]
async fn count_interfaces_returns_zero_on_empty_db() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    let count = q.count_interfaces().await.expect("count_interfaces");
    assert_eq!(count, 0, "fresh DB must have zero interfaces");
}
