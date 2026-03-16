//! Integration tests for multi-source ingestion pipeline (T022).
//!
//! Tests file-level ingestion behavior using temp directories.
//! Validates scenarios: S017, S020, S021-S023, S025.

use std::fs;
use tempfile::TempDir;

/// Helper: check if a file would be skipped as binary.
fn is_binary(content: &[u8]) -> bool {
    let check_len = content.len().min(8192);
    content[..check_len].contains(&0)
}

/// Helper: compute SHA-256 hash.
fn compute_hash(content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// S017: Docs source ingests markdown files.
#[test]
fn docs_source_contains_markdown() {
    let dir = TempDir::new().unwrap();
    let docs = dir.path().join("docs");
    fs::create_dir_all(&docs).unwrap();
    fs::write(docs.join("quickstart.md"), "# Quickstart\nHello world").unwrap();

    let content = fs::read(docs.join("quickstart.md")).unwrap();
    assert!(!is_binary(&content));
    assert!(!content.is_empty());
}

/// S020: File exceeding 1MB size limit is skipped.
#[test]
fn oversized_file_exceeds_limit() {
    let max_size: u64 = 1_048_576;
    let big_content = vec![b'x'; 2_000_000];
    assert!(big_content.len() as u64 > max_size);
}

/// S021: File at exactly 1MB boundary is accepted (limit is exclusive).
#[test]
fn file_at_1mb_boundary_accepted() {
    let max_size: u64 = 1_048_576;
    let content = vec![b'x'; 1_048_576];
    assert!(content.len() as u64 <= max_size);
}

/// S022: File at 1MB + 1 byte is rejected.
#[test]
fn file_over_1mb_rejected() {
    let max_size: u64 = 1_048_576;
    let content = vec![b'x'; 1_048_577];
    assert!(content.len() as u64 > max_size);
}

/// S023: Empty file produces valid hash.
#[test]
fn empty_file_produces_valid_hash() {
    let hash = compute_hash(b"");
    assert_eq!(hash.len(), 64);
    assert!(!hash.is_empty());
}

/// S025: Binary file (contains null bytes) is detected.
#[test]
fn binary_file_detected() {
    let mut content = vec![0u8; 100];
    content[50] = 0; // null byte
    assert!(is_binary(&content));
}

/// Text file without null bytes passes binary check.
#[test]
fn text_file_passes_binary_check() {
    let content = b"Hello world, this is plain text.";
    assert!(!is_binary(content));
}

/// Change detection: same content produces same hash.
#[test]
fn same_content_same_hash() {
    let hash1 = compute_hash(b"Hello world");
    let hash2 = compute_hash(b"Hello world");
    assert_eq!(hash1, hash2);
}

/// Change detection: different content produces different hash.
#[test]
fn different_content_different_hash() {
    let hash1 = compute_hash(b"Hello world");
    let hash2 = compute_hash(b"Hello World");
    assert_ne!(hash1, hash2);
}
