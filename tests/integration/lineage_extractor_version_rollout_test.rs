//! Integration test for the 097-F X1 extractor-version rollout (task 097.006-T).
//!
//! The U4b freshness token = `{CURRENT_EXTRACTOR_VERSION}:{config_fingerprint}`
//! gates re-extraction of unchanged notebooks. V2/V5/W2/W1 changed extractor
//! output, so the version bump (1.0.0 -> 1.1.0) must force a re-extraction of a
//! notebook still stamped at the prior version — otherwise it retains the older,
//! less-precise (potentially false) lineage until its content changes (C4).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[cfg(feature = "cozo-backend")]
use engram::db::{connect_db, queries::CodeGraphQueries};
#[cfg(feature = "cozo-backend")]
use engram::models::lineage::{LineageAuthorityContext, lineage_freshness_token};
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
fn trusted_ctx() -> LineageAuthorityContext {
    let mut catalogs = BTreeMap::new();
    catalogs.insert("main".to_owned(), "metastore-prod".to_owned());
    LineageAuthorityContext::new(catalogs, vec!["s3://bucket".to_owned()])
}

#[cfg(feature = "cozo-backend")]
fn write_notebook(workspace: &Path, name: &str, json: &str) {
    let dir = workspace.join("notebooks");
    fs::create_dir_all(&dir).expect("create notebooks dir");
    fs::write(dir.join(name), json).expect("write notebook fixture");
}

/// A single-edge CTAS fixture: `summary` derives from `orders`.
#[cfg(feature = "cozo-backend")]
const CTAS_SUMMARY: &str = r#"{"cells":[
    {"cell_type":"code","source":"%%sql\nCREATE TABLE main.sales.summary AS SELECT x FROM main.sales.orders"}
],"metadata":{"language_info":{"name":"python"}}}"#;

/// X1 (097.006-T): a notebook stamped at the PRIOR extractor version
/// (`1.0.0:{fingerprint}`) must re-extract after the version bump, even though
/// its content and authority config are unchanged. Before the bump the stamped
/// token equals the current token and the notebook is hash-skipped
/// (`ingested == 0`); after the bump the tokens differ and it re-extracts
/// (`ingested == 1`).
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn prior_version_stamp_forces_reextraction_after_bump() {
    let root = tempfile::TempDir::new().expect("tempdir");
    write_notebook(root.path(), "nb.ipynb", CTAS_SUMMARY);
    let source = notebook_source("notebooks");
    let ctx = trusted_ctx();
    let current_token = lineage_freshness_token(&ctx);
    let expected_edges = vec![(
        ctx.resolve_table("main.sales.summary")
            .expect("summary resolves under the trusted context")
            .id,
        ctx.resolve_table("main.sales.orders")
            .expect("orders resolves under the trusted context")
            .id,
    )];

    let db = connect_db(&root.path().join("data"), "lineage-rollout")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    // First index establishes lineage, stamped at the CURRENT version.
    let first = index_notebook_source(&source, root.path(), &queries, 1_048_576, &ctx)
        .await
        .expect("first index");
    assert_eq!(first.ingested, 1, "first index extracts the notebook");
    assert_eq!(
        queries
            .select_lineage_edges()
            .await
            .expect("edges after first index"),
        expected_edges,
        "first current-version index persists the expected lineage"
    );
    assert_eq!(
        queries
            .lineage_index_version("notebooks/nb.ipynb")
            .await
            .expect("version after first index"),
        Some(current_token.clone()),
        "first current-version index stamps the current extractor version"
    );

    // Unchanged content already stamped at the CURRENT extractor version must
    // hash+version skip without mutating the persisted lineage.
    let unchanged = index_notebook_source(&source, root.path(), &queries, 1_048_576, &ctx)
        .await
        .expect("reindex unchanged content at current version");
    assert_eq!(
        unchanged.ingested, 0,
        "an unchanged notebook already stamped at the current extractor version is skipped"
    );
    assert_eq!(
        queries
            .select_lineage_edges()
            .await
            .expect("edges after current-version control"),
        expected_edges,
        "current-version control run leaves persisted lineage exactly unchanged"
    );
    assert_eq!(
        queries
            .lineage_index_version("notebooks/nb.ipynb")
            .await
            .expect("version after current-version control"),
        Some(current_token.clone()),
        "current-version control run leaves the current freshness stamp unchanged"
    );

    // Roll the stamp back to the PRIOR extractor version (1.0.0), preserving the
    // authority-config fingerprint so ONLY the version differs from current.
    let prior_token = format!("1.0.0:{}", ctx.config_fingerprint());
    queries
        .upsert_lineage_index_state("notebooks/nb.ipynb", &prior_token)
        .await
        .expect("stamp prior extractor version");

    // Re-index unchanged content: the prior version must NOT hash-skip; the
    // version bump forces a re-extraction (C4). Before the 1.0.0 -> 1.1.0 bump
    // the stamped token equals current and this is 0 (RED).
    let rolled = index_notebook_source(&source, root.path(), &queries, 1_048_576, &ctx)
        .await
        .expect("reindex after version rollback");
    assert_eq!(
        rolled.ingested, 1,
        "a notebook stamped at the prior extractor version re-extracts after the bump"
    );
    assert_eq!(
        queries
            .select_lineage_edges()
            .await
            .expect("edges after version rollback"),
        expected_edges,
        "rollback-triggered re-extraction preserves the expected lineage"
    );
    assert_eq!(
        queries
            .lineage_index_version("notebooks/nb.ipynb")
            .await
            .expect("version after version rollback"),
        Some(current_token),
        "rollback-triggered re-extraction re-stamps the current extractor version"
    );
}
