//! Integration tests for model-scope invalidation on incremental sync (P3b,
//! `085.008-T`).
//!
//! P3 (`085.003-T`) aggregates a model's `.tmdl` files and emits
//! `pbi_uses_field` reference edges during a full index. The incremental
//! indexer skips unchanged files by content hash, so a sibling's reference
//! edges could go stale when a peer file changes. These tests prove that when
//! any file in a `canonical_tmdl_model_path` scope changes or is deleted, every
//! sibling's reference edges are re-resolved against the updated model-scope
//! schema.
//!
//! Tests: S-DAXINC-01 (column add), S-DAXINC-02 (column rename),
//! S-DAXINC-03 (file delete).

#![cfg(feature = "cozo-backend")]

use chrono::Utc;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

use engram::db::{connect_db, queries::CodeGraphQueries};
use engram::models::TraversalDirection;
use engram::models::content::ContentRecord;
use engram::models::powerbi_graph::PowerBiNodeKind;
use engram::models::registry::{ContentSource, ContentSourceStatus};
use engram::services::powerbi_indexer::{
    TMDL_DAX_INDEX_VERSION, compute_file_hash, compute_tmdl_dax_index_hash_for_version,
    index_powerbi_source,
};

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

/// Write the `Sales.tmdl` / `Date.tmdl` fixtures into a model definition folder.
fn write_model(tables: &Path, sales: &str, date: Option<&str>) {
    fs::write(tables.join("Sales.tmdl"), sales).expect("write Sales.tmdl");
    if let Some(body) = date {
        fs::write(tables.join("Date.tmdl"), body).expect("write Date.tmdl");
    } else {
        let path = tables.join("Date.tmdl");
        if path.exists() {
            fs::remove_file(path).expect("remove Date.tmdl");
        }
    }
}

/// The tables directory of the standard fixture model.
fn model_tables(workspace: &Path) -> std::path::PathBuf {
    workspace
        .join("models")
        .join("Sales.SemanticModel")
        .join("definition")
        .join("tables")
}

async fn reindex(workspace: &Path, queries: &CodeGraphQueries) {
    index_powerbi_source(&powerbi_source("models"), workspace, queries, MAX_FILE_SIZE)
        .await
        .expect("index tmdl source");
}

async fn seed_legacy_tmdl_record(queries: &CodeGraphQueries, rel_path: &str, content: &str) {
    let record = ContentRecord {
        id: format!("legacy_{}", rel_path.replace(['/', '\\', '.'], "_")),
        content_type: "powerbi".to_string(),
        file_path: rel_path.to_string(),
        content_hash: compute_file_hash(content.as_bytes()),
        content: "legacy pre-DAX index record".to_string(),
        embedding: None,
        source_path: "models".to_string(),
        file_size_bytes: content.len() as u64,
        ingested_at: Utc::now(),
        record_kind: "legacy".to_string(),
        chunk_id: Some("legacy".to_string()),
        chunk_index: None,
        heading_path: Vec::new(),
        line_start: None,
        line_end: None,
        fallback_reason: None,
        lint_summary: None,
        suggestions: Vec::new(),
    };
    queries
        .upsert_content_record(&record)
        .await
        .expect("seed legacy tmdl content record");
}

/// Return the outgoing `pbi_uses_field` neighbour names of the measure `name`.
async fn measure_uses_field_targets(queries: &CodeGraphQueries, name: &str) -> Vec<String> {
    let nodes = queries
        .select_powerbi_nodes(Some("models"))
        .await
        .expect("select powerbi nodes");
    let Some(measure) = nodes
        .iter()
        .find(|node| node.name == name && node.kind == PowerBiNodeKind::Measure)
    else {
        panic!("expected measure {name:?} to be indexed");
    };
    let result = queries
        .query_graph_neighborhood(
            &measure.id,
            TraversalDirection::Outgoing,
            1,
            50,
            &["pbi_uses_field"],
        )
        .await
        .expect("query_graph_neighborhood");
    result
        .nodes
        .into_iter()
        .filter(|node| node.id != measure.id)
        .map(|node| node.name)
        .collect()
}

async fn maybe_measure_uses_field_targets(
    queries: &CodeGraphQueries,
    name: &str,
) -> Option<Vec<String>> {
    let nodes = queries
        .select_powerbi_nodes(Some("models"))
        .await
        .expect("select powerbi nodes");
    let measure = nodes
        .iter()
        .find(|node| node.name == name && node.kind == PowerBiNodeKind::Measure)?;
    Some(
        queries
            .query_graph_neighborhood(
                &measure.id,
                TraversalDirection::Outgoing,
                1,
                50,
                &["pbi_uses_field"],
            )
            .await
            .expect("query_graph_neighborhood")
            .nodes
            .into_iter()
            .filter(|node| node.id != measure.id)
            .map(|node| node.name)
            .collect(),
    )
}

