//! Unit tests for Power BI TMDL entity extraction (061.006-T).
//!
//! Validates structural extraction of TMDL semantic model folders into the
//! shared Power BI entity model used by the JSON-backed PBIP extractor.

use engram::services::powerbi_tmdl::extract_tmdl_semantic_model;

/// Representative TMDL snippet with one table, one measure, one relationship,
/// and one data source.
fn tmdl_fixture() -> &'static str {
    r"
model Sales Model

table Sales
  column Date
    dataType: dateTime
  column Amount
    dataType: double
  measure 'Total Sales' = SUM ( Sales[Amount] )

relationship Sales.ProductID -> Products.ID

dataSource SqlWarehouse
"
}

/// S-PTM-01: TMDL extraction returns the expected table, column, and measure
/// structure.
#[test]
fn extract_tmdl_semantic_model_tables_columns_and_measures() {
    let model =
        extract_tmdl_semantic_model(tmdl_fixture(), "models/Sales.SemanticModel/definition")
            .expect("fixture should produce a semantic model");

    assert_eq!(model.name, "Sales Model");
    assert_eq!(model.tables.len(), 1);
    assert_eq!(model.tables[0].name, "Sales");
    assert_eq!(model.tables[0].columns.len(), 2);
    assert_eq!(model.tables[0].measures.len(), 1);
    assert_eq!(model.tables[0].measures[0].name, "Total Sales");
}

/// S-PTM-02: TMDL extraction preserves relationship endpoints.
#[test]
fn extract_tmdl_semantic_model_relationships() {
    let model =
        extract_tmdl_semantic_model(tmdl_fixture(), "models/Sales.SemanticModel/definition")
            .expect("fixture should produce a semantic model");

    assert_eq!(model.relationships.len(), 1);
    let rel = &model.relationships[0];
    assert_eq!(rel.from_table, "Sales");
    assert_eq!(rel.from_column, "ProductID");
    assert_eq!(rel.to_table, "Products");
    assert_eq!(rel.to_column, "ID");
}

/// S-PTM-03: TMDL extraction emits the same canonical entity kinds used by the
/// JSON-backed semantic model extractor.
#[test]
fn extract_tmdl_semantic_model_uses_canonical_entity_kinds() {
    let model =
        extract_tmdl_semantic_model(tmdl_fixture(), "models/Sales.SemanticModel/definition")
            .expect("fixture should produce a semantic model");

    assert!(
        !model.tables[0].id.is_empty(),
        "table IDs should be generated through the shared semantic model schema"
    );
    assert!(
        !model.tables[0].columns[0].id.is_empty(),
        "column IDs should be generated through the shared semantic model schema"
    );
    assert!(
        !model.tables[0].measures[0].id.is_empty(),
        "measure IDs should be generated through the shared semantic model schema"
    );
    assert!(
        !model.relationships[0].id.is_empty(),
        "relationship IDs should be generated through the shared semantic model schema"
    );
}

/// S-PTM-04: TMDL extraction returns `None` for text that does not describe a
/// semantic model.
#[test]
fn extract_tmdl_semantic_model_returns_none_for_non_model_text() {
    let model = extract_tmdl_semantic_model("note: this is not tmdl", "models/notes.txt");
    assert!(model.is_none());
}

/// S-PTM-05: Entity files under `definition/*.tmdl` infer the semantic model
/// name from the parent `.SemanticModel` directory.
#[test]
fn extract_tmdl_semantic_model_infers_model_name_from_entity_file_path() {
    let model = extract_tmdl_semantic_model(
        "
table Sales
  column Amount
    dataType: double
",
        "models/Sales.SemanticModel/definition/Tables/Sales.tmdl",
    )
    .expect("fixture should produce a semantic model");

    assert_eq!(model.name, "Sales");
}

/// S-PTM-06: Relationship blocks that use `fromColumn:` and `toColumn:` should
/// produce relationship entities.
#[test]
fn extract_tmdl_semantic_model_parses_relationship_blocks() {
    let model = extract_tmdl_semantic_model(
        "
relationship FactToTitle
  fromColumn: FactVehicleRegistrations.VehicleTitleKey
  toColumn: DimVehicleTitle.VehicleTitleKey
",
        "models/Sales.SemanticModel/definition/relationships.tmdl",
    )
    .expect("fixture should produce a semantic model");

    assert_eq!(model.relationships.len(), 1);
    let rel = &model.relationships[0];
    assert_eq!(rel.from_table, "FactVehicleRegistrations");
    assert_eq!(rel.from_column, "VehicleTitleKey");
    assert_eq!(rel.to_table, "DimVehicleTitle");
    assert_eq!(rel.to_column, "VehicleTitleKey");
}

/// S-PTM-07: Multiline measure bodies should be preserved as the measure
/// expression text.
#[test]
fn extract_tmdl_semantic_model_preserves_multiline_measure_expression() {
    let model = extract_tmdl_semantic_model(
        "
table Sales
  measure 'Registrations With Lien Holder' =
    CALCULATE (
      [Total Registrations],
      FILTER ( Sales, Sales[HasLien] = TRUE () )
    )
",
        "models/Sales.SemanticModel/definition/tables/Sales.tmdl",
    )
    .expect("fixture should produce a semantic model");

    assert_eq!(model.tables.len(), 1);
    assert_eq!(model.tables[0].measures.len(), 1);
    assert_eq!(
        model.tables[0].measures[0].expression.as_deref(),
        Some("CALCULATE (\n[Total Registrations],\nFILTER ( Sales, Sales[HasLien] = TRUE () )\n)")
    );
}

/// S-PTM-08: `model.tmdl` files that only carry refs should still produce a
/// semantic-model shell so the canonical model file is indexable.
#[test]
fn extract_tmdl_semantic_model_keeps_ref_only_model_file() {
    let model = extract_tmdl_semantic_model(
        "
model Sales Dataset

ref table Sales
ref relationship SalesToProducts
",
        "models/Sales.SemanticModel/definition/model.tmdl",
    )
    .expect("model.tmdl should still produce a semantic model shell");

    assert_eq!(model.name, "Sales Dataset");
    assert!(model.tables.is_empty());
    assert!(model.relationships.is_empty());
    assert!(model.data_sources.is_empty());
}

/// S-PTM-09: Top-level `expressions.tmdl` declarations should be preserved as
/// semantic-model expressions.
#[test]
fn extract_tmdl_semantic_model_parses_top_level_expressions() {
    let model = extract_tmdl_semantic_model(
        r#"
expression SynapseSqlServer = "dp-da-synw-t-cus-01-ondemand.sql.azuresynapse.net" meta [IsParameterQuery=true, Type="Text"]

expression SynapseDatabase = "ILSOS_EDW" meta [IsParameterQuery=true, Type="Text"]
"#,
        "models/Sales.SemanticModel/definition/expressions.tmdl",
    )
    .expect("expressions.tmdl should produce a semantic model");

    assert_eq!(model.expressions.len(), 2);
    assert_eq!(model.expressions[0].name, "SynapseSqlServer");
    assert_eq!(
        model.expressions[0].expression.as_deref(),
        Some("\"dp-da-synw-t-cus-01-ondemand.sql.azuresynapse.net\"")
    );
    assert_eq!(model.expressions[1].name, "SynapseDatabase");
    assert_eq!(
        model.expressions[1].expression.as_deref(),
        Some("\"ILSOS_EDW\"")
    );
}
