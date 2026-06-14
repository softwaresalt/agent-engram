//! Integration tests for Power BI search ingestion (061.004-T).
//!
//! Validates the pure helper functions exported by [`powerbi_indexer`]:
//! hash-based change detection, deletion sweep, file collection, and
//! entity-summary extraction.  These tests run without a `CozoDB` instance,
//! mirroring the precedent set by `backlog_indexer_test.rs`.
//!
//! Tests: S-PIN-01 through S-PIN-13

use std::fs;
use tempfile::TempDir;

use engram::services::powerbi_indexer::{
    collect_powerbi_files, compute_deleted_paths, compute_file_hash, extract_entity_summaries,
};

#[cfg(feature = "cozo-backend")]
use engram::db::{connect_db, queries::CodeGraphQueries};
#[cfg(feature = "cozo-backend")]
use engram::models::PowerBiNodeKind;
#[cfg(feature = "cozo-backend")]
use engram::models::registry::{ContentSource, ContentSourceStatus};
#[cfg(feature = "cozo-backend")]
use engram::services::powerbi_indexer::index_powerbi_source;

// ── Hash helpers ──────────────────────────────────────────────────────────

/// S-PIN-01: `compute_file_hash` is deterministic for the same content.
#[test]
fn compute_file_hash_is_deterministic() {
    let content = b"{ \"tables\": [] }";
    let h1 = compute_file_hash(content);
    let h2 = compute_file_hash(content);
    assert_eq!(h1, h2, "same content must produce the same hash");
    assert_eq!(h1.len(), 64, "SHA-256 hex digest is 64 characters");
}

/// S-PIN-02: Different content produces a different hash.
#[test]
fn compute_file_hash_differs_for_changed_content() {
    let original = b"{ \"tables\": [] }";
    let changed = b"{ \"tables\": [{ \"name\": \"Sales\" }] }";
    let h1 = compute_file_hash(original);
    let h2 = compute_file_hash(changed);
    assert_ne!(h1, h2, "changed content must produce a different hash");
}

// ── Deletion sweep ────────────────────────────────────────────────────────

/// S-PIN-03: `compute_deleted_paths` detects a file that no longer exists.
#[test]
fn compute_deleted_paths_detects_removed_file() {
    let dir = TempDir::new().expect("tempdir");

    // Create one live file.
    let live_path = dir.path().join("live.json");
    fs::write(&live_path, r#"{"tables":[]}"#).expect("write live file");

    // Normalise paths to forward-slash workspace-relative form.
    let root = dir.path();
    let live_rel = live_path
        .strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let gone_rel = "gone.json".to_string();

    let known = vec![live_rel.clone(), gone_rel.clone()];
    let deleted = compute_deleted_paths(&known, root);

    assert_eq!(
        deleted.len(),
        1,
        "only gone.json should be returned as deleted"
    );
    assert!(
        deleted[0].contains("gone.json"),
        "deleted path should reference gone.json"
    );
}

/// S-PIN-04: When all known files still exist, no paths are returned as deleted.
#[test]
fn compute_deleted_paths_returns_empty_when_all_files_exist() {
    let dir = TempDir::new().expect("tempdir");
    let file = dir.path().join("model.bim");
    fs::write(&file, r#"{"model":{"tables":[]}}"#).expect("write file");

    let root = dir.path();
    let rel = file
        .strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");

    let deleted = compute_deleted_paths(&[rel], root);
    assert!(deleted.is_empty(), "no files should be reported deleted");
}

/// S-PIN-11: Deletion sweeps ignore paths that could escape the workspace root.
#[test]
fn compute_deleted_paths_ignores_escape_attempts() {
    let dir = TempDir::new().expect("tempdir");
    let absolute = dir
        .path()
        .join("outside.json")
        .to_string_lossy()
        .replace('\\', "/");

    let deleted = compute_deleted_paths(
        &[
            absolute,
            "../outside.json".to_string(),
            "gone.json".to_string(),
        ],
        dir.path(),
    );

    assert_eq!(
        deleted,
        vec!["gone.json".to_string()],
        "only safe workspace-relative paths should participate in deletion sweeps"
    );
}

// ── File collection ───────────────────────────────────────────────────────

/// S-PIN-05: `collect_powerbi_files` discovers `.json`, `.bim`, and `.tmdl`
/// files.
#[test]
fn collect_powerbi_files_finds_json_bim_and_tmdl() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("report.json"), "{}").expect("write json");
    fs::write(dir.path().join("model.bim"), "{}").expect("write bim");
    fs::write(dir.path().join("table.tmdl"), "table Sales").expect("write tmdl");
    fs::write(dir.path().join("ignore.md"), "# doc").expect("write md");

    let files = collect_powerbi_files(dir.path());
    assert_eq!(files.len(), 3, "should find exactly 3 Power BI files");

    let names: Vec<_> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap())
        .collect();
    assert!(names.contains(&"report.json"), "report.json must be found");
    assert!(names.contains(&"model.bim"), "model.bim must be found");
    assert!(names.contains(&"table.tmdl"), "table.tmdl must be found");
    assert!(
        !names.contains(&"ignore.md"),
        "markdown files must be ignored"
    );
}

