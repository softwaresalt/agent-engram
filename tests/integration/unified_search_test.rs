//! Integration tests for NaN-embedding guard in the write path (TASK-009.10).
//!
//! Verifies that:
//! * Records with NaN or infinite embedding values are rejected at **write time**
//!   by `upsert_function`, `upsert_class`, `upsert_interface`, and
//!   `upsert_content_record`.
//! * `gc_corrupted_embeddings` detects and removes records whose embeddings
//!   were stored before the guard existed (simulated by a raw DB insert).
//! * `unified_search` never returns error 5001 when all persisted embeddings
//!   are valid (the guard prevents the corrupted state that causes the error).

use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::db::{connect_db, queries::CodeGraphQueries};
use engram::server::state::AppState;
use engram::tools;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Create a fresh in-memory `SurrealDB` workspace and return connected queries.
async fn fresh_queries() -> (tempfile::TempDir, CodeGraphQueries) {
    let dir = tempfile::tempdir().expect("tempdir");
    let data_dir = dir.path().to_path_buf();
    let db = connect_db(&data_dir, "test-nan-branch")
        .await
        .expect("connect_db");
    let q = CodeGraphQueries::new(db);
    (dir, q)
}

/// Build a minimal [`engram::models::Function`] with the given embedding.
fn make_function(embedding: Vec<f32>) -> engram::models::Function {
    use engram::models::function::Function;
    Function {
        id: format!("function:{}", uuid::Uuid::new_v4()),
        name: "test_fn".to_owned(),
        file_path: "src/test.rs".to_owned(),
        line_start: 1,
        line_end: 5,
        signature: "fn test_fn()".to_owned(),
        docstring: None,
        body: "fn test_fn() {}".to_owned(),
        body_hash: "abc123".to_owned(),
        token_count: 3,
        embed_type: "explicit_code".to_owned(),
        embedding,
        summary: "A test function".to_owned(),
    }
}

/// Build a minimal [`engram::models::Class`] with the given embedding.
fn make_class(embedding: Vec<f32>) -> engram::models::Class {
    use engram::models::class::Class;
    Class {
        id: format!("class:{}", uuid::Uuid::new_v4()),
        name: "TestStruct".to_owned(),
        file_path: "src/test.rs".to_owned(),
        line_start: 1,
        line_end: 3,
        docstring: None,
        body: "struct TestStruct;".to_owned(),
        body_hash: "def456".to_owned(),
        token_count: 2,
        embed_type: "explicit_code".to_owned(),
        embedding,
        summary: "A test struct".to_owned(),
    }
}

/// Build a minimal [`engram::models::Interface`] with the given embedding.
fn make_interface(embedding: Vec<f32>) -> engram::models::Interface {
    use engram::models::interface::Interface;
    Interface {
        id: format!("interface:{}", uuid::Uuid::new_v4()),
        name: "TestTrait".to_owned(),
        file_path: "src/test.rs".to_owned(),
        line_start: 1,
        line_end: 3,
        docstring: None,
        body: "trait TestTrait {}".to_owned(),
        body_hash: "ghi789".to_owned(),
        token_count: 2,
        embed_type: "explicit_code".to_owned(),
        embedding,
        summary: "A test trait".to_owned(),
    }
}

/// Build a minimal [`engram::models::ContentRecord`] with the given embedding.
fn make_content_record(embedding: Option<Vec<f32>>) -> engram::models::ContentRecord {
    use chrono::Utc;
    use engram::models::content::ContentRecord;
    ContentRecord {
        id: uuid::Uuid::new_v4().to_string(),
        content_type: "text".to_owned(),
        file_path: "README.md".to_owned(),
        content_hash: "jkl012".to_owned(),
        content: "Test content".to_owned(),
        embedding,
        source_path: "README.md".to_owned(),
        file_size_bytes: 12,
        ingested_at: Utc::now(),
    }
}

// ── Write-time rejection tests ────────────────────────────────────────────────

/// Inserting a function whose embedding contains NaN must fail at write time.
#[test]
async fn nan_embedding_rejected_on_upsert_function() {
    let (_dir, queries) = fresh_queries().await;

    let mut nan_embedding = vec![0.0_f32; 384];
    nan_embedding[0] = f32::NAN;

    let func = make_function(nan_embedding);
    let result = queries.upsert_function(&func).await;

    assert!(
        result.is_err(),
        "upsert_function must reject NaN embeddings at write time; got Ok"
    );
}

/// Inserting a class whose embedding contains NaN must fail at write time.
#[test]
async fn nan_embedding_rejected_on_upsert_class() {
    let (_dir, queries) = fresh_queries().await;

    let mut nan_embedding = vec![0.0_f32; 384];
    nan_embedding[10] = f32::NAN;

    let class = make_class(nan_embedding);
    let result = queries.upsert_class(&class).await;

    assert!(
        result.is_err(),
        "upsert_class must reject NaN embeddings at write time"
    );
}

/// Inserting an interface whose embedding contains Inf must fail at write time.
#[test]
async fn inf_embedding_rejected_on_upsert_interface() {
    let (_dir, queries) = fresh_queries().await;

    let mut inf_embedding = vec![0.0_f32; 384];
    inf_embedding[5] = f32::INFINITY;

    let iface = make_interface(inf_embedding);
    let result = queries.upsert_interface(&iface).await;

    assert!(
        result.is_err(),
        "upsert_interface must reject Inf embeddings at write time"
    );
}

