//! Integration tests for notebook lineage freshness (095-F, Unit U4b,
//! task 095.009-T).
//!
//! Covers the two incremental-index correctness gaps the happy-path U4 write
//! path does not: (1) the version-fingerprint backfill — an unchanged notebook
//! whose persisted extractor version is stale (or absent) must re-extract, then
//! durably skip once re-stamped; and (2) the whole-notebook deletion sweep,
//! which must GC a removed notebook's lineage edges, evidence, and freshness row
//! while sparing datasets still evidenced elsewhere.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[cfg(feature = "cozo-backend")]
use engram::db::{connect_db, queries::CodeGraphQueries};
#[cfg(feature = "cozo-backend")]
use engram::models::lineage::{
    DatasetKind, LineageAuthorityContext, LineageEndpoint, LineageEvidence, lineage_freshness_token,
};
#[cfg(feature = "cozo-backend")]
use engram::models::registry::{ContentSource, ContentSourceStatus};
#[cfg(feature = "cozo-backend")]
use engram::services::notebook_indexer::{index_notebook_source, sweep_deleted_notebook_files};

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

/// A trusted context whose `main` catalog maps to a caller-chosen authority id.
/// Used to prove an authority-config change invalidates the freshness skip.
#[cfg(feature = "cozo-backend")]
fn ctx_with_authority(authority: &str) -> LineageAuthorityContext {
    let mut catalogs = BTreeMap::new();
    catalogs.insert("main".to_owned(), authority.to_owned());
    LineageAuthorityContext::new(catalogs, vec!["s3://bucket".to_owned()])
}

#[cfg(feature = "cozo-backend")]
fn table_id(literal: &str) -> String {
    trusted_ctx()
        .resolve_table(literal)
        .expect("literal resolves under the trusted context")
        .id
}

#[cfg(feature = "cozo-backend")]
async fn index(source: &ContentSource, root: &Path, queries: &CodeGraphQueries) -> usize {
    index_notebook_source(source, root, queries, 1_048_576, &trusted_ctx())
        .await
        .expect("index notebook source")
        .ingested
}

/// A single-edge CTAS fixture: `summary` derives from `orders`.
#[cfg(feature = "cozo-backend")]
const CTAS_SUMMARY: &str = r#"{"cells":[
    {"cell_type":"code","source":"%%sql\nCREATE TABLE main.sales.summary AS SELECT x FROM main.sales.orders"}
],"metadata":{"language_info":{"name":"python"}}}"#;

/// U4b-1 (Review comment 4 / cycle-5 F5): an unchanged notebook whose persisted
/// extractor version is stale must re-extract (backfill), re-stamp the current
/// version, then durably hash+version-skip on the next same-version run — proving
/// the predicate is neither a one-shot nor a perpetual reindex — and the stamped
/// version survives a store re-open.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn stale_version_backfills_then_durably_skips_and_survives_reopen() {
    let root = tempfile::TempDir::new().expect("tempdir");
    write_notebook(root.path(), "nb.ipynb", CTAS_SUMMARY);
    let data = root.path().join("data");
    let source = notebook_source("notebooks");
    let summary = table_id("main.sales.summary");
    let orders = table_id("main.sales.orders");

    {
        let db = connect_db(&data, "lineage-fresh")
            .await
            .expect("connect_db");
        let queries = CodeGraphQueries::new(db);

        // First index: lineage present, stamped at the current version.
        assert_eq!(index(&source, root.path(), &queries).await, 1);
        assert!(
            queries
                .select_lineage_edges()
                .await
                .expect("edges")
                .contains(&(summary.clone(), orders.clone())),
            "first index emits summary←orders"
        );

        // Simulate a notebook indexed by a PRIOR extractor version.
        queries
            .upsert_lineage_index_state("notebooks/nb.ipynb", "0.0.1")
            .await
            .expect("stamp prior version");

        // Re-index: content unchanged, but the version is stale ⇒ backfill.
        assert_eq!(
            index(&source, root.path(), &queries).await,
            1,
            "a stale extractor version forces re-extraction of an unchanged notebook"
        );
        assert_eq!(
            queries
                .lineage_index_version("notebooks/nb.ipynb")
                .await
                .expect("version"),
            Some(lineage_freshness_token(&trusted_ctx())),
            "backfill re-stamps the current version"
        );
        assert!(
            queries
                .select_lineage_edges()
                .await
                .expect("edges after backfill")
                .contains(&(summary.clone(), orders.clone())),
            "lineage reappears after backfill"
        );

        // Re-index again at the current version: durable hash+version skip.
        assert_eq!(
            index(&source, root.path(), &queries).await,
            0,
            "an unchanged notebook already at the current version is skipped (not perpetual reindex)"
        );
    }

    // Re-open the on-disk store: the stamped version is durable (cycle-5 F5).
    let db2 = connect_db(&data, "lineage-fresh")
        .await
        .expect("reconnect_db");
    let queries2 = CodeGraphQueries::new(db2);
    assert_eq!(
        queries2
            .lineage_index_version("notebooks/nb.ipynb")
            .await
            .expect("version after reopen"),
        Some(lineage_freshness_token(&trusted_ctx())),
        "the durable version slot survives a store re-open"
    );
}

