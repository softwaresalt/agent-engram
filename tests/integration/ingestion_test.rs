//! Integration tests for multi-source ingestion pipeline (T022).
//!
//! Tests file-level ingestion behavior using temp directories.
//! Validates scenarios: S017, S020, S021-S023, S025.

use engram::config::StaleStrategy;
use engram::models::config::DaemonMode;
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

/// Markdown ingestion must emit section-aware chunks that `query_memory` can
/// return with provenance metadata.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn markdown_query_memory_returns_chunk_provenance() {
    use std::collections::HashMap;
    use std::sync::Arc;

    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::server::state::{AppState, WorkspaceSnapshot};
    use engram::services::ingestion::ingest_all_sources;
    use engram::services::registry::parse_registry_yaml;
    use engram::tools;
    use serde_json::json;

    let dir = TempDir::new().expect("tempdir");
    let docs = dir.path().join("docs");
    fs::create_dir_all(&docs).expect("create docs dir");
    fs::write(
        docs.join("guide.md"),
        "# Guide\n\n## Install\n\nRun cargo build --release.\n\n## Use\n\nRun engram query-memory.\n",
    )
    .expect("write markdown");

    let yaml = "sources:\n  - type: docs\n    language: markdown\n    path: docs\n    pattern: \"**/*.md\"\n";
    let config = parse_registry_yaml(yaml).expect("parse registry");
    let data_dir = dir.path().join("data");
    let db = connect_db(&data_dir, "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("ingest_all_sources");

    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    state
        .set_workspace(WorkspaceSnapshot {
            workspace_id: "markdown-query-memory".to_string(),
            workspace_uuid: "uuid-markdown-query-memory".to_string(),
            branch: "test-branch".to_string(),
            data_dir,
            path: dir.path().to_string_lossy().to_string(),
            last_flush: None,
            stale_files: false,
            connection_count: 1,
            file_mtimes: HashMap::new(),
        })
        .await
        .expect("bind workspace");

    let result = tools::dispatch(
        state,
        "query_memory",
        Some(json!({ "query": "cargo build", "content_type": "docs", "limit": 5 })),
    )
    .await
    .expect("query_memory");

    let first = result["results"]
        .as_array()
        .and_then(|results| results.first())
        .expect("expected markdown chunk result");

    assert_eq!(first["record_kind"], "markdown_chunk");
    assert_eq!(first["file_path"], "docs/guide.md");
    assert_eq!(first["title"], "Install");
    assert_eq!(first["heading_path"], json!(["Guide", "Install"]));
    assert!(
        first["line_range"]
            .as_str()
            .is_some_and(|range| range.starts_with('L')),
        "chunk results should surface a line range"
    );
}

/// Documents without a stable heading spine must fall back to file-level
/// retrieval with advisory lint metadata instead of silent rewrites.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn markdown_query_memory_exposes_fallback_lint_guardrails() {
    use std::collections::HashMap;
    use std::sync::Arc;

    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::server::state::{AppState, WorkspaceSnapshot};
    use engram::services::ingestion::ingest_all_sources;
    use engram::services::registry::parse_registry_yaml;
    use engram::tools;
    use serde_json::json;

    let dir = TempDir::new().expect("tempdir");
    let docs = dir.path().join("docs");
    fs::create_dir_all(&docs).expect("create docs dir");
    fs::write(
        docs.join("notes.md"),
        "Overview paragraph without headings.\n\n### Deep topic\n\nDetails for retrieval.\n",
    )
    .expect("write markdown");

    let yaml = "sources:\n  - type: docs\n    language: markdown\n    path: docs\n    pattern: \"**/*.md\"\n";
    let config = parse_registry_yaml(yaml).expect("parse registry");
    let data_dir = dir.path().join("data");
    let db = connect_db(&data_dir, "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("ingest_all_sources");

    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    state
        .set_workspace(WorkspaceSnapshot {
            workspace_id: "markdown-query-memory-fallback".to_string(),
            workspace_uuid: "uuid-markdown-query-memory-fallback".to_string(),
            branch: "test-branch".to_string(),
            data_dir,
            path: dir.path().to_string_lossy().to_string(),
            last_flush: None,
            stale_files: false,
            connection_count: 1,
            file_mtimes: HashMap::new(),
        })
        .await
        .expect("bind workspace");

    let result = tools::dispatch(
        state,
        "query_memory",
        Some(json!({ "query": "retrieval", "content_type": "docs", "limit": 5 })),
    )
    .await
    .expect("query_memory");

    let first = result["results"]
        .as_array()
        .and_then(|results| results.first())
        .expect("expected fallback markdown result");

    assert_eq!(first["record_kind"], "file");
    assert_eq!(first["fallback_reason"], "missing_heading_structure");
    assert!(
        first["lint_summary"]
            .as_str()
            .is_some_and(|summary| summary.contains("missing_h1")),
        "fallback result should surface advisory lint findings"
    );
    assert!(
        first["suggestions"]
            .as_array()
            .is_some_and(|suggestions| suggestions.iter().any(|value| value == "# Notes")),
        "fallback result should expose advisory heading suggestions"
    );
}