const SALES_BY_QUARTER: &str = "table Sales\n\
     \x20\x20column Amount\n\
     \x20\x20\x20\x20dataType: double\n\
     \x20\x20measure Total = SUM(Sales[Amount])\n\
     \x20\x20measure SalesByQuarter = CALCULATE([Total], 'Date'[Quarter])\n";

const SALES_BY_YEAR: &str = "table Sales\n\
     \x20\x20column Amount\n\
     \x20\x20\x20\x20dataType: double\n\
     \x20\x20measure Total = SUM(Sales[Amount])\n\
     \x20\x20measure SalesByYear = CALCULATE([Total], 'Date'[Year])\n";

const DATE_YEAR_ONLY: &str = "table Date\n\
     \x20\x20column Year\n\
     \x20\x20\x20\x20dataType: int64\n";

/// S-DAXINC-06: `.tmdl` DAX index hashes are deterministic for a fixed version
/// and differ from both content-only hashes and other format versions.
#[test]
fn tmdl_dax_index_hash_includes_version_deterministically() {
    let content = SALES_BY_YEAR.as_bytes();
    let current = compute_tmdl_dax_index_hash_for_version(content, TMDL_DAX_INDEX_VERSION);
    let repeated = compute_tmdl_dax_index_hash_for_version(content, TMDL_DAX_INDEX_VERSION);
    let next = compute_tmdl_dax_index_hash_for_version(content, TMDL_DAX_INDEX_VERSION + 1);

    assert_eq!(current, repeated, "same version and content must be stable");
    assert_ne!(
        current,
        compute_file_hash(content),
        "versioned .tmdl hash must not collapse to the legacy content-only hash"
    );
    assert_ne!(
        current, next,
        "bumping TMDL_DAX_INDEX_VERSION must force an unchanged file to look dirty"
    );
}

/// S-DAXINC-01: adding a column to `Date.tmdl` resolves a previously-unresolved
/// reference in the UNCHANGED `Sales.tmdl` sibling (model-scope invalidation,
/// not a file-local skip).
#[tokio::test]
async fn column_add_in_sibling_resolves_unchanged_reference() {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    let tables = model_tables(&workspace);
    fs::create_dir_all(&tables).expect("create dirs");

    // Initial: Date has only Year, so `'Date'[Quarter]` is unresolved.
    write_model(&tables, SALES_BY_QUARTER, Some(DATE_YEAR_ONLY));
    let db = connect_db(&root.path().join("data"), "dax-inc-add")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    reindex(&workspace, &queries).await;

    let before = measure_uses_field_targets(&queries, "SalesByQuarter").await;
    assert!(
        !before.contains(&"Quarter".to_string()),
        "Quarter should be unresolved before it exists; got {before:?}"
    );

    // Add the Quarter column to Date.tmdl; Sales.tmdl is untouched.
    let date_with_quarter = "table Date\n\
         \x20\x20column Year\n\
         \x20\x20\x20\x20dataType: int64\n\
         \x20\x20column Quarter\n\
         \x20\x20\x20\x20dataType: int64\n";
    fs::write(tables.join("Date.tmdl"), date_with_quarter).expect("update Date.tmdl");
    reindex(&workspace, &queries).await;

    let after = measure_uses_field_targets(&queries, "SalesByQuarter").await;
    assert!(
        after.contains(&"Quarter".to_string()),
        "unchanged Sales sibling must re-resolve to the newly-added Date[Quarter]; got {after:?}"
    );
}

/// S-DAXINC-02: renaming a column in `Date.tmdl` drops the now-unresolved
/// reference edge from the UNCHANGED `Sales.tmdl` sibling (no stale edge).
#[tokio::test]
async fn column_rename_in_sibling_drops_stale_reference() {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    let tables = model_tables(&workspace);
    fs::create_dir_all(&tables).expect("create dirs");

    write_model(&tables, SALES_BY_YEAR, Some(DATE_YEAR_ONLY));
    let db = connect_db(&root.path().join("data"), "dax-inc-rename")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    reindex(&workspace, &queries).await;

    let before = measure_uses_field_targets(&queries, "SalesByYear").await;
    assert!(
        before.contains(&"Year".to_string()),
        "SalesByYear should resolve to Date[Year] initially; got {before:?}"
    );

    // Rename Year -> FiscalYear in Date.tmdl; Sales.tmdl still references Year.
    let date_renamed = "table Date\n\
         \x20\x20column FiscalYear\n\
         \x20\x20\x20\x20dataType: int64\n";
    fs::write(tables.join("Date.tmdl"), date_renamed).expect("update Date.tmdl");
    reindex(&workspace, &queries).await;

    let after = measure_uses_field_targets(&queries, "SalesByYear").await;
    assert!(
        !after.contains(&"Year".to_string()),
        "stale edge to the renamed-away Date[Year] must be dropped; got {after:?}"
    );
}