/// S-PIN-06: `collect_powerbi_files` is recursive.
#[test]
fn collect_powerbi_files_is_recursive() {
    let dir = TempDir::new().expect("tempdir");
    let sub = dir.path().join("subfolder");
    fs::create_dir_all(&sub).expect("create subfolder");
    fs::write(sub.join("deep.json"), "{}").expect("write nested json");
    fs::write(dir.path().join("top.json"), "{}").expect("write top-level json");

    let files = collect_powerbi_files(dir.path());
    assert_eq!(files.len(), 2, "should find files in subfolders too");
}

/// S-PIN-12: `.pbip` descriptors are ignored until the indexer can parse them.
#[test]
fn collect_powerbi_files_ignores_pbip_descriptors() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("project.pbip"), "{}").expect("write pbip");
    fs::write(dir.path().join("report.json"), "{}").expect("write json");
    fs::write(dir.path().join("model.bim"), "{}").expect("write bim");

    let files = collect_powerbi_files(dir.path());
    let names: Vec<_> = files
        .iter()
        .map(|path| path.file_name().unwrap().to_str().unwrap())
        .collect();

    assert!(names.contains(&"report.json"));
    assert!(names.contains(&"model.bim"));
    assert!(
        !names.contains(&"project.pbip"),
        "pbip descriptors should not be collected until they yield summaries"
    );
}

// ── Entity summary extraction ─────────────────────────────────────────────

/// S-PIN-07: Report JSON produces entity summaries for pages and visuals.
#[test]
fn extract_entity_summaries_from_report_json() {
    let report_json = r#"{
        "displayName": "Sales Dashboard",
        "reportSections": [
            {
                "displayName": "Overview",
                "ordinal": 1,
                "visualContainers": [
                    { "visualType": "barChart" },
                    { "visualType": "lineChart" }
                ]
            }
        ]
    }"#;

    let summaries = extract_entity_summaries(report_json, "reports/sales/report.json");
    assert!(
        !summaries.is_empty(),
        "report JSON should produce at least one entity summary"
    );

    // Expect one page summary and two visual summaries.
    let page_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_page")
        .count();
    let visual_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_visual")
        .count();

    assert_eq!(page_count, 1, "should have one page summary");
    assert_eq!(visual_count, 2, "should have two visual summaries");

    let page = summaries
        .iter()
        .find(|(k, _, _, _)| k == "powerbi_page")
        .unwrap();
    assert_eq!(page.1, "Overview", "page name should be Overview");
}

/// S-PIN-08: `model.bim` JSON produces entity summaries for tables and measures.
#[test]
fn extract_entity_summaries_from_model_bim() {
    let model_json = r#"{
        "model": {
            "tables": [
                {
                    "name": "Sales",
                    "columns": [
                        { "name": "Date", "dataType": "dateTime" }
                    ],
                    "measures": [
                        { "name": "Total Sales", "expression": "SUM(Sales[Amount])" }
                    ]
                }
            ]
        }
    }"#;

    let summaries = extract_entity_summaries(model_json, "semantic/model.bim");
    assert!(
        !summaries.is_empty(),
        "model.bim JSON should produce at least one entity summary"
    );

    let table_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_table")
        .count();
    let measure_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_measure")
        .count();
    let semantic_model_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_semantic_model")
        .count();

    assert_eq!(table_count, 1, "should have one table summary");
    assert_eq!(measure_count, 1, "should have one measure summary");
    assert_eq!(
        semantic_model_count, 1,
        "should have one semantic-model summary"
    );

    let table = summaries
        .iter()
        .find(|(k, _, _, _)| k == "powerbi_table")
        .unwrap();
    assert_eq!(table.1, "Sales", "table name should be Sales");
}

