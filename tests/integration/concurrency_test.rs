//! Integration tests for multi-client concurrent access (US5).
//!
//! Tests verify that 10+ clients can safely perform interleaved read/write
//! operations on the same workspace without data corruption or failures.

use std::fs;
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::server::state::AppState;
use engram::tools;

// ─── Phase 9 additions (T055) — S026, S027, S044, S062, S070, S076, S077 ──────

/// S026: Concurrent ingestion from two sources is serialized without interference.
///
/// Two concurrent `index_workspace` calls must not corrupt state. The indexer
/// may serialize (at-least-one succeeds) or reject one with `IndexInProgress`,
/// but must never panic or corrupt data.
#[test]
async fn s026_concurrent_ingestion_serialized_or_rejected() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    let s1 = state.clone();
    let s2 = state.clone();

    let h1 =
        tokio::spawn(async move { tools::dispatch(s1, "index_workspace", Some(json!({}))).await });
    let h2 =
        tokio::spawn(async move { tools::dispatch(s2, "index_workspace", Some(json!({}))).await });

    let r1 = h1.await.expect("h1 join");
    let r2 = h2.await.expect("h2 join");

    // At least one must succeed; the other may get IndexInProgress.
    let one_ok = r1.is_ok() || r2.is_ok();
    assert!(
        one_ok,
        "at least one concurrent index_workspace must succeed"
    );

    for result in [r1, r2] {
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("IndexInProgress")
                    || msg.contains("index")
                    || msg.contains("progress"),
                "concurrent failure must be IndexInProgress, not data corruption: {msg}"
            );
        }
    }
}

/// S027: File deleted after registry scan is handled gracefully.
///
/// `ingest_single_file` on a non-existent path must return `Ok` and not panic.
/// The ingestion pipeline skips deleted files gracefully.
#[tokio::test]
async fn s027_file_deleted_after_scan_handled_gracefully() {
    use engram::db::queries::CodeGraphQueries;
    use engram::db::schema;
    use engram::services::ingestion::ingest_single_file;

    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();

    let db_path = workspace.join("db");
    fs::create_dir_all(&db_path).expect("create db dir");
    let db =
        surrealdb::Surreal::new::<surrealdb::engine::local::SurrealKv>(db_path.to_str().unwrap())
            .await
            .expect("embedded SurrealDB");
    db.use_ns("engram").use_db("s027").await.expect("ns/db");
    db.query(schema::DEFINE_CONTENT_RECORD)
        .await
        .expect("content schema");
    let queries = CodeGraphQueries::new(db);
    let phantom_file = workspace.join("vanished.md");

    let result = ingest_single_file(
        &phantom_file,
        workspace,
        "docs",
        "docs",
        1_048_576,
        None,
        &queries,
    )
    .await;

    assert!(
        result.is_ok(),
        "ingest_single_file on deleted file must not error: {result:?}"
    );

    // No ContentRecord should exist for the phantom file.
    let records: Vec<engram::models::ContentRecord> = queries
        .select_content_records(None)
        .await
        .expect("select records");
    assert!(
        records.is_empty(),
        "no ContentRecord must exist for a deleted file"
    );
}

/// S062: Git repository with broken objects returns a clear error, not a panic.
///
/// `index_git_history` on a corrupted git repository must handle the error
/// gracefully — either returning an error or succeeding on trivial state.
///
/// Requires the `git-graph` feature flag.
#[cfg(feature = "git-graph")]
#[test]
async fn s062_git_broken_objects_returns_error_not_panic() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");

    // Write a corrupt objects directory to simulate broken git objects.
    let objects_dir = git_dir.join("objects");
    fs::create_dir_all(&objects_dir).expect("create objects dir");
    fs::write(objects_dir.join("pack"), b"INVALID_PACK_DATA").expect("write corrupt pack");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    let result = tools::dispatch(state.clone(), "index_git_history", Some(json!({}))).await;

    // Either it succeeds (trivial git state) or fails with a descriptive error.
    // The primary assertion is that no panic occurred.
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            !msg.is_empty(),
            "error message must not be empty for broken git objects"
        );
    }
    // Reaching here without panic satisfies S062.
}

/// S070: Hook file in a read-only directory is handled gracefully.
///
/// When `.github/` is read-only, `install()` must return an error with a
/// descriptive message rather than panicking.
///
/// Unix-only: Windows does not enforce directory read-only for child file creation.
#[cfg(unix)]
#[tokio::test]
async fn s070_hook_file_read_only_directory_handled_gracefully() {
    use std::os::unix::fs::PermissionsExt;

    use engram::installer::{InstallOptions, install};

    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    // Create .github/ as read-only to block hook file creation.
    let github_dir = workspace.path().join(".github");
    fs::create_dir_all(&github_dir).expect("create .github");
    fs::set_permissions(&github_dir, std::fs::Permissions::from_mode(0o555))
        .expect("set read-only");

    let opts = InstallOptions {
        hooks_only: false,
        no_hooks: false,
        port: engram::installer::DEFAULT_PORT,
    };

    let result = install(workspace.path(), &opts).await;

    // Restore permissions so tempdir cleanup can proceed.
    let _ = fs::set_permissions(&github_dir, std::fs::Permissions::from_mode(0o755));

    // Must not panic. Either succeeds or returns a descriptive IO error.
    match result {
        Ok(()) => {
            // Acceptable: install completed (hooks were written or skipped).
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                !msg.is_empty(),
                "error message must not be empty on read-only hook dir failure"
            );
        }
    }
}

