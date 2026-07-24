//! Integration tests for notebook search ingestion (063.003-T, 063.004-T, 063.002-T).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tempfile::TempDir;

#[cfg(feature = "cozo-backend")]
use engram::db::{connect_db, queries::CodeGraphQueries};
#[cfg(feature = "cozo-backend")]
use engram::models::lineage::LineageAuthorityContext;
#[cfg(feature = "cozo-backend")]
use engram::models::registry::{ContentSource, ContentSourceStatus};
#[cfg(feature = "cozo-backend")]
use engram::server::state::{AppState, WorkspaceSnapshot};
#[cfg(feature = "cozo-backend")]
use engram::services::ingestion::ingest_all_sources;
#[cfg(feature = "cozo-backend")]
use engram::services::notebook_indexer::index_notebook_source;
#[cfg(feature = "cozo-backend")]
use engram::services::registry::parse_registry_yaml;
#[cfg(feature = "cozo-backend")]
use engram::tools;
#[cfg(feature = "cozo-backend")]
use serde_json::json;

#[cfg(feature = "cozo-backend")]
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("notebooks")
        .join(name)
}

#[cfg(feature = "cozo-backend")]
fn copy_fixture(workspace_root: &Path, name: &str) {
    let notebooks_dir = workspace_root.join("notebooks");
    fs::create_dir_all(&notebooks_dir).expect("create notebooks dir");
    fs::copy(fixture_path(name), notebooks_dir.join(name)).expect("copy notebook fixture");
}

#[cfg(feature = "cozo-backend")]
fn notebook_source(path: &str) -> ContentSource {
    ContentSource {
        content_type: "notebook".to_string(),
        language: None,
        path: path.to_string(),
        pattern: None,
        optional: false,
        status: ContentSourceStatus::Active,
    }
}

/// S-NIN-01: Notebook indexing emits one summary record plus derived per-cell records.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn index_notebook_source_emits_summary_and_cell_records() {
    let root = TempDir::new().expect("tempdir");
    copy_fixture(root.path(), "python_markdown.ipynb");

    let db = connect_db(&root.path().join("data"), "notebook-records")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    let result = index_notebook_source(
        &notebook_source("notebooks"),
        root.path(),
        &queries,
        1_048_576,
        &LineageAuthorityContext::empty(),
    )
    .await
    .expect("index notebook source");
    assert_eq!(result.ingested, 1);

    let records = queries
        .select_content_records(Some("notebook"))
        .await
        .expect("select notebook records");

    assert_eq!(
        records.len(),
        3,
        "expected one summary and two cell records"
    );
    assert!(
        records
            .iter()
            .any(|record| record.record_kind == "notebook_summary" && record.chunk_id.is_none()),
        "summary record must be stored at file scope"
    );
    assert!(
        records.iter().any(|record| {
            record.record_kind == "notebook_summary" && record.content.contains("Indexed cells: 2")
        }),
        "summary record content must use the indexed_cell_count label"
    );
    assert!(
        records.iter().any(|record| {
            record.record_kind == "notebook_markdown_cell"
                && record.chunk_id.as_deref() == Some("cell-0001")
                && record.chunk_index == Some(1)
        }),
        "markdown cell record must preserve the first stable cell ordinal"
    );
    assert!(
        records.iter().any(|record| {
            record.record_kind == "notebook_code_cell"
                && record.chunk_id.as_deref() == Some("cell-0002")
                && record.chunk_index == Some(2)
                && !record.content.contains("hello from output")
        }),
        "code cell record must preserve the second stable cell ordinal without outputs"
    );
}

/// S-NIN-02: `query_memory` returns notebook cell content with language provenance.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn query_memory_returns_notebook_language_provenance() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "sql_magic.ipynb");

    let yaml = "sources:\n  - type: notebook\n    path: notebooks\n";
    let config = parse_registry_yaml(yaml).expect("parse registry");
    let data_dir = dir.path().join("data");
    let db = connect_db(&data_dir, "notebook-query-memory")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("ingest notebook source");

    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(WorkspaceSnapshot {
            workspace_id: "notebook-query-memory".to_string(),
            workspace_uuid: "uuid-notebook-query-memory".to_string(),
            branch: "notebook-query-memory".to_string(),
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
        Some(
            json!({ "query": "SELECT region FROM sales", "content_type": "notebook", "limit": 5 }),
        ),
    )
    .await
    .expect("query_memory");

    let matching = result["results"]
        .as_array()
        .and_then(|results| {
            results.iter().find(|candidate| {
                candidate["record_kind"] == "notebook_code_cell"
                    && candidate["file_path"] == "notebooks/sql_magic.ipynb"
            })
        })
        .expect("expected notebook code-cell result");

    assert!(
        matching["content"]
            .as_str()
            .is_some_and(|content| content.contains("Language: sql")),
        "notebook retrieval content must preserve the resolved language"
    );
}

/// S-NIN-03: Malformed notebook files are skipped without crashing ingestion.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn malformed_notebooks_are_skipped_cleanly() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "python_markdown.ipynb");
    let notebooks_dir = dir.path().join("notebooks");
    fs::write(notebooks_dir.join("broken.ipynb"), "{ this is invalid json")
        .expect("write bad fixture");

    let yaml = "sources:\n  - type: notebook\n    path: notebooks\n";
    let config = parse_registry_yaml(yaml).expect("parse registry");
    let db = connect_db(&dir.path().join("data"), "notebook-malformed")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    let summary = ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("ingest notebooks");

    assert_eq!(summary.ingested, 1, "only the valid notebook should ingest");

    let records = queries
        .select_content_records(Some("notebook"))
        .await
        .expect("select notebook records");
    assert_eq!(records.len(), 3, "broken notebook must not emit records");
}

/// S-NIN-04: Overlapping notebook sources retain distinct records for the same file path.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn index_notebook_source_scopes_records_by_source_path() {
    let root = TempDir::new().expect("tempdir");
    copy_fixture(root.path(), "python_markdown.ipynb");

    let db = connect_db(&root.path().join("data"), "notebook-source-scope")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_notebook_source(
        &notebook_source("."),
        root.path(),
        &queries,
        1_048_576,
        &LineageAuthorityContext::empty(),
    )
    .await
    .expect("index root notebook source");
    index_notebook_source(
        &notebook_source("notebooks"),
        root.path(),
        &queries,
        1_048_576,
        &LineageAuthorityContext::empty(),
    )
    .await
    .expect("index nested notebook source");

    let records = queries
        .select_content_records(Some("notebook"))
        .await
        .expect("select notebook records");
    let matching: Vec<_> = records
        .into_iter()
        .filter(|record| {
            record.file_path == "notebooks/python_markdown.ipynb"
                && record.record_kind == "notebook_summary"
        })
        .collect();

    assert_eq!(
        matching.len(),
        2,
        "overlapping notebook sources should retain distinct summary records"
    );
    assert!(
        matching.iter().any(|record| record.source_path == "."),
        "root-scoped notebook record should be preserved"
    );
    assert!(
        matching
            .iter()
            .any(|record| record.source_path == "notebooks"),
        "nested-scoped notebook record should be preserved"
    );
}
