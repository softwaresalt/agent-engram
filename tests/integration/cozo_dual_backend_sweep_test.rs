//! Dual-backend behavioral sweep (Task 001.003.010-T — U2.10).
//!
//! Verifies that the CozoDB backend honours the same observable contracts as
//! the SurrealDB backend: connect → bootstrap → upsert → read → count →
//! delete, all without error.
//!
//! Tests are gated on the `cozo-backend` feature.  Under the default
//! `surreal-backend` build this file compiles but contains no runnable
//! tests; under `--no-default-features --features cozo-backend` all tests
//! run and are expected to FAIL until Phase 2 is complete.
//!
//! CI invocation (advisory axis):
//!   cargo test --no-default-features --features cozo-backend
//!              --test integration_cozo_dual_backend_sweep

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
        let result =
            engram::db::cozo_backend::schema::run_schema_bootstrap(&db);
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

        let retrieved = q
            .get_code_file_by_path("src/sweep.rs")
            .await
            .expect("get");
        assert!(retrieved.is_some(), "file must be found after upsert");

        q.delete_code_file("src/sweep.rs").await.expect("delete");
        let after_delete = q.count_code_files().await.expect("count after delete");
        assert_eq!(after_delete, before, "count must return to baseline after delete");
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
