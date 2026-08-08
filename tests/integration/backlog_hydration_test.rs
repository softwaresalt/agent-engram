//! Integration tests for backlog hydration ingestion pipeline (002.006-T).
//!
//! Tests: source-type recognition, end-to-end ingestion, DB-layer persistence,
//! `query_memory` returns backlog content, deletion sweep, performance.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write_backlog_file(dir: &std::path::Path, name: &str, id: &str, title: &str, kind: &str) {
    let content = format!(
        "---\nid: {id}\ntitle: {title}\nartifact_type: {kind}\nstatus: queued\n---\n\n## Description\n\nThis is the description for {title}.\n"
    );
    fs::write(dir.join(name), content).expect("write backlog file");
}

#[cfg(unix)]
fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

/// S-BH-01: Registry with backlog source is parsed and recognized.
#[cfg(feature = "cozo-backend")]
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

    // `select_backlog_content_records` queries the DB layer directly.
    // The MCP `query_memory` tool also includes these records (when content_type
    // filter is unset or "backlog") via `select_backlog_content_records`, so
    // this test validates DB layer persistence which is the foundation for both
    // access paths.
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

/// 110-S U3: an unavailable backlog source root is non-authoritative and must
/// retain the paired graph/content rows from the last successful pass.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn deletion_sweep_retains_nodes_when_source_root_is_unavailable() {
    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::services::backlog_indexer::sweep_deleted_backlog_files;
    use engram::services::ingestion::ingest_all_sources;
    use engram::services::registry::parse_registry_yaml;

    let dir = TempDir::new().expect("tempdir");
    let queue_dir = dir.path().join(".backlogit").join("queue");
    fs::create_dir_all(&queue_dir).expect("create queue dir");
    write_backlog_file(
        &queue_dir,
        "062-F.md",
        "062-F",
        "Retained during outage",
        "feature",
    );

    let config =
        parse_registry_yaml("sources:\n  - type: backlog\n    path: .backlogit/queue\n")
            .expect("parse registry");
    let db = connect_db(dir.path(), "backlog-sweep-missing-root")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("ingest");
    fs::rename(
        &queue_dir,
        dir.path().join(".backlogit").join("queue-unavailable"),
    )
    .expect("make source root unavailable");

    let removed = sweep_deleted_backlog_files(&config.sources[0], dir.path(), &queries)
        .await
        .expect("deletion sweep");
    assert_eq!(
        removed, 0,
        "an unavailable source root must suppress the deletion sweep"
    );
    assert!(
        queries
            .select_backlog_nodes(None)
            .await
            .expect("nodes after unavailable root")
            .iter()
            .any(|node| node.id == "062-F"),
        "the live control node must be retained"
    );
}

/// 110-S U3: a complete traversal may retire a physically-live stale alias
/// when the canonical directory winner changes within the workspace.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn deletion_sweep_removes_alias_superseded_on_complete_pass() {
    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::services::backlog_indexer::{
        index_backlog_source, sweep_deleted_backlog_files,
    };
    use engram::services::registry::parse_registry_yaml;

    let dir = TempDir::new().expect("tempdir");
    let queue_dir = dir.path().join(".backlogit").join("queue");
    let alias_dir = queue_dir.join("a");
    fs::create_dir_all(&alias_dir).expect("create alias dir");
    write_backlog_file(
        &alias_dir,
        "063-F.md",
        "063-F",
        "Superseded alias",
        "feature",
    );

    let config =
        parse_registry_yaml("sources:\n  - type: backlog\n    path: .backlogit/queue\n")
            .expect("parse registry");
    let source = &config.sources[0];
    let db = connect_db(dir.path(), "backlog-sweep-alias")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_backlog_source(
        source,
        dir.path(),
        &queries,
        config.max_file_size_bytes,
    )
    .await
    .expect("index alias path");

    let real_dir = queue_dir.join("z");
    fs::rename(&alias_dir, &real_dir).expect("rename alias winner");
    if let Err(error) = symlink_dir(&real_dir, &alias_dir) {
        eprintln!("skipping alias-supersession assertion: cannot create directory symlink: {error}");
        return;
    }

    let removed = sweep_deleted_backlog_files(source, dir.path(), &queries)
        .await
        .expect("deletion sweep");
    assert_eq!(
        removed, 1,
        "the complete pass must remove the superseded alias path"
    );
    assert!(
        !queries
            .select_backlog_nodes(None)
            .await
            .expect("nodes after alias replacement")
            .iter()
            .any(|node| node.id == "063-F"),
        "the stale alias node must be removed with its content row"
    );
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
        elapsed < std::time::Duration::from_secs(5),
        "100-item ingestion should complete in under 5 seconds, took {elapsed:?}"
    );
}
