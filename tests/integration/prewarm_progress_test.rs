//! Integration tests for pre-warm progress reporting during index and sync.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use tokio::test;

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

#[test]
async fn index_workspace_with_progress_reports_totals_and_completion() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(ws, "src/lib.rs", "pub fn alpha() {}\n");
    write_sample_file(ws, "src/main.rs", "fn main() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    let progress = Arc::new(Mutex::new(Vec::<(u64, u64)>::new()));

    let mut on_progress = {
        let progress = Arc::clone(&progress);
        move |completed: u64, total: u64| {
            progress
                .lock()
                .expect("progress lock")
                .push((completed, total));
        }
    };

    let result = code_graph::index_workspace_with_progress(
        ws,
        &data_dir,
        &branch,
        &config,
        false,
        Some(&mut on_progress),
    )
    .await
    .expect("indexing should succeed");

    assert_eq!(result.files_parsed, 2, "both files should index");

    let snapshots = progress.lock().expect("progress lock");
    assert!(
        !snapshots.is_empty(),
        "progress callback should receive at least one snapshot"
    );
    assert_eq!(
        snapshots.first().copied(),
        Some((0, 2)),
        "first snapshot should announce total work"
    );
    assert_eq!(
        snapshots.last().copied(),
        Some((2, 2)),
        "last snapshot should report completion"
    );
}

#[test]
async fn sync_workspace_with_progress_counts_deleted_current_and_completed_work() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(ws, "src/kept.rs", "pub fn kept() {}\n");
    write_sample_file(ws, "src/deleted.rs", "pub fn deleted() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("initial index");

    write_sample_file(ws, "src/kept.rs", "pub fn kept() { let _x = 1; }\n");
    write_sample_file(ws, "src/added.rs", "pub fn added() {}\n");
    fs::remove_file(ws.join("src/deleted.rs")).expect("remove deleted file");

    let progress = Arc::new(Mutex::new(Vec::<(u64, u64)>::new()));
    let mut on_progress = {
        let progress = Arc::clone(&progress);
        move |completed: u64, total: u64| {
            progress
                .lock()
                .expect("progress lock")
                .push((completed, total));
        }
    };

    let result = code_graph::sync_workspace_with_progress(
        ws,
        &data_dir,
        &branch,
        &config,
        Some(&mut on_progress),
    )
    .await
    .expect("sync should succeed");

    assert_eq!(result.files_deleted, 1, "deleted file should be counted");
    assert_eq!(result.files_modified, 1, "modified file should be counted");
    assert_eq!(result.files_added, 1, "added file should be counted");

    let snapshots = progress.lock().expect("progress lock");
    assert!(
        !snapshots.is_empty(),
        "progress callback should receive snapshots"
    );
    assert_eq!(
        snapshots.first().copied(),
        Some((0, 3)),
        "first snapshot should include deleted and current-file work"
    );
    assert_eq!(
        snapshots.last().copied(),
        Some((3, 3)),
        "last snapshot should report all work complete"
    );
}

#[test]
async fn sync_workspace_tracks_oversized_files_without_errors() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(ws, "src/kept.rs", "pub fn kept() {}\n");

    let config = CodeGraphConfig {
        max_file_size_bytes: 64,
        ..CodeGraphConfig::default()
    };
    let (data_dir, branch) = test_db_params(ws);

    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("initial index");

    write_sample_file(
        ws,
        "src/oversized.rs",
        &"x".repeat(usize::try_from(config.max_file_size_bytes + 1).expect("limit fits usize")),
    );

    let result = code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync should succeed");

    assert_eq!(
        result.files_added, 0,
        "oversized file must not be indexed as added"
    );
    assert_eq!(
        result.files_unchanged, 1,
        "existing unchanged file should still be counted"
    );
    assert_eq!(
        result.oversized_files_skipped, 1,
        "oversized file must increment oversized_files_skipped"
    );
    assert!(
        result.errors.is_empty(),
        "oversized files must not appear in sync errors"
    );
}
