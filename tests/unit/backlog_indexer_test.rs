//! Unit tests for the backlog indexer (002.004-T, 002.005-T).
//!
//! Tests: index valid files, skip invalid, handle missing fields,
//! hash-based skip, parent/child and dependency edges, delete detection.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

use engram::services::backlog_indexer::{
    collect_backlog_files_in_workspace, compute_file_hash, extract_backlog_data,
};

#[cfg(unix)]
fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

fn create_symlink_dir(src: &Path, dst: &Path) -> bool {
    match symlink_dir(src, dst) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(error) => panic!("create directory symlink: {error}"),
    }
}

#[cfg(unix)]
fn symlink_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn symlink_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(src, dst)
}

fn create_symlink_file(src: &Path, dst: &Path) -> bool {
    match symlink_file(src, dst) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(error) => panic!("create file symlink: {error}"),
    }
}

fn write_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).expect("write file");
    path
}

/// S-BI-01: Valid backlog file produces a `BacklogNode` with correct fields.
#[test]
fn valid_file_produces_backlog_node() {
    let dir = TempDir::new().expect("tempdir");
    let content = "---\nid: 001-T\ntitle: A task\nartifact_type: task\nstatus: queued\n---\n\n## Description\n\nDoes something.";
    let path = write_file(dir.path(), "001-T.md", content);

    let source_path = dir.path().to_str().unwrap();
    let result =
        extract_backlog_data(&path, dir.path(), source_path).expect("extraction should succeed");

    assert!(result.is_some(), "valid file should produce a result");
    let (node, _, _) = result.unwrap();
    assert_eq!(node.id, "001-T");
    assert_eq!(node.kind, "task");
    assert_eq!(node.status, "queued");
}

/// S-BI-02: File with no frontmatter is skipped (returns None).
#[test]
fn file_without_frontmatter_is_skipped() {
    let dir = TempDir::new().expect("tempdir");
    let content = "# Just a heading\n\nNo YAML frontmatter here.";
    let path = write_file(dir.path(), "bare.md", content);

    let source_path = dir.path().to_str().unwrap();
    let result =
        extract_backlog_data(&path, dir.path(), source_path).expect("call should not error");

    assert!(
        result.is_none(),
        "file without frontmatter should be skipped"
    );
}

/// S-BI-03: File with missing `id` field is skipped (returns None).
#[test]
fn file_missing_id_is_skipped() {
    let dir = TempDir::new().expect("tempdir");
    let content = "---\ntitle: No ID task\nartifact_type: task\n---\n\nBody.";
    let path = write_file(dir.path(), "no-id.md", content);

    let source_path = dir.path().to_str().unwrap();
    let result =
        extract_backlog_data(&path, dir.path(), source_path).expect("call should not error");

    assert!(
        result.is_none(),
        "file missing required `id` field should be skipped"
    );
}

/// S-BI-04: Hash-based change detection skips unchanged files.
#[test]
fn unchanged_file_detected_by_hash() {
    let content = b"---\nid: 003-T\ntitle: Stable\n---\n\nBody.";
    let hash1 = compute_file_hash(content);
    let hash2 = compute_file_hash(content);
    assert_eq!(hash1, hash2, "same content should produce the same hash");

    let different = b"---\nid: 003-T\ntitle: Changed\n---\n\nNew body.";
    let hash3 = compute_file_hash(different);
    assert_ne!(
        hash1, hash3,
        "different content should produce different hashes"
    );
}

/// S-BI-05: Parent-child relationship produces a `parent_of` edge.
#[test]
fn parent_child_produces_parent_of_edge() {
    let dir = TempDir::new().expect("tempdir");
    // A task with parent_id produces a parent_of edge.
    let content = "---\nid: 002.001-T\ntitle: Child task\nartifact_type: task\nparent_id: 002-F\n---\n\nBody.";
    let path = write_file(dir.path(), "002.001-T.md", content);

    let source_path = dir.path().to_str().unwrap();
    let result =
        extract_backlog_data(&path, dir.path(), source_path).expect("extraction should succeed");

    assert!(result.is_some());
    let (_, edges, _) = result.unwrap();
    assert!(
        edges.iter().any(|e| {
            use engram::models::backlog_graph::BacklogEdgeType;
            e.edge_type == BacklogEdgeType::ParentOf
                && e.from_id == "002-F"
                && e.to_id == "002.001-T"
        }),
        "parent_id should produce a parent_of edge"
    );
}

/// S-BI-06: Dependency list produces `depends_on` edges.
#[test]
fn dependencies_produce_depends_on_edges() {
    let dir = TempDir::new().expect("tempdir");
    let content = "---\nid: 003.002-T\ntitle: Dependent\nartifact_type: task\ndependencies:\n  - 003.001-T\n---\n\nBody.";
    let path = write_file(dir.path(), "003.002-T.md", content);

    let source_path = dir.path().to_str().unwrap();
    let result =
        extract_backlog_data(&path, dir.path(), source_path).expect("extraction should succeed");

    assert!(result.is_some());
    let (_, edges, _) = result.unwrap();
    assert!(
        edges.iter().any(|e| {
            use engram::models::backlog_graph::BacklogEdgeType;
            e.edge_type == BacklogEdgeType::DependsOn
                && e.from_id == "003.002-T"
                && e.to_id == "003.001-T"
        }),
        "dependencies list should produce depends_on edges"
    );
}