/// U4b-2 (cycle-7 I1 recovery): a notebook whose lineage graph rows exist but
/// whose `lineage_index_state` row is ABSENT — a pre-stamp partial-write failure
/// — is NOT hash-skipped; it re-extracts, replacing the leftover graph rows with
/// the correct lineage and re-stamping the current version.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn missing_stamp_with_present_graph_rows_forces_reextract() {
    let root = tempfile::TempDir::new().expect("tempdir");
    write_notebook(root.path(), "nb.ipynb", CTAS_SUMMARY);
    let source = notebook_source("notebooks");

    let db = connect_db(&root.path().join("data"), "lineage-recover")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    // Full index first, so content records (with a matching hash) exist.
    assert_eq!(index(&source, root.path(), &queries).await, 1);

    // Wipe the notebook's lineage (graph rows + freshness stamp); content
    // records survive — `delete_lineage_by_scope` never touches them.
    queries
        .delete_lineage_by_scope("notebooks/nb.ipynb")
        .await
        .expect("wipe lineage");

    // Re-inject STALE, un-stamped partial-write leftovers: graph rows without a
    // freshness row (the I1 failure shape).
    let stale_a = LineageEndpoint {
        id: "table::metastore-prod::main.stale.a".to_owned(),
        name: "main.stale.a".to_owned(),
        kind: DatasetKind::Table,
    };
    let stale_b = LineageEndpoint {
        id: "table::metastore-prod::main.stale.b".to_owned(),
        name: "main.stale.b".to_owned(),
        kind: DatasetKind::Table,
    };
    queries
        .upsert_dataset_nodes(&[stale_a.clone(), stale_b.clone()])
        .await
        .expect("inject stale nodes");
    queries
        .upsert_lineage_edges(&[(stale_a.id.clone(), stale_b.id.clone())])
        .await
        .expect("inject stale edge");
    queries
        .upsert_lineage_edge_evidence(&[LineageEvidence {
            from_id: stale_a.id.clone(),
            to_id: stale_b.id.clone(),
            notebook_path: "notebooks/nb.ipynb".to_owned(),
            chunk_index: 0,
            content_hash: "stale".to_owned(),
        }])
        .await
        .expect("inject stale evidence");

    // Precondition: graph rows exist, but no freshness stamp.
    assert_eq!(
        queries
            .lineage_index_version("notebooks/nb.ipynb")
            .await
            .expect("version precondition"),
        None,
        "the partial-write shape has no freshness stamp"
    );

    // Re-index: content hash is unchanged, but the absent stamp must force a
    // re-extract (I1) rather than a hash-skip.
    assert_eq!(
        index(&source, root.path(), &queries).await,
        1,
        "an absent freshness stamp forces re-extraction despite a matching hash"
    );

    let edges = queries.select_lineage_edges().await.expect("edges");
    assert!(
        !edges.contains(&(stale_a.id.clone(), stale_b.id.clone())),
        "the leftover partial-write edge is cleaned on re-extract"
    );
    assert!(
        edges.contains(&(
            table_id("main.sales.summary"),
            table_id("main.sales.orders")
        )),
        "the correct lineage is rebuilt"
    );
    assert_eq!(
        queries
            .lineage_index_version("notebooks/nb.ipynb")
            .await
            .expect("version after recovery"),
        Some(lineage_freshness_token(&trusted_ctx())),
        "recovery re-stamps the current version"
    );
}