/// S-PIN-16: Ref-only `model.tmdl` files still produce a semantic-model
/// summary so the canonical model file is searchable.
#[test]
fn extract_entity_summaries_from_ref_only_model_tmdl() {
    let tmdl = r"
model Sales Dataset

ref table Sales
ref relationship SalesToProducts
";

    let summaries =
        extract_entity_summaries(tmdl, "models/Sales.SemanticModel/definition/model.tmdl");
    let semantic_models: Vec<_> = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_semantic_model")
        .collect();

    assert_eq!(
        semantic_models.len(),
        1,
        "ref-only model.tmdl should still yield one semantic-model summary"
    );
    assert_eq!(semantic_models[0].1, "Sales Dataset");
}

/// S-PIN-18: `expressions.tmdl` files produce expression summaries through the
/// shared semantic-model path.
#[test]
fn extract_entity_summaries_from_tmdl_expressions() {
    let tmdl = r#"
expression SynapseSqlServer = "dp-da-synw-t-cus-01-ondemand.sql.azuresynapse.net" meta [IsParameterQuery=true, Type="Text"]

expression SynapseDatabase = "ILSOS_EDW" meta [IsParameterQuery=true, Type="Text"]
"#;

    let summaries = extract_entity_summaries(
        tmdl,
        "models/Sales.SemanticModel/definition/expressions.tmdl",
    );
    let expression_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_expression")
        .count();

    assert_eq!(expression_count, 2, "should have two expression summaries");
}

/// S-PIN-09: Unknown JSON that is neither report nor model returns empty summaries.
#[test]
fn extract_entity_summaries_returns_empty_for_unknown_json() {
    let unknown = r#"{"foo": "bar", "baz": 42}"#;
    let summaries = extract_entity_summaries(unknown, "unknown/file.json");
    assert!(
        summaries.is_empty(),
        "unknown JSON should produce no entity summaries"
    );
}

/// S-PIN-10: Invalid JSON returns empty summaries without panicking.
#[test]
fn extract_entity_summaries_returns_empty_for_invalid_json() {
    let bad = "{ this is not json at all !!!";
    let summaries = extract_entity_summaries(bad, "bad/file.json");
    assert!(
        summaries.is_empty(),
        "invalid JSON should produce no entity summaries"
    );
}

/// S-PIN-11: TMDL content produces table and measure summaries through the
/// shared semantic model path.
#[test]
fn extract_entity_summaries_from_tmdl() {
    let tmdl = r"
model Sales Model

table Sales
  column Amount
    dataType: double
  measure 'Total Sales' = SUM ( Sales[Amount] )
";

    let summaries = extract_entity_summaries(tmdl, "models/Sales.SemanticModel/definition");
    assert!(
        !summaries.is_empty(),
        "TMDL content should produce entity summaries"
    );

    let table_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_table")
        .count();
    let measure_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_measure")
        .count();

    assert_eq!(table_count, 1, "should have one table summary");
    assert_eq!(measure_count, 1, "should have one measure summary");
}

/// S-PIN-12: Windows-style `definition` folder paths still route TMDL content
/// through the semantic-model extractor.
#[test]
fn extract_entity_summaries_from_tmdl_windows_definition_path() {
    let tmdl = r"
model Sales Model

table Sales
  column Amount
    dataType: double
  measure 'Total Sales' = SUM ( Sales[Amount] )
";

    let summaries = extract_entity_summaries(tmdl, r"models\Sales.SemanticModel\definition");
    assert!(
        !summaries.is_empty(),
        "TMDL content should produce entity summaries for Windows-style definition paths"
    );

    let table_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_table")
        .count();
    let measure_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_measure")
        .count();

    assert_eq!(table_count, 1, "should have one table summary");
    assert_eq!(measure_count, 1, "should have one measure summary");
}

