//! Integration tests for workspace file hash tracking (T039–T040).
//!
//! Validates that [`engram::services::file_tracker`] correctly detects offline
//! changes by comparing stored file hashes against current on-disk content.
//!
//! Tests: S067–S074.

use std::fs;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::services::file_tracker::{FileChangeKind, detect_offline_changes, record_file_hash};
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Derive a stable test data-dir + branch name from the workspace path.
fn test_db_params(path: &std::path::Path) -> (std::path::PathBuf, String) {
    use sha2::{Digest, Sha256};
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (
        std::env::temp_dir().join("engram-file-tracker-test"),
        branch,
    )
}

/// Open an isolated `SurrealDB` for `ws`, returning queries handle.
async fn open_queries(ws: &TempDir) -> CodeGraphQueries {
    let (data_dir, branch) = test_db_params(ws.path());
    let db = connect_db(&data_dir, &branch)
        .await
        .expect("connect_db must succeed");
    CodeGraphQueries::new(db)
}

// ── T039: Offline change detection ───────────────────────────────────────────

/// S067: `detect_offline_changes` returns empty when hashes match on-disk state.
#[tokio::test]
async fn s067_no_changes_when_hashes_current() {
    let ws = TempDir::new().expect("tempdir");
    let file = ws.path().join("readme.txt");
    fs::write(&file, b"hello world").expect("write file");

    let queries = open_queries(&ws).await;
    record_file_hash("readme.txt", &file, &queries)
        .await
        .expect("record_file_hash");

    let changes = detect_offline_changes(ws.path(), &queries)
        .await
        .expect("detect_offline_changes");

    assert!(
        changes.is_empty(),
        "expected no changes when hash is current; got: {changes:?}"
    );
}

/// S068: `detect_offline_changes` returns `Modified` when file content changed
/// since the hash was recorded.
#[tokio::test]
async fn s068_modified_when_content_changed() {
    let ws = TempDir::new().expect("tempdir");
    let file = ws.path().join("src.rs");
    fs::write(&file, b"fn old() {}").expect("write initial");

    let queries = open_queries(&ws).await;
    record_file_hash("src.rs", &file, &queries)
        .await
        .expect("record_file_hash");

    // Simulate offline change.
    fs::write(&file, b"fn new() {}").expect("write updated");

    let changes = detect_offline_changes(ws.path(), &queries)
        .await
        .expect("detect_offline_changes");

    let modified: Vec<_> = changes
        .iter()
        .filter(|c| c.path == "src.rs" && c.kind == FileChangeKind::Modified)
        .collect();

    assert_eq!(
        modified.len(),
        1,
        "expected one Modified change for src.rs; got: {changes:?}"
    );
}

/// S069: `detect_offline_changes` returns `Added` for a file present on disk
/// but with no stored hash.
#[tokio::test]
async fn s069_added_when_no_stored_hash() {
    let ws = TempDir::new().expect("tempdir");
    let file = ws.path().join("new_file.rs");
    fs::write(&file, b"pub fn new() {}").expect("write file");

    // Do NOT record a hash for this file.
    let queries = open_queries(&ws).await;

    let changes = detect_offline_changes(ws.path(), &queries)
        .await
        .expect("detect_offline_changes");

    let added: Vec<_> = changes
        .iter()
        .filter(|c| c.path == "new_file.rs" && c.kind == FileChangeKind::Added)
        .collect();

    assert_eq!(
        added.len(),
        1,
        "expected one Added change for new_file.rs; got: {changes:?}"
    );
}