/// Inserting a content record with NaN embedding must fail at write time.
#[test]
async fn nan_embedding_rejected_on_upsert_content_record() {
    let (_dir, queries) = fresh_queries().await;

    let mut nan_embedding = vec![0.0_f32; 384];
    nan_embedding[100] = f32::NAN;

    let record = make_content_record(Some(nan_embedding));
    let result = queries.upsert_content_record(&record).await;

    assert!(
        result.is_err(),
        "upsert_content_record must reject NaN embeddings at write time"
    );
}

/// Valid zero embeddings (finite) must succeed at write time.
/// Confirms the guard does not block legitimate zero-vector placeholders.
#[test]
async fn zero_embedding_accepted_on_upsert_function() {
    let (_dir, queries) = fresh_queries().await;

    let valid_embedding = vec![0.0_f32; 384]; // all zeros, fully finite
    let func = make_function(valid_embedding);
    let result = queries.upsert_function(&func).await;

    assert!(
        result.is_ok(),
        "upsert_function must accept valid zero embeddings; got: {result:?}"
    );
}

// ── GC repair tests ───────────────────────────────────────────────────────────

/// `gc_corrupted_embeddings` returns 0 when the database contains no corrupted
/// records (regression guard: GC must not delete valid records).
#[test]
async fn gc_returns_zero_on_clean_database() {
    let (_dir, queries) = fresh_queries().await;

    // Insert a valid function.
    let valid_func = make_function(vec![0.1_f32; 384]);
    queries
        .upsert_function(&valid_func)
        .await
        .expect("upsert valid func");

    let deleted = queries
        .gc_corrupted_embeddings()
        .await
        .expect("gc_corrupted_embeddings must succeed");

    assert_eq!(
        deleted, 0,
        "gc must not delete valid records; deleted: {deleted}"
    );
}

/// `gc_corrupted_embeddings` detects and removes a record whose embedding was
/// persisted with NaN values before the write-path guard existed.
///
/// The corrupt record is inserted via `SurrealDB`'s native `Array::from(Vec<f64>)`
/// binding with `f64::NAN`, which is stored as a `SurrealDB` `Number::Float(NaN)`.
/// This simulates pre-existing database corruption from an older version of the code.
#[test]
async fn gc_removes_corrupted_record() {
    use surrealdb::sql::{Array, Value};

    let dir = tempfile::tempdir().expect("tempdir");
    let db = connect_db(dir.path(), "gc-test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db.clone());

    // Build a 384-dim embedding as Vec<f64> with NaN at position 0.
    // Array::from(Vec<f64>) preserves NaN as Number::Float(NaN), simulating
    // legacy data that bypasses our Rust-level f32 write guard.
    let nan_f64_vec: Vec<f64> = {
        let mut v = vec![0.0_f64; 384];
        v[0] = f64::NAN;
        v
    };
    let nan_embed_value: Value = Value::Array(Array::from(nan_f64_vec));

    // Insert directly via DB handle to bypass the Rust guard.
    let corrupt_raw_id = format!("corrupt_{}", uuid::Uuid::new_v4().simple());
    let corrupt_thing = surrealdb::sql::Thing::from(("function", corrupt_raw_id.as_str()));

    db.query(
        "UPSERT $id SET name = 'corrupt_fn', file_path = 'x.rs', embedding = $emb, \
         line_start = 1, line_end = 1, signature = 'fn corrupt()', body_hash = 'deadbeef', \
         token_count = 1, embed_type = 'explicit_code', summary = 'corrupt function'",
    )
    .bind(("id", corrupt_thing))
    .bind(("emb", nan_embed_value))
    .await
    .expect("raw insert of corrupt record");

    // WHEN gc_corrupted_embeddings runs
    let deleted = queries
        .gc_corrupted_embeddings()
        .await
        .expect("gc_corrupted_embeddings must not error");

    // THEN the corrupt record was removed
    assert!(
        deleted >= 1,
        "gc must remove at least the one corrupted record we inserted; deleted: {deleted}"
    );
}

// ── End-to-end: index_workspace + unified_search happy path ──────────────────

/// After `index_workspace`, `unified_search` must not return error 5001.
///
/// Uses a fresh workspace with no embeddings (the embeddings feature may or
/// may not be active).  The critical assertion is that a database-deserialization
/// error (5001) is never returned — the query either succeeds or fails with a
/// semantic/model-not-available error.
#[test]
async fn unified_search_never_returns_5001_after_clean_index() {
    use std::fs;

    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
    fs::create_dir_all(workspace.path().join("src")).expect("create src/");
    fs::write(
        workspace.path().join("src/lib.rs"),
        "pub fn greet(name: &str) -> String { format!(\"hello {name}\") }",
    )
    .expect("write lib.rs");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    tools::dispatch(
        state.clone(),
        "index_workspace",
        Some(json!({ "force": true })),
    )
    .await
    .expect("index_workspace");

    // Call unified_search; it may fail for model/feature reasons but NOT with
    // the NaN deserialization error (5001 DatabaseError).
    let result = tools::dispatch(
        state.clone(),
        "unified_search",
        Some(json!({ "query": "greeting function" })),
    )
    .await;

    if let Err(ref e) = result {
        let err_str = format!("{e:?}");
        assert!(
            !err_str.contains("NaNf64") && !err_str.contains("non-finite"),
            "unified_search must not fail with a NaN deserialization error; got: {err_str}"
        );
        // A model-not-loaded or search error is acceptable.
        // Only a database deserialization error is forbidden.
    }
    // If result is Ok, even better — the query returned results or an empty set.
}