#[cfg(feature = "cozo-backend")]
fn powerbi_source(path: &str) -> ContentSource {
    ContentSource {
        content_type: "powerbi".to_string(),
        language: None,
        path: path.to_string(),
        pattern: None,
        optional: false,
        status: ContentSourceStatus::Active,
    }
}

/// S-PIN-13: Overlapping Power BI sources keep distinct content records.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn index_powerbi_source_scopes_records_by_source_path() {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    let reports = workspace.join("reports");
    fs::create_dir_all(&reports).expect("create reports dir");
    fs::write(
        reports.join("report.json"),
        r#"{
            "displayName": "Sales Dashboard",
            "reportSections": [
                { "displayName": "Overview", "ordinal": 1, "visualContainers": [] }
            ]
        }"#,
    )
    .expect("write report");

    let db = connect_db(&root.path().join("data"), "powerbi-source-scope")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_powerbi_source(&powerbi_source("."), &workspace, &queries, 1_048_576)
        .await
        .expect("index root source");
    index_powerbi_source(&powerbi_source("reports"), &workspace, &queries, 1_048_576)
        .await
        .expect("index nested source");

    let records = queries
        .select_content_records(Some("powerbi"))
        .await
        .expect("select content records");
    let report_records: Vec<_> = records
        .into_iter()
        .filter(|record| record.file_path == "reports/report.json")
        .collect();

    assert_eq!(
        report_records.len(),
        2,
        "overlapping sources should retain distinct records for the same file path"
    );
    assert!(
        report_records
            .iter()
            .any(|record| record.source_path == "."),
        "root source record should be preserved"
    );
    assert!(
        report_records
            .iter()
            .any(|record| record.source_path == "reports"),
        "nested source record should be preserved"
    );
}

/// S-PIN-14: Multi-file TMDL models share one semantic-model identity across
/// table and relationship files.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn index_powerbi_source_unifies_multifile_tmdl_graph_nodes() {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    let definition = workspace
        .join("models")
        .join("Sales.SemanticModel")
        .join("definition");
    let tables = definition.join("Tables");
    fs::create_dir_all(&tables).expect("create tmdl directories");

    fs::write(
        definition.join("model.tmdl"),
        r"
model Sales
relationship Sales.ProductID -> Products.ID
",
    )
    .expect("write model.tmdl");
    fs::write(
        tables.join("Sales.tmdl"),
        r"
table Sales
  column ProductID
    dataType: int64
",
    )
    .expect("write sales table");
    fs::write(
        tables.join("Products.tmdl"),
        r"
table Products
  column ID
    dataType: int64
",
    )
    .expect("write products table");

    let db = connect_db(&root.path().join("data"), "powerbi-tmdl-graph-identity")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_powerbi_source(&powerbi_source("models"), &workspace, &queries, 1_048_576)
        .await
        .expect("index tmdl source");

    let nodes = queries
        .select_powerbi_nodes(Some("models"))
        .await
        .expect("select powerbi nodes");

    let semantic_models: Vec<_> = nodes
        .iter()
        .filter(|node| node.kind == PowerBiNodeKind::SemanticModel)
        .collect();
    let tables: Vec<_> = nodes
        .iter()
        .filter(|node| node.kind == PowerBiNodeKind::Table)
        .collect();
    let relationships: Vec<_> = nodes
        .iter()
        .filter(|node| node.kind == PowerBiNodeKind::Relationship)
        .collect();

    assert_eq!(
        semantic_models.len(),
        1,
        "all TMDL files in one semantic model should converge on a single semantic-model node"
    );
    assert_eq!(
        tables.len(),
        2,
        "expected one table node per TMDL table file"
    );
    assert_eq!(
        relationships.len(),
        1,
        "relationship declarations should produce one relationship node"
    );
}

