//! Benchmark tests for CozoDB Phase 3-4 operations (Task U0.5).
//!
//! These tests are decorated with `#[ignore]` so they do NOT run during
//! `cargo test`.  Run them individually via:
//!   `cargo test --no-default-features --features cozo-backend --test integration_cozo_benchmark -- --ignored`
//!
//! Each benchmark measures wall-clock time for bulk operations:
//!  - Bulk edge creation: 500 calls edges
//!  - Bulk vector search: 50 queries × top-10 results
//!
//! Pass / Fail criteria: operations must complete within stated time limits.

use engram::models::Function;
use std::time::Instant;
use tempfile::TempDir;

async fn make_db() -> (TempDir, engram::db::Db) {
    let tmp = TempDir::new().expect("tempdir");
    let db = engram::db::connect_db(tmp.path(), "bench")
        .await
        .expect("connect_db");
    (tmp, db)
}

fn make_fn(index: usize) -> Function {
    let id = format!("function:bench_fn_{index:05}");
    let name = format!("bench_fn_{index:05}");
    let mut emb = vec![0.0_f32; 384];
    emb[index % 384] = 1.0_f32;
    Function {
        id,
        name: name.clone(),
        file_path: format!("src/bench_{index}.rs"),
        line_start: 1,
        line_end: 5,
        signature: format!("fn {name}()"),
        docstring: None,
        body: String::new(),
        body_hash: format!("hash_{index}"),
        token_count: 0,
        embed_type: "explicit_code".to_owned(),
        summary: String::new(),
        embedding: emb,
    }
}

/// Benchmark: create 500 calls edges and measure total time.
///
/// All functions are pre-inserted, then 499 sequential `create_calls_edge`
/// calls form a linear call chain.  Must complete in under 60 seconds.
#[tokio::test]
#[ignore]
async fn bench_bulk_edge_creation_500() {
    const N: usize = 500;
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);

    // Insert all functions first
    for i in 0..N {
        q.upsert_function(&make_fn(i))
            .await
            .expect("upsert bench fn");
    }

    let start = Instant::now();
    for i in 0..N - 1 {
        let caller = format!("function:bench_fn_{i:05}");
        let callee = format!("function:bench_fn_{:05}", i + 1);
        q.create_calls_edge(&caller, &callee)
            .await
            .expect("bench create_calls_edge");
    }
    let elapsed = start.elapsed();
    eprintln!("bench_bulk_edge_creation_500: {elapsed:.2?} for {N} edges");

    assert!(
        elapsed.as_secs() < 60,
        "500 edge creations must complete in < 60s, took {elapsed:.2?}"
    );
}

/// Benchmark: perform 50 vector search queries (top-10) and measure total time.
///
/// 100 functions with distinct unit vectors are pre-inserted.
/// 50 kNN queries are executed against those vectors.
/// Must complete in under 30 seconds.
#[tokio::test]
#[ignore]
async fn bench_vector_search_50_queries() {
    const SYMBOLS: usize = 100;
    const QUERIES: usize = 50;
    const K: usize = 10;

    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);

    for i in 0..SYMBOLS {
        q.upsert_function(&make_fn(i)).await.expect("upsert vec fn");
    }

    let start = Instant::now();
    for i in 0..QUERIES {
        let mut query_vec = vec![0.0_f32; 384];
        query_vec[i % 384] = 1.0_f32;
        q.vector_search_symbols_native(&query_vec, K)
            .await
            .expect("bench vector_search_symbols_native");
    }
    let elapsed = start.elapsed();
    eprintln!("bench_vector_search_50_queries: {elapsed:.2?} for {QUERIES} queries");

    assert!(
        elapsed.as_secs() < 30,
        "50 vector queries must complete in < 30s, took {elapsed:.2?}"
    );
}
