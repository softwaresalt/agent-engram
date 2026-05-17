//! Integration test for `sync_workspace` deletion correctness — 060.001-T.
//!
//! Verifies that when a tracked source file is deleted from the workspace and
//! `sync_workspace` is run again, all `CozoDB` symbol and edge records for that
//! file (function nodes, code-file record, and content-hash entry) are removed.
//!
//! This is the Option C prerequisite from deliberation `008-D` before any
//! branch-DB seeding implementation can proceed safely.

use std::fs;
use std::path::Path;

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;

fn write_sample_file(dir: &Path, rel_path: &str, content: &str) {
    let full = dir.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
}

fn test_db_params(path: &Path) -> (std::path::PathBuf, String) {
    use sha2::{Digest, Sha256};
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

/// S-DEL-01: `sync_workspace` removes `CozoDB` records for a deleted source file.
///
/// Audit result for `008-D` open question 1: deletion IS correct.
/// The function node, code-file record, and file-hash entry are all removed
/// from the branch DB after the file is deleted and the workspace is
/// re-synced.
#[test]
async fn sync_workspace_removes_symbols_and_file_record_after_file_deletion() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    // ── Phase 1: create a workspace with one file containing a function ──
    write_sample_file(ws, "src/to_delete.rs", "pub fn will_be_gone() {}\n");
    write_sample_file(ws, "src/permanent.rs", "pub fn stays_forever() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    // Full initial index.
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("initial index");

    // ── Phase 2: verify symbol EXISTS before deletion ────────────────
    {
        let db = connect_db(&data_dir, &branch)
            .await
            .expect("connect_db before deletion");
        let queries = CodeGraphQueries::new(db);

        let file_record = queries
            .get_code_file_by_path("src/to_delete.rs")
            .await
            .expect("query file record before deletion");
        assert!(
            file_record.is_some(),
            "code-file record must exist before deletion"
        );

        let functions = queries
            .get_functions_by_file("src/to_delete.rs")
            .await
            .expect("query functions before deletion");
        assert!(
            !functions.is_empty(),
            "function symbols must exist before deletion (found {} functions)",
            functions.len()
        );
    }

    // ── Phase 3: delete the file from the workspace ──────────────────
    fs::remove_file(ws.join("src/to_delete.rs")).expect("remove file from workspace");

    // ── Phase 4: re-sync ─────────────────────────────────────────────
    let result = code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync_workspace after file deletion");

    assert_eq!(
        result.files_deleted, 1,
        "sync_workspace must report exactly 1 deleted file"
    );

    // ── Phase 5: verify symbol is ABSENT after deletion ──────────────
    {
        let db = connect_db(&data_dir, &branch)
            .await
            .expect("connect_db after deletion");
        let queries = CodeGraphQueries::new(db);

        let file_record = queries
            .get_code_file_by_path("src/to_delete.rs")
            .await
            .expect("query file record after deletion");
        assert!(
            file_record.is_none(),
            "code-file record must be absent after deletion and re-sync"
        );

        let functions = queries
            .get_functions_by_file("src/to_delete.rs")
            .await
            .expect("query functions after deletion");
        assert!(
            functions.is_empty(),
            "function symbols must be absent after deletion and re-sync (found {} lingering)",
            functions.len()
        );
    }

    // ── Phase 6: verify permanent file is unaffected ─────────────────
    {
        let db = connect_db(&data_dir, &branch)
            .await
            .expect("connect_db verify permanent file");
        let queries = CodeGraphQueries::new(db);

        let permanent_record = queries
            .get_code_file_by_path("src/permanent.rs")
            .await
            .expect("query permanent file record");
        assert!(
            permanent_record.is_some(),
            "permanent file record must remain after deleting an unrelated file"
        );

        let permanent_functions = queries
            .get_functions_by_file("src/permanent.rs")
            .await
            .expect("query permanent functions");
        assert!(
            !permanent_functions.is_empty(),
            "permanent file functions must remain after deleting an unrelated file"
        );
    }
}
