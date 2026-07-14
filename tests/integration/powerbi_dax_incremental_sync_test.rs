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

use std::fs;
use std::path::Path;
use tempfile::TempDir;

use engram::db::{connect_db, queries::CodeGraphQueries};
use engram::models::TraversalDirection;
use engram::models::powerbi_graph::PowerBiNodeKind;
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
