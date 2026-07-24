//! Integration tests for the notebook lineage write-path (095-F, Unit U4,
//! task 095.015-T).
//!
//! Exercises `index_notebook_source` end-to-end: routing notebook cells to the
//! Spark-lineage extractors, flattening candidates to directional
//! `lineage_derives_from` edges with edge-driven `dataset_node` creation and
//! per-cell evidence, the incremental scope-delete on re-index, and the
//! unconditional `lineage_index_state` freshness stamp.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[cfg(feature = "cozo-backend")]
use engram::db::{connect_db, queries::CodeGraphQueries};
#[cfg(feature = "cozo-backend")]
use engram::models::lineage::{CURRENT_EXTRACTOR_VERSION, LineageAuthorityContext};
#[cfg(feature = "cozo-backend")]
use engram::models::registry::{ContentSource, ContentSourceStatus};
#[cfg(feature = "cozo-backend")]
use engram::services::notebook_indexer::index_notebook_source;

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

#[cfg(feature = "cozo-backend")]
fn write_notebook(workspace: &Path, name: &str, json: &str) {
    let dir = workspace.join("notebooks");
    fs::create_dir_all(&dir).expect("create notebooks dir");
    fs::write(dir.join(name), json).expect("write notebook fixture");
}

#[cfg(feature = "cozo-backend")]
fn trusted_ctx() -> LineageAuthorityContext {
    let mut catalogs = BTreeMap::new();
    catalogs.insert("main".to_owned(), "metastore-prod".to_owned());
    LineageAuthorityContext::new(catalogs, vec!["s3://bucket".to_owned()])
}

#[cfg(feature = "cozo-backend")]
fn table_id(literal: &str) -> String {
    trusted_ctx()
        .resolve_table(literal)
        .expect("literal resolves under the trusted context")
        .id
}

/// U4-1 (Review comment D1): a cell containing only a bare read with no
/// downstream write produces zero `dataset_node`s and zero `lineage_edge`s
/// (edge-driven node creation), yet the notebook is still version-stamped.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn bare_read_produces_no_nodes_or_edges_but_stamps_version() {
    let root = tempfile::TempDir::new().expect("tempdir");
    write_notebook(
        root.path(),
        "bare.ipynb",
        r#"{"cells":[{"cell_type":"code","source":"df = spark.table(\"main.sales.orders\")"}],"metadata":{"language_info":{"name":"python"}}}"#,
    );

    let db = connect_db(&root.path().join("data"), "lineage-bare")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    let result = index_notebook_source(
        &notebook_source("notebooks"),
        root.path(),
        &queries,
        1_048_576,
        &trusted_ctx(),
    )
    .await
    .expect("index notebook source");
    assert_eq!(result.ingested, 1);

    assert!(
        queries
            .select_lineage_edges()
            .await
            .expect("edges")
            .is_empty(),
        "a bare read must emit no edges"
    );
    assert!(
        queries
            .select_dataset_node_ids()
            .await
            .expect("nodes")
            .is_empty(),
        "edge-driven: no edge ⇒ no dataset_node (D1)"
    );
    assert_eq!(
        queries
            .lineage_index_version("notebooks/bare.ipynb")
            .await
            .expect("version"),
        Some(CURRENT_EXTRACTOR_VERSION.to_owned()),
        "even a zero-lineage notebook is version-stamped (AR-03)"
    );
}

