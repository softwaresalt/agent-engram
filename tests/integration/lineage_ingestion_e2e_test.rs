//! Integration test for the registry → ingestion → lineage end-to-end path
//! (097-F V4, task 097.005-T).
//!
//! Proves that a `RegistryConfig.lineage` section is threaded through
//! `ingest_all_sources` into the notebook indexer's trusted-authority context,
//! so a notebook CTAS emits an authority-bound lineage edge. The existing
//! freshness / write-path integration tests call `index_notebook_source` with a
//! hand-built context directly; this test closes the seam a prior PR #284 review
//! flagged as untested by exercising the full `parse_registry_yaml →
//! ingest_all_sources` wiring (`config.lineage.to_authority_context()`).

use std::fs;

use tempfile::TempDir;

#[cfg(feature = "cozo-backend")]
use engram::db::connect_db;
#[cfg(feature = "cozo-backend")]
use engram::db::queries::CodeGraphQueries;
#[cfg(feature = "cozo-backend")]
use engram::services::ingestion::ingest_all_sources;
#[cfg(feature = "cozo-backend")]
use engram::services::registry::parse_registry_yaml;

/// A single-edge CTAS notebook: `main.sales.summary` derives from
/// `main.sales.orders`, both under the trusted `main` catalog.
#[cfg(feature = "cozo-backend")]
const CTAS_NOTEBOOK: &str = r#"{"cells":[
    {"cell_type":"code","source":"%%sql\nCREATE TABLE main.sales.summary AS SELECT x FROM main.sales.orders"}
],"metadata":{"language_info":{"name":"python"}}}"#;

/// A registry declaring a notebook source AND a trusted-authority `lineage`
/// section, so the ingestion path can bind authority-embedded lineage endpoints.
#[cfg(feature = "cozo-backend")]
const REGISTRY_YAML: &str = concat!(
    "sources:\n",
    "  - type: notebook\n",
    "    path: notebooks\n",
    "lineage:\n",
    "  metastore_authority_id: metastore-prod\n",
    "  catalog_authorities:\n",
    "    main: metastore-prod\n",
    "  storage_authorities:\n",
    "    - s3://bucket\n",
);

/// V4 (097.005-T): a `RegistryConfig.lineage` section drives
/// `ingest_all_sources` to bind an authority-embedded lineage edge for a
/// notebook CTAS.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn registry_lineage_config_binds_authority_edge_through_ingest_all_sources() {
    let root = TempDir::new().expect("tempdir");
    let notebooks = root.path().join("notebooks");
    fs::create_dir_all(&notebooks).expect("create notebooks dir");
    fs::write(notebooks.join("nb.ipynb"), CTAS_NOTEBOOK).expect("write notebook fixture");

    let config = parse_registry_yaml(REGISTRY_YAML).expect("parse registry");

    // The expected edge ids are derived from the SAME config the ingestion path
    // consumes, so the assertion is meaningful only if the registry lineage
    // config actually threads through `ingest_all_sources`.
    let authority_ctx = config.lineage.to_authority_context();
    let summary = authority_ctx
        .resolve_table("main.sales.summary")
        .expect("summary resolves under the registry lineage config")
        .id;
    let orders = authority_ctx
        .resolve_table("main.sales.orders")
        .expect("orders resolves under the registry lineage config")
        .id;
    assert!(
        summary.starts_with("table::metastore-prod::"),
        "the resolved id is authority-embedded"
    );

    let db = connect_db(&root.path().join("data"), "lineage-e2e")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    let report = ingest_all_sources(&config, root.path(), &queries)
        .await
        .expect("ingest_all_sources");
    assert!(report.ingested >= 1, "the notebook source is ingested");

    let edges = queries.select_lineage_edges().await.expect("lineage edges");
    assert!(
        edges.contains(&(summary, orders)),
        "registry lineage config binds an authority-embedded summary<-orders edge"
    );
}
