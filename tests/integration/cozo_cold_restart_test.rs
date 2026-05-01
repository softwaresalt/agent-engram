//! Integration test: `CozoDB` cold-restart round-trip (Task 001.006.004-T — U5.4).
//!
//! Verifies that the `CozoDB` backend survives a cold restart: code graph data
//! dehydrated to JSONL files survives a data-directory deletion and is fully
//! restored after reconnect + hydrate.
//!
//! Run with the default features (or explicitly):
//!   `cargo test --test integration_cozo_cold_restart`

#[cfg(feature = "cozo-backend")]
mod cold_restart {
    use tempfile::TempDir;

    /// Connect the `CozoDB` backend and return a query handle.
    async fn open_cg(tmp: &TempDir, branch: &str) -> engram::db::queries::CodeGraphQueries {
        let db = engram::db::connect_db(tmp.path(), branch)
            .await
            .expect("connect_db must succeed");
        engram::db::queries::CodeGraphQueries::new(db)
    }

    /// Simulate a cold restart: close the current DB handle, delete the `CozoDB`
    /// on-disk files, then reconnect and hydrate from the JSONL dehydration artifacts.
    ///
    /// Takes `prev_cg` by value so the `Arc<cozo::DbInstance>` is dropped before
    /// `remove_dir_all` — required on Windows where `SQLite` holds the file open.
    async fn cold_restart(
        prev_cg: engram::db::queries::CodeGraphQueries,
        tmp: &TempDir,
        branch: &str,
    ) -> engram::db::queries::CodeGraphQueries {
        // Drop the previous handle first to release the SQLite file lock.
        drop(prev_cg);
        // Remove the CozoDB data directory to simulate process restart with empty DB.
        let cozo_dir = tmp.path().join("cozo");
        if cozo_dir.exists() {
            std::fs::remove_dir_all(&cozo_dir).expect("remove cozo dir for restart");
        }
        // Fresh connection creates an empty schema.
        let cg2 = open_cg(tmp, branch).await;
        // Restore graph from JSONL dehydration artifacts.
        engram::services::hydration::hydrate_code_graph(tmp.path(), tmp.path(), branch, &cg2)
            .await
            .expect("hydrate_code_graph must succeed after cold restart");
        cg2
    }

    fn sample_code_file(id: &str, path: &str) -> engram::models::CodeFile {
        engram::models::CodeFile {
            id: id.into(),
            path: path.into(),
            language: "rust".into(),
            size_bytes: 512,
            content_hash: "hash-cold-01".into(),
            last_indexed_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    /// Prove that a `code_file` node survives the full cold-restart cycle:
    /// upsert → dehydrate → delete `CozoDB` files → reconnect → hydrate → verify node present.
    #[tokio::test]
    async fn code_file_survives_cold_restart() {
        let branch = "restart-file";
        let tmp = TempDir::new().expect("tempdir");
        let cg = open_cg(&tmp, branch).await;
        let file = sample_code_file("file:cold-001", "src/cold.rs");
        cg.upsert_code_file(&file).await.expect("upsert_code_file");
        engram::services::dehydration::dehydrate_code_graph(&cg, tmp.path(), branch)
            .await
            .expect("dehydrate_code_graph");

        let cg2 = cold_restart(cg, &tmp, branch).await;
        let retrieved = cg2
            .get_code_file_by_path("src/cold.rs")
            .await
            .expect("get_code_file_by_path after restart");
        assert!(retrieved.is_some(), "code_file must survive cold restart");
        assert_eq!(
            retrieved.unwrap().path,
            "src/cold.rs",
            "path must be preserved across restart"
        );
    }

    /// Prove that a `function` node survives the full cold-restart cycle.
    #[tokio::test]
    async fn function_survives_cold_restart() {
        let branch = "restart-fn";
        let tmp = TempDir::new().expect("tempdir");
        let cg = open_cg(&tmp, branch).await;
        let func = engram::models::Function {
            id: "fn:cold-fn-001".into(),
            name: "cold_fn".into(),
            file_path: "src/cold.rs".into(),
            line_start: 1,
            line_end: 10,
            signature: "fn cold_fn()".into(),
            docstring: None,
            body: "fn cold_fn() {}".into(),
            body_hash: "fnhash-cold-01".into(),
            token_count: 10,
            embed_type: "explicit_code".into(),
            embedding: vec![0.0_f32; engram::services::embedding::EMBEDDING_DIM],
            summary: String::new(),
        };
        cg.upsert_function(&func).await.expect("upsert_function");
        engram::services::dehydration::dehydrate_code_graph(&cg, tmp.path(), branch)
            .await
            .expect("dehydrate_code_graph");

        let cg2 = cold_restart(cg, &tmp, branch).await;
        let retrieved = cg2
            .get_function_by_name("cold_fn")
            .await
            .expect("get_function_by_name after restart");
        assert!(retrieved.is_some(), "function must survive cold restart");
        assert_eq!(
            retrieved.unwrap().name,
            "cold_fn",
            "function name must be preserved"
        );
    }

    /// Prove that graph edges survive the cold-restart round-trip.
    #[tokio::test]
    async fn edge_survives_cold_restart() {
        let branch = "restart-edge";
        let tmp = TempDir::new().expect("tempdir");
        let cg = open_cg(&tmp, branch).await;
        let file_a = sample_code_file("file:edge-a", "src/a.rs");
        let file_b = sample_code_file("file:edge-b", "src/b.rs");
        cg.upsert_code_file(&file_a).await.expect("upsert file_a");
        cg.upsert_code_file(&file_b).await.expect("upsert file_b");
        cg.create_imports_edge("file:edge-a", "file:edge-b", "crate::b")
            .await
            .expect("create_imports_edge");
        engram::services::dehydration::dehydrate_code_graph(&cg, tmp.path(), branch)
            .await
            .expect("dehydrate_code_graph");

        let cg2 = cold_restart(cg, &tmp, branch).await;
        let edges = cg2
            .all_code_edges()
            .await
            .expect("all_code_edges after restart");
        assert!(!edges.is_empty(), "edges must survive cold restart");
        assert!(
            edges
                .iter()
                .any(|e| e.from == "file:edge-a" && e.to == "file:edge-b"),
            "imports edge from file:edge-a to file:edge-b must be present"
        );
    }

    /// Prove that dehydration of an empty graph does not write stale files and
    /// that a subsequent hydrate from an empty data dir produces an empty graph.
    #[tokio::test]
    async fn empty_graph_cold_restart_produces_empty_graph() {
        let branch = "restart-empty";
        let tmp = TempDir::new().expect("tempdir");
        let cg = open_cg(&tmp, branch).await;
        // Dehydrate an empty graph — nothing should be written.
        engram::services::dehydration::dehydrate_code_graph(&cg, tmp.path(), branch)
            .await
            .expect("dehydrate empty graph");

        let cg2 = cold_restart(cg, &tmp, branch).await;
        let files = cg2
            .list_code_files()
            .await
            .expect("list_code_files after empty restart");
        assert!(
            files.is_empty(),
            "empty graph must remain empty after cold restart"
        );
        let edges = cg2
            .all_code_edges()
            .await
            .expect("all_code_edges after empty restart");
        assert!(
            edges.is_empty(),
            "no edges expected after empty cold restart"
        );
    }
}