/// S-BI-07: Deletion sweep detects files that no longer exist on disk.
#[test]
fn deletion_sweep_detects_removed_files() {
    use engram::services::backlog_indexer::compute_deleted_paths;

    let dir = TempDir::new().expect("tempdir");
    // Create one file on disk.
    write_file(dir.path(), "live.md", "---\nid: live\n---\n");

    // Tell the sweep about two workspace-relative "known" paths; one is gone.
    let known = vec!["live.md".to_string(), "gone.md".to_string()];

    let deleted = compute_deleted_paths(&known, dir.path());
    assert_eq!(deleted.len(), 1, "only gone.md should appear as deleted");
    assert!(
        deleted[0].contains("gone.md"),
        "the deleted path should reference gone.md"
    );
}

/// S-BI-09: deletion sweeps mirror collectors by treating final-component file
/// symlinks and directory-symlink escapes as deleted while preserving regular
/// files and absent paths.
#[test]
fn deletion_sweep_reports_symlink_candidates_and_escapes_as_deleted() {
    use engram::services::backlog_indexer::compute_deleted_paths;

    let workspace = TempDir::new().expect("workspace tempdir");
    let external = TempDir::new().expect("external tempdir");
    write_file(workspace.path(), "regular.md", "---\nid: regular\n---\n");
    let target_path = write_file(workspace.path(), "target.md", "---\nid: target\n---\n");
    let symlink_path = workspace.path().join("indexed.md");
    if !create_symlink_file(&target_path, &symlink_path) {
        return;
    }

    let external_dir = external.path().join("escape");
    fs::create_dir_all(&external_dir).expect("create external dir");
    fs::write(external_dir.join("outside.md"), "---\nid: outside\n---\n").expect("write outside");
    if !create_symlink_dir(&external_dir, &workspace.path().join("linked-outside")) {
        return;
    }

    let known = vec![
        "regular.md".to_string(),
        "indexed.md".to_string(),
        "linked-outside/outside.md".to_string(),
        "absent.md".to_string(),
    ];

    let deleted = compute_deleted_paths(&known, workspace.path());

    assert_eq!(
        deleted,
        vec![
            "indexed.md".to_string(),
            "linked-outside/outside.md".to_string(),
            "absent.md".to_string(),
        ]
    );
}

/// S-BI-10: deletion sweeps refuse to probe absolute, root-relative, or `..`
/// paths, skipping poisoned records instead of touching the filesystem outside
/// the workspace root.
#[test]
fn deletion_sweep_skips_paths_that_escape_the_workspace() {
    use engram::services::backlog_indexer::compute_deleted_paths;

    let workspace = TempDir::new().expect("workspace tempdir");
    write_file(workspace.path(), "live.md", "---\nid: live\n---\n");

    let known = vec![
        "live.md".to_string(),
        "../escape.md".to_string(),
        workspace
            .path()
            .join("live.md")
            .to_string_lossy()
            .to_string(),
    ];

    let deleted = compute_deleted_paths(&known, workspace.path());

    assert!(
        deleted.is_empty(),
        "live file is kept and escaping paths are skipped without probing: {deleted:?}"
    );
}

/// S-BI-08: Backlog recursive collection follows in-workspace symlinked
/// directories once, skips escaping symlink targets, and terminates cycles.
#[test]
fn collect_backlog_files_handles_symlink_cycles_and_workspace_bounds() {
    let workspace = TempDir::new().expect("workspace tempdir");
    let external = TempDir::new().expect("external tempdir");

    let queue = workspace.path().join(".backlogit").join("queue");
    fs::create_dir_all(&queue).expect("create queue");
    fs::write(queue.join("local.md"), "---\nid: local\n---\n").expect("write local");

    let shared = workspace.path().join("shared");
    fs::create_dir_all(&shared).expect("create shared");
    fs::write(shared.join("shared.md"), "---\nid: shared\n---\n").expect("write shared");

    let escape = external.path().join("escape");
    fs::create_dir_all(&escape).expect("create escape");
    fs::write(escape.join("outside.md"), "---\nid: outside\n---\n").expect("write outside");

    let linked_escape_created = create_symlink_dir(&escape, &queue.join("linked-escape"));
    let linked_shared_created = create_symlink_dir(&shared, &queue.join("linked-shared"));
    if linked_shared_created {
        let _ = create_symlink_dir(&queue, &shared.join("cycle"));
    }

    if !linked_escape_created && !linked_shared_created {
        return;
    }

    let files = collect_backlog_files_in_workspace(&queue, workspace.path());
    let rel_paths: Vec<String> = files
        .iter()
        .map(|path| {
            path.strip_prefix(workspace.path())
                .expect("path under workspace")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();

    assert!(rel_paths.iter().any(|p| p == ".backlogit/queue/local.md"));
    assert!(
        !rel_paths.iter().any(|p| p.contains("outside.md")),
        "workspace-escaping symlink targets must be skipped; got {rel_paths:?}"
    );
    if linked_shared_created {
        assert!(
            rel_paths
                .iter()
                .any(|p| p == ".backlogit/queue/linked-shared/shared.md"),
            "in-workspace symlinked directories should be collected; got {rel_paths:?}"
        );
        assert_eq!(
            rel_paths
                .iter()
                .filter(|p| p.ends_with("shared.md"))
                .count(),
            1,
            "symlink cycles should not collect duplicate real files; got {rel_paths:?}"
        );
    }
}
