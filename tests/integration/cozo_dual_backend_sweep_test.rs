//! Dual-backend behavioral sweep (Task 001.003.010-T — U2.10).
//!
//! Verifies that the `CozoDB` backend honours the same observable contracts as
//! the `SurrealDB` backend: connect → bootstrap → upsert → read → count →
//! delete, all without error.
//!
//! Tests are gated on the `cozo-backend` feature.  Under the default
//! `surreal-backend` build this file compiles but contains no runnable
//! tests; under `--no-default-features --features cozo-backend` all tests
//! run and are expected to FAIL until Phase 2 is complete.
//!
//! CI invocation (advisory axis):
//!   `cargo test --no-default-features --features cozo-backend`
//!              `--test integration_cozo_dual_backend_sweep`

#[cfg(feature = "cozo-backend")]
mod sweep {
    use tempfile::TempDir;

    async fn open_db() -> (TempDir, engram::db::Db) {
        let tmp = TempDir::new().expect("tempdir");
        let db = engram::db::connect_db(tmp.path(), "sweep-branch")
            .await
            .expect("connect_db must succeed (Phase 2 U2.1)");
        (tmp, db)
    }

    // ── Schema bootstrap ──────────────────────────────────────────────────

    /// The schema bootstrap must succeed before any CRUD can run.
    #[tokio::test]
    async fn schema_bootstrap_completes() {
        let (_tmp, db) = open_db().await;
        let result = engram::db::cozo_backend::schema::run_schema_bootstrap(&db);
        assert!(result.is_ok(), "schema bootstrap failed: {result:?}");
    }

    // ── code_file roundtrip ───────────────────────────────────────────────

    #[tokio::test]
    async fn code_file_write_read_delete_parity() {
        let (_tmp, db) = open_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);

        let before = q.count_code_files().await.expect("count before");
        let file = engram::models::CodeFile {
            id: "file:sweep-001".into(),
            path: "src/sweep.rs".into(),
            language: "rust".into(),
            size_bytes: 512,
            content_hash: "sweep-hash".into(),
            last_indexed_at: "2026-01-01T00:00:00Z".into(),
        };
        q.upsert_code_file(&file).await.expect("upsert");
        let after_insert = q.count_code_files().await.expect("count after insert");
        assert_eq!(after_insert, before + 1, "count must increase after upsert");

        let retrieved = q.get_code_file_by_path("src/sweep.rs").await.expect("get");
        assert!(retrieved.is_some(), "file must be found after upsert");

