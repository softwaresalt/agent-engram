//! Contract tests for the dedicated PBIP indexer's content records and project
//! graph (`062.002-T`).
//!
//! Drives [`engram::services::pbip_indexer::index_pbip_source`] over a
//! fixture PBIP project laid out on disk, then asserts:
//! * object-level `content_type = "pbip"` records are searchable per entity,
//! * the project graph walks report→page→visual via `pbi_contains`,
//! * the report depends on its semantic model via `pbi_depends_on_model`,
//! * a visual binds a model measure via `pbi_uses_field`,
//! * re-indexing unchanged input is a no-op, and a deletion sweep prunes
//!   records and nodes for files removed from disk.

#![cfg(feature = "cozo-backend")]

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::TraversalDirection;
use engram::models::registry::{ContentSource, ContentSourceStatus};
use engram::services::pbip_indexer::{index_pbip_source, sweep_deleted_pbip_files};

const MAX_FILE_SIZE: u64 = 1_048_576;

fn pbip_source(path: &str) -> ContentSource {
    ContentSource {
        content_type: "pbip".to_string(),
        language: None,
        path: path.to_string(),
        pattern: None,
        optional: false,
        status: ContentSourceStatus::Active,
    }
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(&path, content).expect("write fixture file");
}

/// Lay out a minimal but complete PBIP project under `<root>/proj`:
/// one report with one page and one visual, plus a TMDL semantic model with a
/// `Sales` table carrying an `Amount` column and a `Total Sales` measure that
/// the visual binds to.
fn write_project(root: &Path) {
    write(
        root,
        "proj/My.pbip",
        r#"{"version":"1.0","artifacts":[{"report":{"path":"My.Report"}}]}"#,
    );
    write(
        root,
        "proj/My.Report/definition.pbir",
        r#"{"version":"1.0","datasetReference":{"byPath":{"path":"../My.SemanticModel"}}}"#,
    );
    write(root, "proj/My.Report/definition/report.json", "{}");
    write(
        root,
        "proj/My.Report/definition/pages/pages.json",
        r#"{"pageOrder":["Page1"],"activePageName":"Page1"}"#,
    );
    write(
        root,
        "proj/My.Report/definition/pages/Page1/page.json",
        r#"{"name":"Page1","displayName":"First Page"}"#,
    );
    write(
        root,
        "proj/My.Report/definition/pages/Page1/visuals/v1/visual.json",
        r#"{
            "name":"v1",
            "visual":{
                "visualType":"card",
                "query":{"queryState":{"Values":{"projections":[
                    {"field":{"Measure":{"Expression":{"SourceRef":{"Entity":"Sales"}},"Property":"Total Sales"}}}
                ]}}}
            }
        }"#,
    );
    write(
        root,
        "proj/My.SemanticModel/definition.pbism",
        r#"{"version":"4.0"}"#,
    );
    write(
        root,
        "proj/My.SemanticModel/definition/model.tmdl",
        "model Sales Dataset\n\nref table Sales\n",
    );
    write(
        root,
        "proj/My.SemanticModel/definition/tables/Sales.tmdl",
        "table Sales\n  column Amount\n    dataType: double\n  measure 'Total Sales' = SUM ( Sales[Amount] )\n",
    );
}

/// S-PGQ-01: object-level PBIP content records are emitted per entity.
#[tokio::test]
async fn index_pbip_source_emits_object_level_records() {
    let root = TempDir::new().expect("tempdir");
    write_project(root.path());

    let db = connect_db(&root.path().join("data"), "pbip-records")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    let result = index_pbip_source(&pbip_source("proj"), root.path(), &queries, MAX_FILE_SIZE)
        .await
        .expect("index pbip source");
    assert!(result.ingested > 0, "expected files to be ingested");

    let records = queries
        .select_content_records(Some("pbip"))
        .await
        .expect("select pbip records");

    let kinds: Vec<&str> = records.iter().map(|r| r.record_kind.as_str()).collect();
    for expected in [
        "pbip_workspace",
        "pbip_report",
        "pbip_page",
        "pbip_visual",
        "pbip_table",
        "pbip_measure",
    ] {
        assert!(
            kinds.contains(&expected),
            "expected a {expected} record, got {kinds:?}"
        );
    }

    // Ensure-coverage: every collected file owns at least one record so change
    // detection stays stable across runs.
    for file in [
        "proj/My.pbip",
        "proj/My.Report/definition.pbir",
        "proj/My.Report/definition/report.json",
        "proj/My.Report/definition/pages/pages.json",
        "proj/My.Report/definition/pages/Page1/page.json",
        "proj/My.Report/definition/pages/Page1/visuals/v1/visual.json",
        "proj/My.SemanticModel/definition.pbism",
        "proj/My.SemanticModel/definition/model.tmdl",
        "proj/My.SemanticModel/definition/tables/Sales.tmdl",
    ] {
        assert!(
            records.iter().any(|r| r.file_path == file),
            "no record covers collected file {file}"
        );
    }
}