/// U4b-3 (Review comment 5 / AR-22): deleting a whole notebook sweeps its
/// lineage evidence, GCs its now-unevidenced edges and orphaned nodes, and
/// deletes its `lineage_index_state` row — while a dataset still evidenced by
/// another notebook survives.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn deleting_notebook_sweeps_its_lineage_but_spares_shared_datasets() {
    let root = tempfile::TempDir::new().expect("tempdir");
    write_notebook(
        root.path(),
        "nb_a.ipynb",
        r#"{"cells":[
            {"cell_type":"code","source":"%%sql\nCREATE TABLE main.sales.summary AS SELECT x FROM main.sales.orders"}
        ],"metadata":{"language_info":{"name":"python"}}}"#,
    );
    write_notebook(
        root.path(),
        "nb_b.ipynb",
        r#"{"cells":[
            {"cell_type":"code","source":"%%sql\nCREATE TABLE main.sales.report AS SELECT x FROM main.sales.orders"}
        ],"metadata":{"language_info":{"name":"python"}}}"#,
    );
    let source = notebook_source("notebooks");

    let db = connect_db(&root.path().join("data"), "lineage-sweep")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index(&source, root.path(), &queries).await;

    let summary = table_id("main.sales.summary");
    let report = table_id("main.sales.report");
    let orders = table_id("main.sales.orders");

    let edges = queries.select_lineage_edges().await.expect("edges v1");
    assert!(edges.contains(&(summary.clone(), orders.clone())));
    assert!(edges.contains(&(report.clone(), orders.clone())));

    // Delete notebook A from disk, then sweep.
    fs::remove_file(root.path().join("notebooks").join("nb_a.ipynb")).expect("delete notebook A");
    let removed = sweep_deleted_notebook_files(&source, root.path(), &queries)
        .await
        .expect("sweep deleted notebooks");
    assert_eq!(removed, 1, "one notebook removed");

    assert_eq!(
        queries
            .count_lineage_evidence_for("notebooks/nb_a.ipynb")
            .await
            .expect("A evidence"),
        0,
        "notebook A's evidence is swept"
    );

    let edges = queries.select_lineage_edges().await.expect("edges v2");
    assert!(
        !edges.contains(&(summary.clone(), orders.clone())),
        "A's now-unevidenced edge is GC'd"
    );
    assert!(
        edges.contains(&(report.clone(), orders.clone())),
        "B's edge survives"
    );

    let nodes = queries.select_dataset_node_ids().await.expect("nodes");
    assert!(
        !nodes.contains(&summary),
        "A's orphaned target node is GC'd"
    );
    assert!(
        nodes.contains(&orders),
        "the shared source node survives (still evidenced by B)"
    );
    assert!(nodes.contains(&report), "B's target node survives");

    assert_eq!(
        queries
            .lineage_index_version("notebooks/nb_a.ipynb")
            .await
            .expect("A version"),
        None,
        "A's lineage_index_state row is deleted (AR-22)"
    );
    assert_eq!(
        queries
            .lineage_index_version("notebooks/nb_b.ipynb")
            .await
            .expect("B version"),
        Some(lineage_freshness_token(&trusted_ctx())),
        "B stays stamped"
    );
}

/// U4b / C4: the freshness token folds an authority-config fingerprint, so the
/// SAME notebook content re-extracts when the trusted-authority config changes
/// (otherwise a hash-unchanged notebook keeps stale or empty lineage forever),
/// then durably skips again once the config is stable.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn changed_authority_config_invalidates_hash_skip_and_backfills() {
    let root = tempfile::TempDir::new().expect("tempdir");
    write_notebook(root.path(), "nb.ipynb", CTAS_SUMMARY);
    let source = notebook_source("notebooks");
    let db = connect_db(&root.path().join("data"), "lineage-cfg")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    // Config A: catalog `main` → `metastore-A`.
    let ctx_a = ctx_with_authority("metastore-A");
    let first = index_notebook_source(&source, root.path(), &queries, 1_048_576, &ctx_a)
        .await
        .expect("index A");
    assert_eq!(first.ingested, 1, "first index extracts the notebook");

    // Re-index the SAME content under a CHANGED config (`main` → `metastore-B`).
    let ctx_b = ctx_with_authority("metastore-B");
    let changed = index_notebook_source(&source, root.path(), &queries, 1_048_576, &ctx_b)
        .await
        .expect("index B");
    assert_eq!(
        changed.ingested, 1,
        "a changed authority config forces re-extraction of unchanged content (C4)"
    );
    assert_eq!(
        changed.unchanged, 0,
        "the notebook must NOT be hash-skipped after a config change"
    );

    // The refreshed lineage now binds to config B's authority-embedded ids.
    let summary_b = ctx_b
        .resolve_table("main.sales.summary")
        .expect("resolve summary under B")
        .id;
    let orders_b = ctx_b
        .resolve_table("main.sales.orders")
        .expect("resolve orders under B")
        .id;
    assert!(
        queries
            .select_lineage_edges()
            .await
            .expect("edges B")
            .contains(&(summary_b, orders_b)),
        "re-extraction rebinds lineage to the new authority"
    );

    // Re-index AGAIN under the same config B: content AND config unchanged, so a
    // durable hash+fingerprint skip (never a perpetual reindex).
    let stable = index_notebook_source(&source, root.path(), &queries, 1_048_576, &ctx_b)
        .await
        .expect("index B again");
    assert_eq!(
        stable.unchanged, 1,
        "unchanged content and config durably skip (no perpetual reindex)"
    );
    assert_eq!(stable.ingested, 0);
}