/// U4-2 (cycle-5 F3): a multi-source CTAS persists two directional edges sharing
/// the write target, each with per-cell evidence; re-indexing a changed notebook
/// with one lineage cell removed drops exactly that cell's edge and GCs the
/// now-orphaned node.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn multi_source_ctas_shares_target_and_reindex_drops_removed_cell() {
    let root = tempfile::TempDir::new().expect("tempdir");
    // v1: a multi-source CTAS (summary ← orders, returns) plus a second CTAS
    // (daily ← orders).
    write_notebook(
        root.path(),
        "nb.ipynb",
        r#"{"cells":[
            {"cell_type":"code","source":"%%sql\nCREATE TABLE main.sales.summary AS SELECT a.x FROM main.sales.orders JOIN main.sales.returns ON a.id = b.id"},
            {"cell_type":"code","source":"%%sql\nCREATE TABLE main.sales.daily AS SELECT x FROM main.sales.orders"}
        ],"metadata":{"language_info":{"name":"python"}}}"#,
    );

    let db = connect_db(&root.path().join("data"), "lineage-multi")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_notebook_source(
        &notebook_source("notebooks"),
        root.path(),
        &queries,
        1_048_576,
        &trusted_ctx(),
    )
    .await
    .expect("index v1");

    let summary = table_id("main.sales.summary");
    let orders = table_id("main.sales.orders");
    let returns = table_id("main.sales.returns");
    let daily = table_id("main.sales.daily");

    let edges = queries.select_lineage_edges().await.expect("edges v1");
    assert_eq!(
        edges.len(),
        3,
        "3 edges: summary←orders, summary←returns, daily←orders"
    );
    let shared = edges.iter().filter(|(from, _)| *from == summary).count();
    assert_eq!(shared, 2, "multi-source edges share the write target");
    assert!(edges.contains(&(summary.clone(), orders.clone())));
    assert!(edges.contains(&(summary.clone(), returns.clone())));
    assert!(edges.contains(&(daily.clone(), orders.clone())));

    let mut nodes = queries.select_dataset_node_ids().await.expect("nodes v1");
    nodes.sort();
    let mut expected = vec![
        summary.clone(),
        orders.clone(),
        returns.clone(),
        daily.clone(),
    ];
    expected.sort();
    assert_eq!(nodes, expected, "four edge-driven nodes");

    assert_eq!(
        queries
            .count_lineage_evidence_for("notebooks/nb.ipynb")
            .await
            .expect("evidence v1"),
        3,
        "one evidence row per (edge, cell)"
    );

    // v2: remove the second cell (daily ← orders); content changes ⇒ re-extract.
    write_notebook(
        root.path(),
        "nb.ipynb",
        r#"{"cells":[
            {"cell_type":"code","source":"%%sql\nCREATE TABLE main.sales.summary AS SELECT a.x FROM main.sales.orders JOIN main.sales.returns ON a.id = b.id"}
        ],"metadata":{"language_info":{"name":"python"}}}"#,
    );
    index_notebook_source(
        &notebook_source("notebooks"),
        root.path(),
        &queries,
        1_048_576,
        &trusted_ctx(),
    )
    .await
    .expect("index v2");

    let edges = queries.select_lineage_edges().await.expect("edges v2");
    assert_eq!(edges.len(), 2, "the removed cell's edge is dropped");
    assert!(
        !edges.contains(&(daily.clone(), orders.clone())),
        "daily←orders gone"
    );

    let nodes = queries.select_dataset_node_ids().await.expect("nodes v2");
    assert!(!nodes.contains(&daily), "orphaned daily node GC'd");
    assert!(nodes.contains(&summary) && nodes.contains(&orders) && nodes.contains(&returns));

    assert_eq!(
        queries
            .count_lineage_evidence_for("notebooks/nb.ipynb")
            .await
            .expect("evidence v2"),
        2,
        "the removed cell's evidence is dropped"
    );
}

/// U4-3 (AR-03): a zero-lineage notebook is still version-stamped and a second
/// unchanged run hash-skips it rather than re-extracting perpetually.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn zero_lineage_notebook_stamps_and_hash_skips_on_reindex() {
    let root = tempfile::TempDir::new().expect("tempdir");
    write_notebook(
        root.path(),
        "plain.ipynb",
        r##"{"cells":[
            {"cell_type":"markdown","source":"# Title"},
            {"cell_type":"code","source":"total = 1 + 2\nprint(total)"}
        ],"metadata":{"language_info":{"name":"python"}}}"##,
    );

    let db = connect_db(&root.path().join("data"), "lineage-zero")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    let first = index_notebook_source(
        &notebook_source("notebooks"),
        root.path(),
        &queries,
        1_048_576,
        &trusted_ctx(),
    )
    .await
    .expect("index first");
    assert_eq!(first.ingested, 1);
    assert!(
        queries
            .select_lineage_edges()
            .await
            .expect("edges")
            .is_empty(),
        "no spark ⇒ no lineage"
    );
    assert_eq!(
        queries
            .lineage_index_version("notebooks/plain.ipynb")
            .await
            .expect("version"),
        Some(CURRENT_EXTRACTOR_VERSION.to_owned()),
        "zero-lineage notebook is still stamped (AR-03)"
    );

    // Second run: content unchanged ⇒ hash-skip, not re-extracted.
    let second = index_notebook_source(
        &notebook_source("notebooks"),
        root.path(),
        &queries,
        1_048_576,
        &trusted_ctx(),
    )
    .await
    .expect("index second");
    assert_eq!(second.unchanged, 1, "unchanged notebook is hash-skipped");
    assert_eq!(second.ingested, 0);
    assert_eq!(
        queries
            .lineage_index_version("notebooks/plain.ipynb")
            .await
            .expect("version after reindex"),
        Some(CURRENT_EXTRACTOR_VERSION.to_owned()),
    );
}
