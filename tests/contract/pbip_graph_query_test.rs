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
use engram::models::registry::{ContentSource, ContentSourceStatus};
use engram::models::{PowerBiNode, PowerBiNodeKind, TraversalDirection};
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

#[cfg(unix)]
fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
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

/// 110-S U2: an unavailable PBIP source root cannot certify a mass deletion.
/// Records remain intact until a later authoritative pass or source removal.
#[tokio::test]
async fn sweep_deleted_pbip_files_retains_records_when_source_root_is_unavailable() {
    let root = TempDir::new().expect("tempdir");
    write_project(root.path());

    let db = connect_db(&root.path().join("data"), "pbip-sweep-missing-root")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    let source = pbip_source("proj");

    index_pbip_source(&source, root.path(), &queries, MAX_FILE_SIZE)
        .await
        .expect("index pbip source");
    let before = queries
        .select_content_records(Some("pbip"))
        .await
        .expect("records before unavailable root");
    assert!(!before.is_empty(), "fixture must create PBIP records");

    fs::rename(
        root.path().join("proj"),
        root.path().join("proj-unavailable"),
    )
    .expect("make source root unavailable");

    let removed = sweep_deleted_pbip_files(&source, root.path(), &queries)
        .await
        .expect("sweep unavailable source");
    assert_eq!(
        removed, 0,
        "an unavailable source root must suppress the deletion sweep"
    );

    let after = queries
        .select_content_records(Some("pbip"))
        .await
        .expect("records after unavailable root");
    assert_eq!(
        after.len(),
        before.len(),
        "all PBIP records must survive a non-authoritative source pass"
    );
}

/// 110-S U2: when a complete traversal chooses a real directory over a
/// physically-live alias, the prior alias path is stale deletion evidence.
#[tokio::test]
async fn sweep_deleted_pbip_files_removes_alias_superseded_on_complete_pass() {
    let root = TempDir::new().expect("tempdir");
    write(
        root.path(),
        "proj/a/live.pbip",
        r#"{"version":"1.0","artifacts":[]}"#,
    );

    let db = connect_db(&root.path().join("data"), "pbip-sweep-alias")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    let source = pbip_source("proj");

    index_pbip_source(&source, root.path(), &queries, MAX_FILE_SIZE)
        .await
        .expect("index alias path");
    let alias_path = "proj/a/live.pbip";
    assert!(
        queries
            .select_content_records(Some("pbip"))
            .await
            .expect("records before alias replacement")
            .iter()
            .any(|record| record.file_path == alias_path),
        "fixture must index the original alias path"
    );

    let real_dir = root.path().join("proj/z");
    fs::rename(root.path().join("proj/a"), &real_dir).expect("rename alias winner");
    if let Err(error) = symlink_dir(&real_dir, &root.path().join("proj/a")) {
        eprintln!(
            "skipping alias-supersession assertion: cannot create directory symlink: {error}"
        );
        return;
    }

    let removed = sweep_deleted_pbip_files(&source, root.path(), &queries)
        .await
        .expect("sweep stale alias");
    assert_eq!(
        removed, 1,
        "the complete pass must remove the superseded alias path"
    );
    assert!(
        !queries
            .select_content_records(Some("pbip"))
            .await
            .expect("records after alias replacement")
            .iter()
            .any(|record| record.file_path == alias_path),
        "the superseded alias record must be removed"
    );
}

/// S-PGQ-05 (Issue A regression): a legacy `powerbi` source registered at the
/// same registry path keeps its graph nodes across a PBIP re-index. PBIP graph
/// deletion must be scoped to PBIP-owned file paths, not the whole source.
#[tokio::test]
async fn index_pbip_source_preserves_legacy_powerbi_nodes_at_same_path() {
    let root = TempDir::new().expect("tempdir");
    write_project(root.path());

    let db = connect_db(&root.path().join("data"), "pbip-legacy-coexist")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    // A legacy `powerbi` node sharing the same registry source_path ("proj"),
    // anchored to a file the PBIP walker never collects (`.bim` is not a PBIP
    // extension). The independence boundary requires this node to survive.
    let legacy = PowerBiNode {
        id: "pbi_legacy_fixture".to_string(),
        name: "LegacyModel".to_string(),
        kind: PowerBiNodeKind::SemanticModel,
        file_path: "proj/Legacy.SemanticModel/model.bim".to_string(),
        source_path: "proj".to_string(),
        content_hash: "deadbeefdeadbeef".to_string(),
        ingested_at: chrono::Utc::now(),
    };
    queries
        .upsert_powerbi_nodes(std::slice::from_ref(&legacy))
        .await
        .expect("seed legacy powerbi node");

    // First PBIP index: must not touch the pre-existing legacy node.
    index_pbip_source(&pbip_source("proj"), root.path(), &queries, MAX_FILE_SIZE)
        .await
        .expect("first pbip index");
    assert!(
        queries
            .select_powerbi_nodes(Some("proj"))
            .await
            .expect("nodes after first index")
            .iter()
            .any(|n| n.id == "pbi_legacy_fixture"),
        "legacy powerbi node must survive the first PBIP index"
    );

    // Change a PBIP file so the next index triggers a full rebuild (delete +
    // re-emit) of the PBIP graph.
    write(
        root.path(),
        "proj/My.Report/definition/pages/Page1/visuals/v1/visual.json",
        r#"{"name":"v1","visual":{"visualType":"barChart"}}"#,
    );
    index_pbip_source(&pbip_source("proj"), root.path(), &queries, MAX_FILE_SIZE)
        .await
        .expect("second pbip index");

    let nodes = queries
        .select_powerbi_nodes(Some("proj"))
        .await
        .expect("nodes after rebuild");
    assert!(
        nodes.iter().any(|n| n.id == "pbi_legacy_fixture"),
        "legacy powerbi node must survive a PBIP re-index rebuild"
    );
    // The PBIP graph itself is rebuilt — the report node is present again.
    assert!(
        nodes
            .iter()
            .any(|n| n.kind == PowerBiNodeKind::Report && n.name == "My"),
        "PBIP report node should be rebuilt by the re-index"
    );
}

