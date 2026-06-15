//! Integration tests for the PBIP project-definition file collector and
//! deletion sweep (062.004-T).
//!
//! Verifies that `collect_pbip_files` walks a PBIP project tree and finds
//! `.pbip`, `.pbir`, `.pbism`, project-definition JSON, and `definition/**/*.tmdl`
//! files while ignoring unrelated noise; and that `compute_deleted_paths`
//! reports workspace-relative paths whose backing files are gone.
//!
//! Tests: S-PFC-01..S-PFC-08

use std::fs;
use std::path::Path;

use engram::services::pbip_indexer::{collect_pbip_files, compute_deleted_paths};
use tempfile::TempDir;

fn touch(dir: &Path, rel: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent dir");
    fs::write(&path, "fixture").expect("write fixture file");
}

/// Build a representative PBIP project tree under the given root.
///
/// Mirrors the real-fixture layout in `tmp/`:
/// * `<root>/Project.pbip` workspace entry
/// * `<root>/Project.Report/definition.pbir`
/// * `<root>/Project.Report/definition/report.json`
/// * `<root>/Project.Report/definition/pages/p1/page.json`
/// * `<root>/Project.Report/definition/pages/p1/visuals/v1/visual.json`
/// * `<root>/Project.SemanticModel/definition.pbism`
/// * `<root>/Project.SemanticModel/definition/model.tmdl`
/// * `<root>/Project.SemanticModel/definition/relationships.tmdl`
/// * `<root>/Project.SemanticModel/definition/tables/Sales.tmdl`
fn write_pbip_fixture(root: &Path) {
    touch(root, "Project.pbip");
    touch(root, "Project.Report/definition.pbir");
    touch(root, "Project.Report/definition/report.json");
    touch(root, "Project.Report/definition/pages/p1/page.json");
    touch(
        root,
        "Project.Report/definition/pages/p1/visuals/v1/visual.json",
    );
    touch(root, "Project.SemanticModel/definition.pbism");
    touch(root, "Project.SemanticModel/definition/model.tmdl");
    touch(root, "Project.SemanticModel/definition/relationships.tmdl");
    touch(root, "Project.SemanticModel/definition/tables/Sales.tmdl");
}

/// S-PFC-01: `collect_pbip_files` returns an empty list for a missing directory.
#[test]
fn collect_pbip_files_returns_empty_for_missing_directory() {
    let temp = TempDir::new().expect("tempdir");
    let missing = temp.path().join("does-not-exist");
    let files = collect_pbip_files(&missing);
    assert!(files.is_empty(), "missing directory should yield no files");
}

/// S-PFC-02: `collect_pbip_files` returns an empty list for a directory with no PBIP-shaped files.
#[test]
fn collect_pbip_files_returns_empty_for_unrelated_directory() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    touch(root, "README.md");
    touch(root, "notes.txt");
    touch(root, "build.log");

    let files = collect_pbip_files(root);
    assert!(
        files.is_empty(),
        "directory without PBIP-shaped files should yield no files"
    );
}

/// S-PFC-03: `collect_pbip_files` finds the full set of PBIP file extensions
/// across the project-definition layout.
#[test]
fn collect_pbip_files_finds_all_pbip_extensions() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_pbip_fixture(root);

    let files = collect_pbip_files(root);

    let names: Vec<String> = files
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();

    assert!(
        names.iter().any(|n| n == "Project.pbip"),
        "should find .pbip"
    );
    assert!(
        names.iter().any(|n| n == "definition.pbir"),
        "should find .pbir"
    );
    assert!(
        names.iter().any(|n| n == "definition.pbism"),
        "should find .pbism"
    );
    assert!(
        names.iter().any(|n| n == "report.json"),
        "should find report.json"
    );
    assert!(
        names.iter().any(|n| n == "page.json"),
        "should find page.json"
    );
    assert!(
        names.iter().any(|n| n == "visual.json"),
        "should find visual.json"
    );
    assert!(
        names.iter().any(|n| n == "model.tmdl"),
        "should find model.tmdl"
    );
    assert!(
        names.iter().any(|n| n == "relationships.tmdl"),
        "should find relationships.tmdl"
    );
    assert!(
        names.iter().any(|n| n == "Sales.tmdl"),
        "should find Sales.tmdl"
    );
}

/// S-PFC-04: `collect_pbip_files` ignores files with unrelated extensions
/// even when they sit alongside PBIP files.
#[test]
fn collect_pbip_files_ignores_unrelated_extensions() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_pbip_fixture(root);
    touch(root, "README.md");
    touch(root, "Project.SemanticModel/definition/notes.txt");
    touch(root, "Project.Report/definition/changelog.log");

    let files = collect_pbip_files(root);

    let names: Vec<String> = files
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();

    assert!(
        !names.iter().any(|n| n == "README.md"),
        ".md should be ignored"
    );
    assert!(
        !names.iter().any(|n| n == "notes.txt"),
        ".txt should be ignored"
    );
    assert!(
        !names.iter().any(|n| n == "changelog.log"),
        ".log should be ignored"
    );
}

/// S-PFC-05: `collect_pbip_files` returns a sorted list for deterministic
/// downstream behaviour (matches the ordering contract of `collect_powerbi_files`).
#[test]
fn collect_pbip_files_returns_sorted_results() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_pbip_fixture(root);

    let files = collect_pbip_files(root);
    let mut expected = files.clone();
    expected.sort();
    assert_eq!(
        files, expected,
        "collect_pbip_files must return a sorted list for deterministic indexing"
    );
}

/// S-PFC-06: `compute_deleted_paths` returns an empty list when every recorded
/// workspace-relative path still exists.
#[test]
fn compute_deleted_paths_returns_empty_when_all_files_exist() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    touch(root, "Project.pbip");
    touch(root, "Project.SemanticModel/definition.pbism");

    let recorded = vec![
        "Project.pbip".to_string(),
        "Project.SemanticModel/definition.pbism".to_string(),
    ];
    let deleted = compute_deleted_paths(&recorded, root);
    assert!(
        deleted.is_empty(),
        "no recorded paths are deleted; expected empty list"
    );
}

/// S-PFC-07: `compute_deleted_paths` reports the recorded paths whose backing
/// files are missing.
#[test]
fn compute_deleted_paths_detects_removed_files() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    touch(root, "Project.pbip");
    // Note: Project.SemanticModel/definition.pbism intentionally NOT created.

    let recorded = vec![
        "Project.pbip".to_string(),
        "Project.SemanticModel/definition.pbism".to_string(),
    ];
    let deleted = compute_deleted_paths(&recorded, root);
    assert_eq!(
        deleted,
        vec!["Project.SemanticModel/definition.pbism".to_string()],
        "missing recorded paths should be returned verbatim"
    );
}

/// S-PFC-08: `compute_deleted_paths` rejects workspace-escape attempts so a
/// poisoned record cannot probe outside the workspace.
#[test]
fn compute_deleted_paths_rejects_workspace_escape() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    touch(root, "Project.pbip");

    let recorded = vec![
        "Project.pbip".to_string(),
        "../escape.pbip".to_string(),
        "/etc/passwd".to_string(),
    ];
    let deleted = compute_deleted_paths(&recorded, root);
    assert!(
        !deleted
            .iter()
            .any(|p| p.contains("..") || p.starts_with('/')),
        "workspace-escape recorded paths must not be reported as deleted: got {deleted:?}"
    );
}