/// S-PIN-15: Multi-file TMDL models keep one semantic-model node even when
/// explicit model names differ across files.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn index_powerbi_source_unifies_multifile_tmdl_graph_nodes_with_mixed_model_names() {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    let definition = workspace
        .join("models")
        .join("Sales.SemanticModel")
        .join("definition");
    let tables = definition.join("Tables");
    fs::create_dir_all(&tables).expect("create tmdl directories");

    fs::write(
        definition.join("model.tmdl"),
        r"
model Sales Dataset
relationship Sales.ProductID -> Products.ID
",
    )
    .expect("write model.tmdl");
    fs::write(
        tables.join("Sales.tmdl"),
        r"
table Sales
  column ProductID
    dataType: int64
",
    )
    .expect("write sales table");
    fs::write(
        tables.join("Products.tmdl"),
        r"
table Products
  column ID
    dataType: int64
",
    )
    .expect("write products table");

    let db = connect_db(
        &root.path().join("data"),
        "powerbi-tmdl-graph-stable-identity",
    )
    .await
    .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_powerbi_source(&powerbi_source("models"), &workspace, &queries, 1_048_576)
        .await
        .expect("index tmdl source");

    let nodes = queries
        .select_powerbi_nodes(Some("models"))
        .await
        .expect("select powerbi nodes");

    let semantic_models: Vec<_> = nodes
        .iter()
        .filter(|node| node.kind == PowerBiNodeKind::SemanticModel)
        .collect();

    assert_eq!(
        semantic_models.len(),
        1,
        "stable TMDL identity should not split one semantic model into multiple graph nodes"
    );
}

/// S-PIN-17: A ref-only `model.tmdl` file still creates a searchable semantic
/// model record and graph node.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn index_powerbi_source_indexes_ref_only_model_tmdl() {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    let definition = workspace
        .join("models")
        .join("Sales.SemanticModel")
        .join("definition");
    fs::create_dir_all(&definition).expect("create definition dir");

    fs::write(
        definition.join("model.tmdl"),
        r"
model Sales Dataset

ref table Sales
ref relationship SalesToProducts
",
    )
    .expect("write model.tmdl");

    let db = connect_db(&root.path().join("data"), "powerbi-tmdl-model-shell")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_powerbi_source(&powerbi_source("models"), &workspace, &queries, 1_048_576)
        .await
        .expect("index tmdl source");

    let records = queries
        .select_content_records(Some("powerbi"))
        .await
        .expect("select content records");
    let semantic_model_records: Vec<_> = records
        .iter()
        .filter(|record| record.record_kind == "powerbi_semantic_model")
        .collect();
    assert_eq!(
        semantic_model_records.len(),
        1,
        "ref-only model.tmdl should still create one semantic-model content record"
    );

    let nodes = queries
        .select_powerbi_nodes(Some("models"))
        .await
        .expect("select powerbi nodes");
    let semantic_models: Vec<_> = nodes
        .iter()
        .filter(|node| node.kind == PowerBiNodeKind::SemanticModel)
        .collect();
    assert_eq!(
        semantic_models.len(),
        1,
        "ref-only model.tmdl should still create one semantic-model graph node"
    );
}

/// S-PIN-19: `expressions.tmdl` files create searchable expression records and
/// expression graph nodes.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn index_powerbi_source_indexes_tmdl_expressions() {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    let definition = workspace
        .join("models")
        .join("Sales.SemanticModel")
        .join("definition");
    fs::create_dir_all(&definition).expect("create definition dir");

    fs::write(
        definition.join("expressions.tmdl"),
        r#"
expression SynapseSqlServer = "dp-da-synw-t-cus-01-ondemand.sql.azuresynapse.net" meta [IsParameterQuery=true, Type="Text"]

expression SynapseDatabase = "ILSOS_EDW" meta [IsParameterQuery=true, Type="Text"]
"#,
    )
    .expect("write expressions.tmdl");

    let db = connect_db(&root.path().join("data"), "powerbi-tmdl-expressions")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_powerbi_source(&powerbi_source("models"), &workspace, &queries, 1_048_576)
        .await
        .expect("index tmdl source");

    let records = queries
        .select_content_records(Some("powerbi"))
        .await
        .expect("select content records");
    let expression_records: Vec<_> = records
        .iter()
        .filter(|record| record.record_kind == "powerbi_expression")
        .collect();
    assert_eq!(
        expression_records.len(),
        2,
        "expressions.tmdl should create one content record per expression"
    );

    let nodes = queries
        .select_powerbi_nodes(Some("models"))
        .await
        .expect("select powerbi nodes");
    let expression_nodes: Vec<_> = nodes
        .iter()
        .filter(|node| node.kind == PowerBiNodeKind::Expression)
        .collect();
    assert_eq!(
        expression_nodes.len(),
        2,
        "expressions.tmdl should create one graph node per expression"
    );
}
