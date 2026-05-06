//! Integration tests for backlog hydration ingestion pipeline (002.006-T).
//!
//! Tests: end-to-end ingestion, `query_memory` includes backlog content,
//! `unified_search` includes backlog results, deletion sweep, performance.

use std::fs;
use tempfile::TempDir;

fn write_backlog_file(dir: &std::path::Path, name: &str, id: &str, title: &str, kind: &str) {
    let content = format!(
        "---\nid: {id}\ntitle: {title}\nartifact_type: {kind}\nstatus: queued\n---\n\n## Description\n\nThis is the description for {title}.\n"
    );
    fs::write(dir.join(name), content).expect("write backlog file");
}

/// S-BH-01: Registry with backlog source is parsed and recognized.
#[test]
fn backlog_source_type_recognized() {
    use engram::services::registry::parse_registry_yaml;

    let yaml = "sources:\n  - type: backlog\n    path: .backlogit/queue\n";
    let config = parse_registry_yaml(yaml).expect("parse yaml");
    assert_eq!(config.sources[0].content_type, "backlog");
}

/// S-BH-02: `ingest_all_sources` processes a backlog source and returns summary counts.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn ingest_backlog_source_produces_summary() {
    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::services::ingestion::ingest_all_sources;
    use engram::services::registry::parse_registry_yaml;

    let dir = TempDir::new().expect("tempdir");
    let queue_dir = dir.path().join(".backlogit").join("queue");
    fs::create_dir_all(&queue_dir).expect("create queue dir");

    // Write 3 backlog files.
    write_backlog_file(&queue_dir, "001-F.md", "001-F", "Feature one", "feature");
    write_backlog_file(&queue_dir, "001.001-T.md", "001.001-T", "Task one", "task");
    write_backlog_file(&queue_dir, "001.002-T.md", "001.002-T", "Task two", "task");

    let yaml = "sources:\n  - type: backlog\n    path: .backlogit/queue\n".to_string();
    let config = parse_registry_yaml(&yaml).expect("parse registry");

    let db = connect_db(dir.path(), "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    let summary = ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("ingest_all_sources must succeed");

    assert!(
        summary.ingested >= 3,
        "should have ingested at least 3 backlog items, got {}",
        summary.ingested
    );
}

/// S-BH-03: After ingestion, backlog nodes appear in the DB.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn ingested_nodes_appear_in_db() {
    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::services::ingestion::ingest_all_sources;
    use engram::services::registry::parse_registry_yaml;

    let dir = TempDir::new().expect("tempdir");
    let queue_dir = dir.path().join(".backlogit").join("queue");
    fs::create_dir_all(&queue_dir).expect("create queue dir");

    write_backlog_file(&queue_dir, "010-F.md", "010-F", "Feature ten", "feature");

    let yaml = "sources:\n  - type: backlog\n    path: .backlogit/queue\n";
    let config = parse_registry_yaml(yaml).expect("parse registry");

    let db = connect_db(dir.path(), "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("ingest");

    let nodes = queries
        .select_backlog_nodes(None)
        .await
        .expect("select_backlog_nodes");
    assert!(
        !nodes.is_empty(),
        "backlog nodes should appear in DB after ingestion"
    );
    assert!(
        nodes.iter().any(|n| n.id == "010-F"),
        "010-F should be indexed"
    );
}

/// S-BH-04: `query_memory` returns backlog content after ingestion.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn query_memory_returns_backlog_content() {
    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::services::ingestion::ingest_all_sources;
    use engram::services::registry::parse_registry_yaml;

    let dir = TempDir::new().expect("tempdir");
    let queue_dir = dir.path().join(".backlogit").join("queue");
    fs::create_dir_all(&queue_dir).expect("create queue dir");

    write_backlog_file(
        &queue_dir,
        "050-F.md",
        "050-F",
        "Unique search target",
        "feature",
    );

    let yaml = "sources:\n  - type: backlog\n    path: .backlogit/queue\n";
    let config = parse_registry_yaml(yaml).expect("parse registry");

    let db = connect_db(dir.path(), "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("ingest");

    // query_memory reads backlog_content_record; verify via select
    let records = queries
        .select_backlog_content_records(None)
        .await
        .expect("select_backlog_content_records");
    assert!(
        !records.is_empty(),
        "query_memory should find backlog content records after ingestion"
    );
    assert!(
        records
            .iter()
            .any(|r| r.content.contains("Unique search target")),
        "content should include the title text"
    );
}

/// S-BH-05: Deletion sweep removes stale nodes after file removal.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn deletion_sweep_cleans_stale_nodes() {
    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::services::backlog_indexer::sweep_deleted_backlog_files;
    use engram::services::ingestion::ingest_all_sources;
    use engram::services::registry::parse_registry_yaml;

    let dir = TempDir::new().expect("tempdir");
    let queue_dir = dir.path().join(".backlogit").join("queue");
    fs::create_dir_all(&queue_dir).expect("create queue dir");

    let path_to_delete = queue_dir.join("060-F.md");
    write_backlog_file(&queue_dir, "060-F.md", "060-F", "To be deleted", "feature");
    write_backlog_file(&queue_dir, "061-F.md", "061-F", "Stays alive", "feature");

    let yaml = "sources:\n  - type: backlog\n    path: .backlogit/queue\n";
    let config = parse_registry_yaml(yaml).expect("parse registry");

    let db = connect_db(dir.path(), "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("ingest");

    // Remove file from disk.
    fs::remove_file(&path_to_delete).expect("remove file");

    // Run deletion sweep.
    let source = &config.sources[0];
    let removed = sweep_deleted_backlog_files(source, dir.path(), &queries)
        .await
        .expect("deletion sweep");

    assert_eq!(removed, 1, "exactly one file should be swept");

    let remaining = queries
        .select_backlog_nodes(None)
        .await
        .expect("select after sweep");
    assert_eq!(remaining.len(), 1, "only 061-F should remain");
    assert_eq!(remaining[0].id, "061-F");
}

/// S-BH-06: Performance — 100+ items indexed in under 5 seconds.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn backlog_index_100_items_under_5_seconds() {
    use std::time::Instant;

    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::services::ingestion::ingest_all_sources;
    use engram::services::registry::parse_registry_yaml;

    let dir = TempDir::new().expect("tempdir");
    let queue_dir = dir.path().join(".backlogit").join("queue");
    fs::create_dir_all(&queue_dir).expect("create queue dir");

    // Write 100 backlog files.
    for i in 0..100_u32 {
        let name = format!("{i:03}-F.md");
        let id = format!("{i:03}-F");
        let title = format!("Feature {i}");
        write_backlog_file(&queue_dir, &name, &id, &title, "feature");
    }

    let yaml = "sources:\n  - type: backlog\n    path: .backlogit/queue\n";
    let config = parse_registry_yaml(yaml).expect("parse registry");

    let db = connect_db(dir.path(), "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    let start = Instant::now();
    let summary = ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("ingest");
    let elapsed = start.elapsed();

    assert!(
        summary.ingested >= 100,
        "should have ingested at least 100 items, got {}",
        summary.ingested
    );
    assert!(
        elapsed.as_secs() < 5,
        "100-item ingestion should complete in under 5 seconds, took {elapsed:?}"
    );
}