/// S-DAXINC-03: deleting `Date.tmdl` drops the now-unresolved reference edge
/// from the UNCHANGED `Sales.tmdl` sibling (no orphaned edge).
#[tokio::test]
async fn file_delete_in_scope_drops_stale_reference() {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    let tables = model_tables(&workspace);
    fs::create_dir_all(&tables).expect("create dirs");

    write_model(&tables, SALES_BY_YEAR, Some(DATE_YEAR_ONLY));
    let db = connect_db(&root.path().join("data"), "dax-inc-delete")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    reindex(&workspace, &queries).await;

    let before = measure_uses_field_targets(&queries, "SalesByYear").await;
    assert!(
        before.contains(&"Year".to_string()),
        "SalesByYear should resolve to Date[Year] initially; got {before:?}"
    );

    // Delete Date.tmdl entirely; Sales.tmdl is untouched.
    fs::remove_file(tables.join("Date.tmdl")).expect("remove Date.tmdl");
    reindex(&workspace, &queries).await;

    let after = measure_uses_field_targets(&queries, "SalesByYear").await;
    assert!(
        !after.contains(&"Year".to_string()),
        "stale edge to the deleted Date[Year] must be dropped; got {after:?}"
    );
}

/// S-DAXINC-04: a case-mismatched DAX reference (`sales[amount]` / `'date'[year]`)
/// still emits a `pbi_uses_field` edge, and the edge targets the CANONICAL,
/// declared-case node (`Amount`, `Year`) — proving the case-insensitive resolver
/// recovers the declared casing so edge node ids match the indexed nodes.
#[tokio::test]
async fn case_mismatched_reference_resolves_to_canonical_node() {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    let tables = model_tables(&workspace);
    fs::create_dir_all(&tables).expect("create dirs");

    // References use lowercase table/column casing that differs from the declared
    // `Sales`/`Amount` and `Date`/`Year`. DAX folds case, so both must resolve.
    let sales = "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure Total = SUM(sales[amount])\n\
         \x20\x20measure ByYear = CALCULATE([total], 'date'[year])\n";
    write_model(&tables, sales, Some(DATE_YEAR_ONLY));

    let db = connect_db(&root.path().join("data"), "dax-inc-case")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    reindex(&workspace, &queries).await;

    let total_targets = measure_uses_field_targets(&queries, "Total").await;
    assert!(
        total_targets.contains(&"Amount".to_string()),
        "lowercase sales[amount] must resolve to the canonical Sales[Amount] node; got {total_targets:?}"
    );

    let by_year_targets = measure_uses_field_targets(&queries, "ByYear").await;
    assert!(
        by_year_targets.contains(&"Year".to_string()),
        "lowercase 'date'[year] must resolve to the canonical Date[Year] node; got {by_year_targets:?}"
    );
    assert!(
        by_year_targets.contains(&"Total".to_string()),
        "lowercase [total] must resolve to the canonical Total measure node; got {by_year_targets:?}"
    );
}

/// S-DAXINC-05: a workspace seeded with legacy content-only `.tmdl` hashes
/// represents an index created before DAX reference edges existed. A DAX index
/// format-version fingerprint bump must force a one-time reprocess without
/// editing files or passing `--force`, so the missing `pbi_uses_field` edges are
/// materialized.
#[tokio::test]
async fn legacy_tmdl_hash_forces_one_time_dax_reindex_without_file_edit() {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    let tables = model_tables(&workspace);
    fs::create_dir_all(&tables).expect("create dirs");

    write_model(&tables, SALES_BY_YEAR, Some(DATE_YEAR_ONLY));
    let db = connect_db(&root.path().join("data"), "dax-inc-version")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    seed_legacy_tmdl_record(
        &queries,
        "models/Sales.SemanticModel/definition/tables/Sales.tmdl",
        SALES_BY_YEAR,
    )
    .await;
    seed_legacy_tmdl_record(
        &queries,
        "models/Sales.SemanticModel/definition/tables/Date.tmdl",
        DATE_YEAR_ONLY,
    )
    .await;

    assert!(
        maybe_measure_uses_field_targets(&queries, "SalesByYear")
            .await
            .is_none(),
        "legacy fixture should start without DAX graph nodes"
    );

    let result = index_powerbi_source(
        &powerbi_source("models"),
        &workspace,
        &queries,
        MAX_FILE_SIZE,
    )
    .await
    .expect("index tmdl source");
    assert_eq!(
        result.ingested, 2,
        "versioned hash mismatch should reprocess unchanged legacy .tmdl files"
    );

    let after = measure_uses_field_targets(&queries, "SalesByYear").await;
    assert!(
        after.contains(&"Year".to_string()),
        "unchanged legacy-indexed files must materialize Date[Year] reference edges; got {after:?}"
    );

    let second = index_powerbi_source(
        &powerbi_source("models"),
        &workspace,
        &queries,
        MAX_FILE_SIZE,
    )
    .await
    .expect("index tmdl source again");
    assert_eq!(
        (second.ingested, second.unchanged),
        (0, 2),
        "same-version re-index should hash-skip unchanged .tmdl files after migration"
    );
}