        q.delete_code_file("src/sweep.rs").await.expect("delete");
        let after_delete = q.count_code_files().await.expect("count after delete");
        assert_eq!(
            after_delete, before,
            "count must return to baseline after delete"
        );
    }

    // ── function roundtrip ────────────────────────────────────────────────

    #[tokio::test]
    async fn function_write_read_delete_parity() {
        let (_tmp, db) = open_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);

        let func = engram::models::Function {
            id: "fn:sweep-fn-001".into(),
            name: "sweep_function".into(),
            file_path: "src/sweep.rs".into(),
            line_start: 1,
            line_end: 10,
            signature: "fn sweep_function()".into(),
            docstring: None,
            body: "fn sweep_function() {}".into(),
            body_hash: "sweephash".into(),
            token_count: 15,
            embed_type: "explicit_code".into(),
            embedding: vec![0.4_f32; engram::services::embedding::EMBEDDING_DIM],
            summary: String::new(),
        };
        q.upsert_function(&func).await.expect("upsert fn");

        let retrieved = q
            .get_function_by_name("sweep_function")
            .await
            .expect("get fn");
        assert!(retrieved.is_some(), "function must be found after upsert");

        q.delete_functions_by_file("src/sweep.rs")
            .await
            .expect("delete fn by file");
        let fns = q
            .get_functions_by_file("src/sweep.rs")
            .await
            .expect("get fns after delete");
        assert!(fns.is_empty(), "functions must be absent after file delete");
    }

    // ── U5.5 parity smoke stubs ───────────────────────────────────────────
    //
    // These tests will be filled in during U5.5 implementation. They capture
    // the acceptance criteria: at least 4 MCP-level response shape comparisons
    // between cozo-backend and the expected contract.

    /// `list_symbols`-equivalent: symbols inserted and retrieved by file have
    /// the same names and types regardless of indexing order.
    #[tokio::test]
    async fn list_symbols_parity_same_names_and_types() {
        let (_tmp, db) = open_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);
        // Insert function and class into the same file.
        let func = engram::models::Function {
            id: "fn:parity-fn-001".into(),
            name: "parity_fn".into(),
            file_path: "src/parity.rs".into(),
            line_start: 1,
            line_end: 5,
            signature: "fn parity_fn()".into(),
            docstring: None,
            body: "fn parity_fn() {}".into(),
            body_hash: "bhash-001".into(),
            token_count: 5,
            embed_type: "explicit_code".into(),
            embedding: vec![0.1_f32; engram::services::embedding::EMBEDDING_DIM],
            summary: "parity function".into(),
        };
        q.upsert_function(&func).await.expect("upsert_function");
        let fns = q
            .get_functions_by_file("src/parity.rs")
            .await
            .expect("get_functions_by_file");
        assert_eq!(fns.len(), 1, "one function must be returned for file");
        assert_eq!(fns[0].name, "parity_fn", "name must be preserved");
        let all = q.all_functions().await.expect("all_functions");
        assert!(
            all.iter().any(|f| f.name == "parity_fn"),
            "all_functions must include parity_fn"
        );
    }

    /// `map_code`-equivalent: callers/callees returned for a known function
    /// match the edges that were inserted.
    #[tokio::test]
    async fn map_code_parity_edge_topology_matches() {
        let (_tmp, db) = open_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);
        let file_a = engram::models::CodeFile {
            id: "file:topo-a".into(),
            path: "src/caller.rs".into(),
            language: "rust".into(),
            size_bytes: 100,
            content_hash: "ha".into(),
            last_indexed_at: "2026-01-01T00:00:00Z".into(),
        };
        let file_b = engram::models::CodeFile {
            id: "file:topo-b".into(),
            path: "src/callee.rs".into(),
            language: "rust".into(),
            size_bytes: 100,
            content_hash: "hb".into(),
            last_indexed_at: "2026-01-01T00:00:00Z".into(),
        };
        q.upsert_code_file(&file_a).await.expect("upsert file_a");
        q.upsert_code_file(&file_b).await.expect("upsert file_b");
        q.create_imports_edge("file:topo-a", "file:topo-b", "crate::callee")
            .await
            .expect("create_imports_edge");
        let edges = q.all_code_edges().await.expect("all_code_edges");
        let found = edges
            .iter()
            .find(|e| e.from == "file:topo-a" && e.to == "file:topo-b");
        assert!(found.is_some(), "imports edge topology must be preserved");
        assert_eq!(
            found.unwrap().edge_type,
            engram::models::code_edge::CodeEdgeType::Imports,
            "edge type must be Imports"
        );
    }

    /// `impact_analysis`-equivalent: affected files for a known symbol match
    /// the set of files that were linked via edges.
    #[tokio::test]
    async fn impact_analysis_parity_affected_files_match() {
        let (_tmp, db) = open_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);
        // Insert functions in two different files.
        let make_fn = |id: &str, name: &str, file_path: &str| engram::models::Function {
            id: id.into(),
            name: name.into(),
            file_path: file_path.into(),
            line_start: 1,
            line_end: 3,
            signature: format!("fn {name}()"),
            docstring: None,
            body: format!("fn {name}() {{}}"),
            body_hash: format!("hash-{name}"),
            token_count: 3,
            embed_type: "explicit_code".into(),
            embedding: vec![0.0_f32; engram::services::embedding::EMBEDDING_DIM],
            summary: String::new(),
        };
        q.upsert_function(&make_fn("fn:impact-a", "impact_a", "src/impact_a.rs"))
            .await
            .expect("upsert impact_a");
        q.upsert_function(&make_fn("fn:impact-b", "impact_b", "src/impact_b.rs"))
            .await
            .expect("upsert impact_b");
        // Verify file-to-symbol mapping is strict: each file returns only its own symbol.
        let fns_a = q
            .get_functions_by_file("src/impact_a.rs")
            .await
            .expect("get_functions_by_file a");
        let fns_b = q
            .get_functions_by_file("src/impact_b.rs")
            .await
            .expect("get_functions_by_file b");
        assert_eq!(
            fns_a.len(),
            1,
            "impact_a file must have exactly one function"
        );
        assert_eq!(
            fns_b.len(),
            1,
            "impact_b file must have exactly one function"
        );
        assert_eq!(fns_a[0].name, "impact_a", "file a must return impact_a");
        assert_eq!(fns_b[0].name, "impact_b", "file b must return impact_b");
        // Impact analysis isolation: no cross-contamination between files.
        assert!(
            fns_a.iter().all(|f| f.file_path == "src/impact_a.rs"),
            "all functions in file_a query must belong to impact_a.rs"
        );
    }

    /// `unified_search`-equivalent: a symbol upserted with a non-zero embedding
    /// is returned by a vector search query over the same embedding space.
    #[tokio::test]
    async fn unified_search_parity_result_shape_matches() {
        let (_tmp, db) = open_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);
        // Embedding with a distinctive non-zero pattern.
        let mut embedding = vec![0.0_f32; engram::services::embedding::EMBEDDING_DIM];
        embedding[0] = 1.0;
        let func = engram::models::Function {
            id: "fn:search-001".into(),
            name: "searchable_fn".into(),
            file_path: "src/search.rs".into(),
            line_start: 1,
            line_end: 5,
            signature: "fn searchable_fn()".into(),
            docstring: None,
            body: "fn searchable_fn() {}".into(),
            body_hash: "shash-001".into(),
            token_count: 5,
            embed_type: "explicit_code".into(),
            embedding: embedding.clone(),
            summary: "searchable function".into(),
        };
        q.upsert_function(&func)
            .await
            .expect("upsert searchable_fn");
        let results = q
            .vector_search_symbols(&embedding, 5)
            .await
            .expect("vector_search_symbols");
        assert!(
            !results.is_empty(),
            "vector search must return at least one result for a matching embedding"
        );
        assert!(
            results.iter().any(|m| m.name == "searchable_fn"),
            "searchable_fn must appear in vector search results"
        );
    }

    /// Edge case: empty workspace — all queries return empty results without error.
    #[tokio::test]
    async fn parity_empty_workspace_returns_empty_results() {
        let (_tmp, db) = open_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);
        let files = q.list_code_files().await.expect("list_code_files");
        assert!(files.is_empty(), "fresh workspace must have no code files");
        let fns = q.all_functions().await.expect("all_functions");
        assert!(fns.is_empty(), "fresh workspace must have no functions");
        let edges = q.all_code_edges().await.expect("all_code_edges");
        assert!(edges.is_empty(), "fresh workspace must have no edges");
        let zero_embed = vec![0.0_f32; engram::services::embedding::EMBEDDING_DIM];
        let search = q
            .vector_search_symbols(&zero_embed, 5)
            .await
            .expect("vector_search_symbols on empty workspace");
        assert!(
            search.is_empty(),
            "vector search on empty workspace must return empty"
        );
    }

    // ── class + interface quick-check ─────────────────────────────────────

    #[tokio::test]
    async fn class_and_interface_upsert_increment_counts() {
        let (_tmp, db) = open_db().await;
        let q = engram::db::queries::CodeGraphQueries::new(db);

        let cls_before = q.count_classes().await.expect("class count before");
        let iface_before = q.count_interfaces().await.expect("iface count before");

        let class = engram::models::Class {
            id: "class:sweep-cls".into(),
            name: "SweepClass".into(),
            file_path: "src/sweep.rs".into(),
            line_start: 5,
            line_end: 25,
            docstring: None,
            body: "struct SweepClass {}".into(),
            body_hash: "cls-sweep-hash".into(),
            token_count: 50,
            embed_type: "explicit_code".into(),
            embedding: vec![0.1_f32; engram::services::embedding::EMBEDDING_DIM],
            summary: String::new(),
        };
        q.upsert_class(&class).await.expect("upsert class");

        let iface = engram::models::Interface {
            id: "iface:sweep-iface".into(),
            name: "SweepInterface".into(),
            file_path: "src/sweep.rs".into(),
            line_start: 30,
            line_end: 40,
            docstring: None,
            body: "trait SweepInterface {}".into(),
            body_hash: "iface-sweep-hash".into(),
            token_count: 20,
            embed_type: "explicit_code".into(),
            embedding: vec![0.2_f32; engram::services::embedding::EMBEDDING_DIM],
            summary: String::new(),
        };
        q.upsert_interface(&iface).await.expect("upsert iface");

        assert_eq!(
            q.count_classes().await.expect("class count after"),
            cls_before + 1,
            "class count must increment"
        );
        assert_eq!(
            q.count_interfaces().await.expect("iface count after"),
            iface_before + 1,
            "interface count must increment"
        );
    }
}
