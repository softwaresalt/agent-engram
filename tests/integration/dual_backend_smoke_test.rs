//! Dual-backend smoke test (U1.4).
//!
//! Verifies that code-graph count queries compile and run correctly under both
//! feature-gated database backends:
//!
//! * **`surreal-backend`** (default): an empty workspace returns `0` for all counts.
//! * **`cozo-backend`** (`--no-default-features --features cozo-backend`): `connect_db`
//!   succeeds and count queries return `0` on a fresh DB.

// Include the dual_backend helper macros from tests/helpers/.
#[path = "../helpers/dual_backend.rs"]
mod dual_backend;

use engram::db::connect_db;

// ── Helpers ────────────────────────────────────────────────────────────────

/// Create a minimal workspace tempdir with `.git/HEAD` and `.engram/`.
fn make_workspace() -> (tempfile::TempDir, std::path::PathBuf) {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(workspace.path().join(".git")).expect("create .git");
    let data_dir = workspace.path().join(".engram");
    std::fs::create_dir_all(&data_dir).expect("create .engram");
    (workspace, data_dir)
}

// ── Smoke tests ────────────────────────────────────────────────────────────

/// Smoke: `connect_db` on an empty workspace returns `Ok(db)` under both backends.
#[tokio::test]
async fn smoke_connect_db() {
    let (_workspace, data_dir) = make_workspace();

    let result = connect_db(&data_dir, "main").await;

    #[cfg(feature = "surreal-backend")]
    assert!(
        result.is_ok(),
        "connect_db should succeed under surreal-backend, got: {result:?}"
    );

    #[cfg(feature = "cozo-backend")]
    assert!(
        result.is_ok(),
        "connect_db should succeed under cozo-backend, got: {result:?}"
    );
}

/// Smoke: `count_code_files` on an empty workspace.
///
/// Under `surreal-backend`: returns `Ok(0)`.
/// Under `cozo-backend`: connects and counts, returns `Ok(0)`.
#[tokio::test]
async fn smoke_empty_count_code_files() {
    let (_workspace, data_dir) = make_workspace();

    #[cfg(feature = "surreal-backend")]
    {
        use engram::db::queries::CodeGraphQueries;

        let db = connect_db(&data_dir, "main")
            .await
            .expect("connect_db should succeed");
        let cg = CodeGraphQueries::new(db);
        assert_empty_count_or_stub!(cg.count_code_files().await);
    }

    #[cfg(feature = "cozo-backend")]
    {
        use engram::db::queries::CodeGraphQueries;
        let db = connect_db(&data_dir, "main")
            .await
            .expect("connect_db should succeed under cozo-backend");
        let cg = CodeGraphQueries::new(db);
        let count = cg
            .count_code_files()
            .await
            .expect("count_code_files should succeed");
        assert_eq!(count, 0_u64, "fresh DB must have zero code files");
    }
}

/// Smoke: `count_functions` on an empty workspace.
#[tokio::test]
async fn smoke_empty_count_functions() {
    let (_workspace, data_dir) = make_workspace();

    #[cfg(feature = "surreal-backend")]
    {
        use engram::db::queries::CodeGraphQueries;

        let db = connect_db(&data_dir, "main")
            .await
            .expect("connect_db should succeed");
        let cg = CodeGraphQueries::new(db);
        assert_empty_count_or_stub!(cg.count_functions().await);
    }

    #[cfg(feature = "cozo-backend")]
    {
        use engram::db::queries::CodeGraphQueries;
        let db = connect_db(&data_dir, "main")
            .await
            .expect("connect_db should succeed under cozo-backend");
        let cg = CodeGraphQueries::new(db);
        let count = cg
            .count_functions()
            .await
            .expect("count_functions should succeed");
        assert_eq!(count, 0_u64, "fresh DB must have zero functions");
    }
}