/// S070: `detect_offline_changes` returns `Deleted` for a hash record whose
/// file no longer exists on disk.
#[tokio::test]
async fn s070_deleted_when_file_removed() {
    let ws = TempDir::new().expect("tempdir");
    let file = ws.path().join("gone.rs");
    fs::write(&file, b"fn gone() {}").expect("write file");

    let queries = open_queries(&ws).await;
    record_file_hash("gone.rs", &file, &queries)
        .await
        .expect("record_file_hash");

    // Simulate offline deletion.
    fs::remove_file(&file).expect("remove file");

    let changes = detect_offline_changes(ws.path(), &queries)
        .await
        .expect("detect_offline_changes");

    let deleted: Vec<_> = changes
        .iter()
        .filter(|c| c.path == "gone.rs" && c.kind == FileChangeKind::Deleted)
        .collect();

    assert_eq!(
        deleted.len(),
        1,
        "expected one Deleted change for gone.rs; got: {changes:?}"
    );
}

/// S071: After recording a hash, a second `detect_offline_changes` call with
/// the same file returns no changes.
#[tokio::test]
async fn s071_no_changes_after_record() {
    let ws = TempDir::new().expect("tempdir");
    let file = ws.path().join("stable.rs");
    fs::write(&file, b"fn stable() {}").expect("write file");

    let queries = open_queries(&ws).await;
    record_file_hash("stable.rs", &file, &queries)
        .await
        .expect("record_file_hash");

    // Detect once — should be empty since we just recorded.
    let first = detect_offline_changes(ws.path(), &queries)
        .await
        .expect("detect_offline_changes first");
    assert!(
        first.iter().all(|c| c.path != "stable.rs"),
        "stable.rs should have no changes after recording; got: {first:?}"
    );
}

// ── T040: Hash computation ────────────────────────────────────────────────────

/// S072: `compute_file_hash` returns the same value for identical content.
#[tokio::test]
async fn s072_compute_hash_consistent() {
    use engram::services::file_tracker::compute_file_hash;

    let ws = TempDir::new().expect("tempdir");
    let file = ws.path().join("consistent.txt");
    fs::write(&file, b"deterministic content").expect("write");

    let h1 = compute_file_hash(&file).expect("hash 1");
    let h2 = compute_file_hash(&file).expect("hash 2");

    assert_eq!(h1, h2, "same content must produce same hash");
}

/// S073: `compute_file_hash` returns different values for different content.
#[tokio::test]
async fn s073_compute_hash_different_content() {
    use engram::services::file_tracker::compute_file_hash;

    let ws = TempDir::new().expect("tempdir");
    let file_a = ws.path().join("a.txt");
    let file_b = ws.path().join("b.txt");
    fs::write(&file_a, b"content alpha").expect("write a");
    fs::write(&file_b, b"content beta").expect("write b");

    let ha = compute_file_hash(&file_a).expect("hash a");
    let hb = compute_file_hash(&file_b).expect("hash b");

    assert_ne!(ha, hb, "different content must produce different hashes");
}

/// S074: Excluded paths (`.git/`, `.engram/`, `target/`, `node_modules/`) are
/// not reported as `Added` by `detect_offline_changes`.
#[tokio::test]
async fn s074_excluded_dirs_not_reported() {
    let ws = TempDir::new().expect("tempdir");

    // Write files in every excluded directory.
    for dir in [".git", ".engram", "target", "node_modules"] {
        let d = ws.path().join(dir);
        fs::create_dir_all(&d).expect("create excluded dir");
        fs::write(d.join("ignored.txt"), b"ignored").expect("write in excluded dir");
    }
    // Write one real file.
    fs::write(ws.path().join("real.rs"), b"fn real() {}").expect("write real file");

    let queries = open_queries(&ws).await;
    let changes = detect_offline_changes(ws.path(), &queries)
        .await
        .expect("detect_offline_changes");

    // Only real.rs should be reported as Added.
    for change in &changes {
        let path_str = change.path.replace('\\', "/");
        assert!(
            !path_str.starts_with(".git/")
                && !path_str.starts_with(".engram/")
                && !path_str.starts_with("target/")
                && !path_str.starts_with("node_modules/"),
            "excluded path should not be reported: {}",
            change.path
        );
    }

    let added: Vec<_> = changes.iter().filter(|c| c.path == "real.rs").collect();
    assert_eq!(added.len(), 1, "real.rs should be reported as Added");
}