/// S-PGQ-02: the project graph walks report→page→visual and report→model, and
/// the visual binds the model measure via `pbi_uses_field`.
#[tokio::test]
async fn index_pbip_source_builds_traversable_project_graph() {
    let root = TempDir::new().expect("tempdir");
    write_project(root.path());

    let db = connect_db(&root.path().join("data"), "pbip-graph")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_pbip_source(&pbip_source("proj"), root.path(), &queries, MAX_FILE_SIZE)
        .await
        .expect("index pbip source");

    let nodes = queries
        .select_powerbi_nodes(Some("proj"))
        .await
        .expect("select pbip nodes");
    let report = nodes
        .iter()
        .find(|n| n.name == "My")
        .expect("report node present");

    // report → page → visual via contains
    let contains = queries
        .query_graph_neighborhood(
            &report.id,
            TraversalDirection::Outgoing,
            3,
            50,
            &["pbi_contains"],
        )
        .await
        .expect("contains traversal");
    let contains_names: Vec<&str> = contains.nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(
        contains_names.contains(&"First Page"),
        "page reachable from report via contains: {contains_names:?}"
    );
    assert!(
        contains_names.contains(&"v1"),
        "visual reachable from report via contains: {contains_names:?}"
    );

    // report → semantic model via depends_on_model
    let depends = queries
        .query_graph_neighborhood(
            &report.id,
            TraversalDirection::Outgoing,
            2,
            50,
            &["pbi_depends_on_model"],
        )
        .await
        .expect("depends_on_model traversal");
    assert!(
        depends.nodes.iter().any(|n| n.name == "Sales Dataset"),
        "semantic model reachable from report via depends_on_model"
    );

    // visual → measure via uses_field
    let visual = nodes
        .iter()
        .find(|n| n.name == "v1")
        .expect("visual node present");
    let uses = queries
        .query_graph_neighborhood(
            &visual.id,
            TraversalDirection::Outgoing,
            2,
            50,
            &["pbi_uses_field"],
        )
        .await
        .expect("uses_field traversal");
    assert!(
        uses.nodes.iter().any(|n| n.name == "Total Sales"),
        "measure reachable from visual via uses_field"
    );
}

/// S-PGQ-03: re-indexing unchanged input is a no-op (all files reported as
/// unchanged, record count stable).
#[tokio::test]
async fn index_pbip_source_is_idempotent_on_unchanged_input() {
    let root = TempDir::new().expect("tempdir");
    write_project(root.path());

    let db = connect_db(&root.path().join("data"), "pbip-idempotent")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    let first = index_pbip_source(&pbip_source("proj"), root.path(), &queries, MAX_FILE_SIZE)
        .await
        .expect("first index");
    let after_first = queries
        .select_content_records(Some("pbip"))
        .await
        .expect("records after first")
        .len();

    let second = index_pbip_source(&pbip_source("proj"), root.path(), &queries, MAX_FILE_SIZE)
        .await
        .expect("second index");
    let after_second = queries
        .select_content_records(Some("pbip"))
        .await
        .expect("records after second")
        .len();

    assert!(first.ingested > 0, "first run ingests");
    assert_eq!(
        second.unchanged, second.total_files,
        "second run treats all files as unchanged"
    );
    assert_eq!(
        after_first, after_second,
        "record count stable across idempotent re-index"
    );
}

/// S-PGQ-04: the deletion sweep prunes records (and graph nodes) for files
/// removed from disk.
#[tokio::test]
async fn sweep_deleted_pbip_files_prunes_removed_files() {
    let root = TempDir::new().expect("tempdir");
    write_project(root.path());

    let db = connect_db(&root.path().join("data"), "pbip-sweep")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_pbip_source(&pbip_source("proj"), root.path(), &queries, MAX_FILE_SIZE)
        .await
        .expect("index pbip source");

    // Remove the visual file from disk, then sweep.
    fs::remove_file(
        root.path()
            .join("proj/My.Report/definition/pages/Page1/visuals/v1/visual.json"),
    )
    .expect("remove visual file");

    let removed = sweep_deleted_pbip_files(&pbip_source("proj"), root.path(), &queries)
        .await
        .expect("sweep deleted files");
    assert_eq!(removed, 1, "exactly one deleted file should be swept");

    let records = queries
        .select_content_records(Some("pbip"))
        .await
        .expect("records after sweep");
    assert!(
        !records
            .iter()
            .any(|r| r.file_path.ends_with("Page1/visuals/v1/visual.json")),
        "swept visual record should be gone"
    );
}
