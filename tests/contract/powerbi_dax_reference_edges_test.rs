//! Contract tests for Power BI DAX reference edges (`085.003-T`, P3).
//!
//! Indexing a multi-file TMDL semantic model must emit `pbi_uses_field`
//! reference edges from each measure / calculated column to the columns and
//! measures its DAX expression references, resolved against the whole model
//! scope (union of every sibling `.tmdl` file), so that cross-file references
//! such as a `Sales.tmdl` measure using `'Date'[Year]` link correctly.
//!
//! Tests: S-DAXREF-01 through S-DAXREF-05.

#![cfg(feature = "cozo-backend")]

use std::fs;
use tempfile::TempDir;

use engram::db::{connect_db, queries::CodeGraphQueries};
use engram::models::TraversalDirection;
use engram::models::powerbi_graph::{PowerBiNode, PowerBiNodeKind};
use engram::models::registry::{ContentSource, ContentSourceStatus};
use engram::services::powerbi_indexer::index_powerbi_source;

const MAX_FILE_SIZE: u64 = 1_048_576;

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

/// Build a two-file TMDL model fixture (`Sales.tmdl` + `Date.tmdl`) sharing one
/// `definition/` model scope, index it, and return every indexed Power BI node.
async fn index_fixture_nodes() -> (TempDir, CodeGraphQueries, Vec<PowerBiNode>) {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    let tables = workspace
        .join("models")
        .join("Sales.SemanticModel")
        .join("definition")
        .join("tables");
    fs::create_dir_all(&tables).expect("create tmdl directories");

    // Sales table: two base columns, one calculated column, and three measures
    // exercising measure->column, measure->measure, and cross-file references.
    fs::write(
        tables.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20column Region\n\
         \x20\x20\x20\x20dataType: string\n\
         \x20\x20column FullRegion = Sales[Region]\n\
         \x20\x20measure 'Total Sales' = SUM(Sales[Amount])\n\
         \x20\x20measure 'Sales Ratio' = [Total Sales] / SUM(Sales[Amount])\n\
         \x20\x20measure 'Sales By Year' = CALCULATE([Total Sales], 'Date'[Year])\n",
    )
    .expect("write Sales.tmdl");

    // Date table lives in a sibling file of the same model scope.
    fs::write(
        tables.join("Date.tmdl"),
        "table Date\n\
         \x20\x20column Year\n\
         \x20\x20\x20\x20dataType: int64\n\
         \x20\x20column MonthName\n\
         \x20\x20\x20\x20dataType: string\n",
    )
    .expect("write Date.tmdl");

    let db = connect_db(&root.path().join("data"), "powerbi-dax-reference-edges")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_powerbi_source(
        &powerbi_source("models"),
        &workspace,
        &queries,
        MAX_FILE_SIZE,
    )
    .await
    .expect("index tmdl source");

    let nodes = queries
        .select_powerbi_nodes(Some("models"))
        .await
        .expect("select powerbi nodes");

    (root, queries, nodes)
}

fn node_id<'a>(nodes: &'a [PowerBiNode], name: &str, kind: PowerBiNodeKind) -> &'a str {
    let Some(node) = nodes
        .iter()
        .find(|node| node.name == name && node.kind == kind)
    else {
        panic!("expected {kind:?} node named {name:?} to be indexed");
    };
    node.id.as_str()
}

/// Return the outgoing `pbi_uses_field` neighbour names of `source_id`.
async fn uses_field_targets(queries: &CodeGraphQueries, source_id: &str) -> Vec<String> {
    let result = queries
        .query_graph_neighborhood(
            source_id,
            TraversalDirection::Outgoing,
            1,
            50,
            &["pbi_uses_field"],
        )
        .await
        .expect("query_graph_neighborhood must succeed");
    result
        .nodes
        .into_iter()
        .filter(|node| node.id != source_id)
        .map(|node| node.name)
        .collect()
}

/// S-DAXREF-01: A measure referencing a same-table column emits a
/// `pbi_uses_field` edge to that column.
#[tokio::test]
async fn measure_references_same_table_column() {
    let (_tmp, queries, nodes) = index_fixture_nodes().await;
    let total_sales = node_id(&nodes, "Total Sales", PowerBiNodeKind::Measure);
    let targets = uses_field_targets(&queries, total_sales).await;
    assert!(
        targets.iter().any(|name| name == "Amount"),
        "Total Sales measure should reference the Amount column; got {targets:?}"
    );
}

/// S-DAXREF-02: A calculated column referencing a same-table column emits a
/// `pbi_uses_field` edge to that column.
#[tokio::test]
async fn calculated_column_references_column() {
    let (_tmp, queries, nodes) = index_fixture_nodes().await;
    let full_region = node_id(&nodes, "FullRegion", PowerBiNodeKind::Column);
    let targets = uses_field_targets(&queries, full_region).await;
    assert!(
        targets.iter().any(|name| name == "Region"),
        "FullRegion calculated column should reference the Region column; got {targets:?}"
    );
}

/// S-DAXREF-03: A measure referencing another measure via `[Measure]` emits a
/// measure->measure `pbi_uses_field` edge.
#[tokio::test]
async fn measure_references_measure() {
    let (_tmp, queries, nodes) = index_fixture_nodes().await;
    let ratio = node_id(&nodes, "Sales Ratio", PowerBiNodeKind::Measure);
    let targets = uses_field_targets(&queries, ratio).await;
    assert!(
        targets.iter().any(|name| name == "Total Sales"),
        "Sales Ratio measure should reference the Total Sales measure; got {targets:?}"
    );
    assert!(
        targets.iter().any(|name| name == "Amount"),
        "Sales Ratio measure should also reference the Amount column; got {targets:?}"
    );
}

/// S-DAXREF-04: A qualified cross-file reference (`'Date'[Year]`) resolves
/// against the whole model scope, not just the file being indexed.
#[tokio::test]
async fn measure_resolves_cross_file_reference() {
    let (_tmp, queries, nodes) = index_fixture_nodes().await;
    let by_year = node_id(&nodes, "Sales By Year", PowerBiNodeKind::Measure);
    let targets = uses_field_targets(&queries, by_year).await;
    assert!(
        targets.iter().any(|name| name == "Year"),
        "Sales By Year measure should reference the cross-file Date[Year] column; got {targets:?}"
    );
    assert!(
        targets.iter().any(|name| name == "Total Sales"),
        "Sales By Year measure should reference the Total Sales measure; got {targets:?}"
    );
}

/// S-DAXREF-05: Base (non-calculated) columns emit no `pbi_uses_field` edges.
#[tokio::test]
async fn base_column_has_no_reference_edges() {
    let (_tmp, queries, nodes) = index_fixture_nodes().await;
    let amount = node_id(&nodes, "Amount", PowerBiNodeKind::Column);
    let targets = uses_field_targets(&queries, amount).await;
    assert!(
        targets.is_empty(),
        "base column Amount should have no reference edges; got {targets:?}"
    );
}