/// S-PGQ-06 (Issue C regression): an oversized `.tmdl` is excluded from the
/// snapshot and therefore from semantic-model extraction, exactly as it is
/// excluded from every other PBIP ingestion path.
#[tokio::test]
async fn index_pbip_source_skips_oversized_tmdl() {
    let root = TempDir::new().expect("tempdir");
    write(
        root.path(),
        "proj/My.pbip",
        r#"{"version":"1.0","artifacts":[{"report":{"path":"My.Report"}}]}"#,
    );
    write(
        root.path(),
        "proj/My.Report/definition.pbir",
        r#"{"version":"1.0","datasetReference":{"byPath":{"path":"../My.SemanticModel"}}}"#,
    );
    write(root.path(), "proj/My.Report/definition/report.json", "{}");
    write(root.path(), "proj/My.SemanticModel/definition.pbism", "{}");
    // Small in-bounds model file declares the model name.
    write(
        root.path(),
        "proj/My.SemanticModel/definition/model.tmdl",
        "model Sales Dataset\n",
    );
    // Oversized table file: its entities must never be indexed under the cap.
    let padding = "/// filler comment line to inflate the file size\n".repeat(200);
    write(
        root.path(),
        "proj/My.SemanticModel/definition/tables/Big.tmdl",
        &format!("table BigTable\n  column HugeColumn\n    dataType: string\n{padding}"),
    );

    // Cap large enough for the small descriptors, far below the padded file.
    let cap: u64 = 2_048;

    let db = connect_db(&root.path().join("data"), "pbip-oversized-tmdl")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_pbip_source(&pbip_source("proj"), root.path(), &queries, cap)
        .await
        .expect("index pbip source");

    let records = queries
        .select_content_records(Some("pbip"))
        .await
        .expect("select pbip records");
    assert!(
        !records
            .iter()
            .any(|r| r.content.contains("BigTable") || r.content.contains("HugeColumn")),
        "oversized .tmdl entities must not be indexed"
    );

    let nodes = queries
        .select_powerbi_nodes(Some("proj"))
        .await
        .expect("select pbip nodes");
    assert!(
        !nodes.iter().any(|n| n.name == "BigTable"),
        "no graph node should come from the oversized .tmdl"
    );
    // The in-bounds model file is still indexed, proving the cap is selective.
    assert!(
        nodes
            .iter()
            .any(|n| n.kind == PowerBiNodeKind::SemanticModel && n.name == "Sales Dataset"),
        "the in-bounds semantic model should still be indexed"
    );
}

/// S-PGQ-07 (Issue D regression): a `pages.json` that exists but cannot be
/// parsed yields the generic `pbip_file` coverage record, not a misleading
/// `pbip_page_order` record.
#[tokio::test]
async fn index_pbip_source_unparseable_pages_falls_back_to_pbip_file() {
    let root = TempDir::new().expect("tempdir");
    write(
        root.path(),
        "proj/My.pbip",
        r#"{"version":"1.0","artifacts":[{"report":{"path":"My.Report"}}]}"#,
    );
    write(root.path(), "proj/My.Report/definition/report.json", "{}");
    write(
        root.path(),
        "proj/My.Report/definition/pages/pages.json",
        "{ this is not valid json",
    );

    let db = connect_db(&root.path().join("data"), "pbip-bad-pages")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_pbip_source(&pbip_source("proj"), root.path(), &queries, MAX_FILE_SIZE)
        .await
        .expect("index pbip source");

    let records = queries
        .select_content_records(Some("pbip"))
        .await
        .expect("select pbip records");

    assert!(
        !records.iter().any(|r| r.record_kind == "pbip_page_order"),
        "no page-order record should be emitted for an unparseable pages.json"
    );
    assert!(
        records.iter().any(|r| r.record_kind == "pbip_file"
            && r.file_path == "proj/My.Report/definition/pages/pages.json"),
        "the generic pbip_file coverage record should cover the unparseable pages.json"
    );
}