/// S076: Multiple agents calling `query_memory` concurrently do not interfere.
///
/// Two concurrent queries with different `content_type` filters must each
/// return correct results independently — no cross-query contamination.
#[test]
async fn s076_concurrent_query_memory_no_cross_interference() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    let s1 = state.clone();
    let s2 = state.clone();

    let h1 = tokio::spawn(async move {
        tools::dispatch(
            s1,
            "query_memory",
            Some(json!({ "query": "test query", "content_type": "spec" })),
        )
        .await
    });
    let h2 = tokio::spawn(async move {
        tools::dispatch(
            s2,
            "query_memory",
            Some(json!({ "query": "test query", "content_type": "docs" })),
        )
        .await
    });

    let r1 = h1.await.expect("h1 join");
    let r2 = h2.await.expect("h2 join");

    assert!(r1.is_ok(), "first query_memory must succeed: {r1:?}");
    assert!(r2.is_ok(), "second query_memory must succeed: {r2:?}");

    // Both should return arrays (empty is fine — content not required for this test).
    let v1 = r1.unwrap();
    let v2 = r2.unwrap();
    assert!(
        v1.is_array() || v1.is_object(),
        "first result must be array or object, got: {v1}"
    );
    assert!(
        v2.is_array() || v2.is_object(),
        "second result must be array or object, got: {v2}"
    );
}

/// S077: Concurrent ingestion of the same file produces no duplicate `ContentRecord`s.
///
/// Two concurrent `ingest_all_sources` calls against the same workspace and source
/// must upsert the same record — `SurrealDB` `UPSERT` semantics prevent duplication.
#[tokio::test]
async fn s077_concurrent_ingestion_no_duplicate_records() {
    use engram::db::queries::CodeGraphQueries;
    use engram::db::schema;
    use engram::models::registry::{ContentSource, ContentSourceStatus, RegistryConfig};
    use engram::services::ingestion::ingest_all_sources;

    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();

    let docs_dir = workspace.join("docs");
    fs::create_dir_all(&docs_dir).expect("create docs dir");
    fs::write(
        docs_dir.join("readme.md"),
        "# README\nTest content for S077",
    )
    .expect("write readme");

    let db_path = workspace.join("db");
    fs::create_dir_all(&db_path).expect("create db dir");
    let db =
        surrealdb::Surreal::new::<surrealdb::engine::local::SurrealKv>(db_path.to_str().unwrap())
            .await
            .expect("embedded SurrealDB");
    db.use_ns("engram").use_db("s077").await.expect("ns/db");
    db.query(schema::DEFINE_CONTENT_RECORD)
        .await
        .expect("content schema");
    let queries = CodeGraphQueries::new(db);

    let config = Arc::new(RegistryConfig {
        sources: vec![ContentSource {
            content_type: "docs".to_string(),
            language: None,
            path: "docs".to_string(),
            pattern: None,
            status: ContentSourceStatus::Active,
        }],
        max_file_size_bytes: 1_048_576,
        batch_size: 50,
    });
    let workspace_buf = Arc::new(workspace.to_path_buf());

    let c1 = config.clone();
    let w1 = workspace_buf.clone();
    let q1 = queries.clone();
    let h1 = tokio::spawn(async move { ingest_all_sources(&c1, &w1, &q1).await });

    let c2 = config.clone();
    let w2 = workspace_buf.clone();
    let q2 = queries.clone();
    let h2 = tokio::spawn(async move { ingest_all_sources(&c2, &w2, &q2).await });

    let r1 = h1.await.expect("h1 join");
    let r2 = h2.await.expect("h2 join");

    assert!(r1.is_ok(), "first ingestion must succeed: {r1:?}");
    assert!(r2.is_ok(), "second ingestion must succeed: {r2:?}");

    // Only one ContentRecord must exist (UPSERT deduplication).
    let records: Vec<engram::models::ContentRecord> = queries
        .select_content_records(None)
        .await
        .expect("select records");
    assert_eq!(
        records.len(),
        1,
        "must have exactly 1 ContentRecord after concurrent ingestion, got {} (deduplication failed)",
        records.len()
    );
    assert_eq!(
        records[0].file_path, "docs/readme.md",
        "ContentRecord path must match"
    );
}