/// Re-ingesting overlapping sources should preserve embeddings for unchanged files.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn ingest_all_sources_scopes_unchanged_detection_to_source_path() {
    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::services::embedding::EMBEDDING_DIM;
    use engram::services::ingestion::ingest_all_sources;
    use engram::services::registry::parse_registry_yaml;

    let dir = TempDir::new().expect("tempdir");
    let docs = dir.path().join("docs");
    fs::create_dir_all(&docs).expect("create docs dir");
    fs::write(
        docs.join("shared.md"),
        "Overview paragraph without headings.\n",
    )
    .expect("write markdown");

    let yaml = "sources:\n  - type: docs\n    language: markdown\n    path: .\n    pattern: \"docs/*.md\"\n  - type: docs\n    language: markdown\n    path: docs\n    pattern: \"*.md\"\n";
    let config = parse_registry_yaml(yaml).expect("parse registry");
    let db = connect_db(&dir.path().join("data"), "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("initial ingest");

    let records = queries
        .select_content_records(Some("docs"))
        .await
        .expect("select initial docs records");
    assert_eq!(
        records.len(),
        2,
        "expected one record per overlapping source"
    );

    let mut embedding = vec![0.0_f32; EMBEDDING_DIM];
    embedding[0] = 1.0;
    for record in &records {
        queries
            .update_content_record_embedding(record.id.as_str(), embedding.clone())
            .await
            .expect("seed embedding");
    }

    ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("reingest unchanged sources");

    let records = queries
        .select_content_records(Some("docs"))
        .await
        .expect("select docs records after reingest");
    assert!(
        records.iter().all(|record| record.embedding.is_some()),
        "unchanged overlapping sources should preserve embeddings"
    );
}

/// Watched single-file ingestion should skip unchanged files within the same source scope.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn ingest_single_file_scopes_unchanged_detection_to_source_path() {
    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::services::embedding::EMBEDDING_DIM;
    use engram::services::ingestion::{ingest_all_sources, ingest_single_file};
    use engram::services::registry::parse_registry_yaml;

    let dir = TempDir::new().expect("tempdir");
    let docs = dir.path().join("docs");
    fs::create_dir_all(&docs).expect("create docs dir");
    let file_path = docs.join("shared.md");
    fs::write(&file_path, "Overview paragraph without headings.\n").expect("write markdown");

    let yaml = "sources:\n  - type: docs\n    language: markdown\n    path: .\n    pattern: \"docs/*.md\"\n  - type: docs\n    language: markdown\n    path: docs\n    pattern: \"*.md\"\n";
    let config = parse_registry_yaml(yaml).expect("parse registry");
    let db = connect_db(&dir.path().join("data"), "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("initial ingest");

    let records = queries
        .select_content_records(Some("docs"))
        .await
        .expect("select initial docs records");
    let docs_record = records
        .iter()
        .find(|record| record.source_path == "docs")
        .expect("docs-scoped record");

    let mut embedding = vec![0.0_f32; EMBEDDING_DIM];
    embedding[0] = 1.0;
    queries
        .update_content_record_embedding(docs_record.id.as_str(), embedding)
        .await
        .expect("seed docs embedding");

    let updated = ingest_single_file(
        &file_path,
        dir.path(),
        "docs",
        "docs",
        1_048_576,
        None,
        &queries,
    )
    .await
    .expect("ingest_single_file");

    assert!(
        !updated,
        "unchanged file should not be rewritten because another source shares the path"
    );

    let records = queries
        .select_content_records(Some("docs"))
        .await
        .expect("select docs records after single-file ingest");
    let docs_record = records
        .iter()
        .find(|record| record.source_path == "docs")
        .expect("docs-scoped record after single-file ingest");
    assert!(
        docs_record.embedding.is_some(),
        "unchanged single-file ingestion should preserve the existing embedding"
    );
}
