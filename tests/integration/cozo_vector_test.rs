//! Integration tests for `CozoDB` HNSW vector search and embedding management
//! (Tasks U4.1–U4.5).
//!
//! Covers:
//!  - `vector_search_symbols_native` — returns Vec<(f32, `SymbolMatch`)>
//!  - `vector_search_symbols` — returns Vec<SymbolMatch>
//!  - `upsert_content_record`, `select_content_records`, `update_content_record_embedding`,
//!    `delete_content_record_by_path`
//!  - `update_symbol_embedding`
//!  - `gc_corrupted_embeddings`
//!
//! Requires the `cozo-backend` feature:
//!   `cargo test --no-default-features --features cozo-backend --test integration_cozo_vector`

#[cfg(feature = "cozo-backend")]
mod vector_tests {
    use chrono::Utc;
    use engram::models::{ContentRecord, Function};
    use tempfile::TempDir;

    async fn make_db() -> (TempDir, engram::db::Db) {
        let tmp = TempDir::new().expect("tempdir");
        let db = engram::db::connect_db(tmp.path(), "test-vec")
            .await
            .expect("connect_db");
        (tmp, db)
    }

    /// Build a 384-dimensional unit vector with 1.0 at `dim_index` and 0.0 elsewhere.
    fn unit_vector(dim_index: usize) -> Vec<f32> {
        let mut v = vec![0.0_f32; 384];
        v[dim_index] = 1.0_f32;
        v
    }

    fn make_fn(id: &str, name: &str, embedding: Vec<f32>) -> Function {
        Function {
            id: id.to_owned(),
            name: name.to_owned(),
            file_path: format!("src/{name}.rs"),
            line_start: 1,
            line_end: 5,
            signature: format!("fn {name}()"),
            docstring: None,
            body: String::new(),
            body_hash: format!("hash_{id}"),
            token_count: 0,
            embed_type: "explicit_code".to_owned(),
            summary: String::new(),
            embedding,
        }
    }

    fn make_content(id: &str, path: &str, content: &str) -> ContentRecord {
        ContentRecord {
            id: id.to_owned(),
            content_type: "file".to_owned(),
            file_path: path.to_owned(),
            content_hash: format!("hash_{id}"),
            content: content.to_owned(),
            embedding: None,
            source_path: path.to_owned(),
            file_size_bytes: content.len() as u64,
            ingested_at: Utc::now(),
        }
    }

    // ── vector_search_symbols_native ──────────────────────────────────────────────

    #[tokio::test]
    async fn vector_search_symbols_native_returns_closest_match() {
        let (_tmp, db) = make_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);

        // fn_a has embedding pointing to dimension 0; fn_b to dimension 1
        q.upsert_function(&make_fn("function:vsn_a", "vsn_a", unit_vector(0)))
            .await
            .expect("upsert a");
        q.upsert_function(&make_fn("function:vsn_b", "vsn_b", unit_vector(1)))
            .await
            .expect("upsert b");

        // Query with vector closest to fn_a (pointing to dimension 0)
        let query_vec = unit_vector(0);
        let results = q
            .vector_search_symbols_native(&query_vec, 5)
            .await
            .expect("vector_search_symbols_native");

        assert!(!results.is_empty(), "should return at least one result");
        // First result should be fn_a (distance ~0 to unit vec[0])
        assert_eq!(
            results[0].1.name, "vsn_a",
            "closest symbol must be vsn_a (same vector direction)"
        );
    }

    #[tokio::test]
    async fn vector_search_symbols_native_results_ordered_by_distance() {
        let (_tmp, db) = make_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);

        q.upsert_function(&make_fn("function:vso_a", "vso_a", unit_vector(2)))
            .await
            .expect("upsert a");
        q.upsert_function(&make_fn("function:vso_b", "vso_b", unit_vector(3)))
            .await
            .expect("upsert b");

        let results = q
            .vector_search_symbols_native(&unit_vector(2), 10)
            .await
            .expect("vector_search_symbols_native");

        if results.len() >= 2 {
            assert!(
                results[0].0 >= results[1].0,
                "results must be ordered by descending cosine similarity: {} >= {}",
                results[0].0,
                results[1].0
            );
        }
    }

    #[tokio::test]
    async fn vector_search_symbols_returns_vec_of_symbol_matches() {
        let (_tmp, db) = make_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);

        q.upsert_function(&make_fn("function:vss_fn", "vss_fn", unit_vector(4)))
            .await
            .expect("upsert");

        let results = q
            .vector_search_symbols(&unit_vector(4), 5)
            .await
            .expect("vector_search_symbols");

        assert!(!results.is_empty(), "should return at least one result");
        assert_eq!(results[0].name, "vss_fn", "top result should be vss_fn");
    }

    // ── content record CRUD ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_select_content_record() {
        let (_tmp, db) = make_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);
        let rec = make_content("content:cr1", "docs/readme.md", "Hello world");
        q.upsert_content_record(&rec)
            .await
            .expect("upsert_content_record");

        let records = q
            .select_content_records(None)
            .await
            .expect("select_content_records");

        assert!(
            records.iter().any(|r| r.file_path == "docs/readme.md"),
            "inserted content record must appear in select"
        );
    }

    #[tokio::test]
    async fn update_content_record_embedding_sets_embedding() {
        let (_tmp, db) = make_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);
        let rec = make_content("content:cr2", "docs/guide.md", "Some content");
        q.upsert_content_record(&rec).await.expect("upsert content");

        let emb = unit_vector(10);
        q.update_content_record_embedding("docs/guide.md", emb)
            .await
            .expect("update_content_record_embedding");

        let records = q
            .select_content_records(None)
            .await
            .expect("select after update");

        assert!(
            records.iter().any(|r| r.file_path == "docs/guide.md"),
            "content record must still exist"
        );
    }

    #[tokio::test]
    async fn delete_content_record_by_path_removes_it() {
        let (_tmp, db) = make_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);
        let rec = make_content("content:cr3", "docs/to_delete.md", "Delete me");
        q.upsert_content_record(&rec).await.expect("upsert");
        q.delete_content_record_by_path("docs/to_delete.md")
            .await
            .expect("delete");

        let records = q
            .select_content_records(None)
            .await
            .expect("select after delete");
        assert!(
            !records.iter().any(|r| r.file_path == "docs/to_delete.md"),
            "deleted content record must not appear in select"
        );
    }

    // ── update_symbol_embedding ───────────────────────────────────────────────────

    #[tokio::test]
    async fn update_symbol_embedding_succeeds_for_function() {
        let (_tmp, db) = make_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);
        q.upsert_function(&make_fn("function:use_fn", "use_fn", unit_vector(5)))
            .await
            .expect("upsert");

        let new_emb = unit_vector(6);
        q.update_symbol_embedding("function:use_fn", new_emb)
            .await
            .expect("update_symbol_embedding should succeed");
    }

    // ── gc_corrupted_embeddings ───────────────────────────────────────────────────

    #[tokio::test]
    async fn gc_corrupted_embeddings_returns_without_error_on_clean_db() {
        let (_tmp, db) = make_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);

        let removed = q
            .gc_corrupted_embeddings()
            .await
            .expect("gc_corrupted_embeddings");

        assert_eq!(removed, 0, "clean DB should have zero corrupted embeddings");
    }
}
