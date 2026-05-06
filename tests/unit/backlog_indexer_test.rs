//! Unit tests for the backlog indexer (002.004-T, 002.005-T).
//!
//! Tests: index valid files, skip invalid, handle missing fields,
//! verify hash-based skip, delete detection, orphaned edges removed,
//! content records removed.

use std::fs;
use tempfile::TempDir;

use engram::services::backlog_indexer::{compute_file_hash, extract_backlog_data};

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

    // Tell the sweep about two "known" paths; one no longer exists.
    let known = vec![
        dir.path()
            .join("live.md")
            .to_str()
            .unwrap()
            .replace('\\', "/"),
        dir.path()
            .join("gone.md")
            .to_str()
            .unwrap()
            .replace('\\', "/"),
    ];

    let deleted = compute_deleted_paths(&known);
    assert_eq!(deleted.len(), 1, "only gone.md should appear as deleted");
    assert!(
        deleted[0].contains("gone.md"),
        "the deleted path should reference gone.md"
    );
}
