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

/// S-PTM-10: Partition blocks with a fenced M source body surface a partition
/// entity on the table, preserving name, source kind, mode, and opaque M body.
#[test]
fn extract_tmdl_semantic_model_parses_partition_with_fenced_m_body() {
    let model = extract_tmdl_semantic_model(
        "
table FactVehicleRegistrations
  column Amount
    dataType: double
  partition FactVehicleRegistrations = m
    mode: import
    source = ```
        let
            Source = Sql.Database(\"server\", \"db\")
        in
            Source
        ```
",
        "models/Sales.SemanticModel/definition/tables/FactVehicleRegistrations.tmdl",
    )
    .expect("fixture should produce a semantic model");

    assert_eq!(model.tables.len(), 1);
    let table = &model.tables[0];
    assert_eq!(
        table.partitions.len(),
        1,
        "the table should surface exactly one partition entity"
    );
    let partition = &table.partitions[0];
    assert_eq!(partition.name, "FactVehicleRegistrations");
    assert_eq!(partition.source_kind.as_deref(), Some("m"));
    assert_eq!(partition.mode.as_deref(), Some("import"));
    assert!(
        !partition.id.is_empty(),
        "partition IDs should be generated through the shared semantic model schema"
    );
    let body = partition
        .source_expression
        .as_deref()
        .expect("partition should capture the embedded M body");
    assert!(body.contains("Sql.Database"), "M body should be preserved");
    assert!(
        !body.contains("```"),
        "captured body must not include the fence delimiters"
    );
}

/// S-PTM-11: Data source blocks expose richer connection properties
/// (`kind`/`provider`/`connectionString`/`server`/`database`) on
/// `PowerBiDataSource`.
#[test]
fn extract_tmdl_semantic_model_parses_data_source_properties() {
    let model = extract_tmdl_semantic_model(
        "
dataSource SqlWarehouse
  kind: sql
  provider: System.Data.SqlClient
  connectionString: Data Source=myserver;Initial Catalog=EDW
  server: myserver
  database: EDW
",
        "models/Sales.SemanticModel/definition/dataSources.tmdl",
    )
    .expect("fixture should produce a semantic model");

    assert_eq!(model.data_sources.len(), 1);
    let ds = &model.data_sources[0];
    assert_eq!(ds.name, "SqlWarehouse");
    assert_eq!(ds.kind.as_deref(), Some("sql"));
    assert_eq!(ds.provider.as_deref(), Some("System.Data.SqlClient"));
    assert_eq!(
        ds.connection_string.as_deref(),
        Some("Data Source=myserver;Initial Catalog=EDW")
    );
    assert_eq!(ds.server.as_deref(), Some("myserver"));
    assert_eq!(ds.database.as_deref(), Some("EDW"));
    assert!(
        !ds.id.is_empty(),
        "data source IDs should be generated through the shared semantic model schema"
    );
}

/// S-PTM-12: `ref` statements, `annotation` blocks (model/table/column/measure
/// scope), and `lineageTag`/`culture`/`defaultMode` metadata are surfaced on the
/// extracted semantic model rather than silently dropped.
#[test]
fn extract_tmdl_semantic_model_parses_refs_annotations_and_lineage() {
    let model = extract_tmdl_semantic_model(
        "
model Sales Model
  culture: en-US
  defaultMode: import
  lineageTag: model-guid-1
  annotation PBI_QueryOrder = [\"Sales\"]

  ref table Sales
  ref cultureInfo en-US

table Sales
  lineageTag: table-guid-1
  annotation IsHidden = false

  column Amount
    dataType: double
    lineageTag: column-guid-1
    annotation Format = \"#,0\"

  measure 'Total' = SUM(Sales[Amount])
    lineageTag: measure-guid-1
    annotation DisplayFolder = KPIs
",
        "models/Sales.SemanticModel/definition/model.tmdl",
    )
    .expect("fixture should produce a semantic model");

    // Model-level metadata.
    assert_eq!(model.culture.as_deref(), Some("en-US"));
    assert_eq!(model.default_mode.as_deref(), Some("import"));
    assert_eq!(model.lineage_tag.as_deref(), Some("model-guid-1"));
    assert_eq!(model.annotations.len(), 1);
    assert_eq!(model.annotations[0].name, "PBI_QueryOrder");

    // Model-level refs.
    assert_eq!(model.refs.len(), 2);
    assert_eq!(model.refs[0].kind, "table");
    assert_eq!(model.refs[0].name, "Sales");
    assert_eq!(model.refs[1].kind, "cultureInfo");
    assert_eq!(model.refs[1].name, "en-US");

    // Table / column / measure scoped metadata.
    let table = &model.tables[0];
    assert_eq!(table.lineage_tag.as_deref(), Some("table-guid-1"));
    assert_eq!(table.annotations.len(), 1);
    assert_eq!(table.annotations[0].name, "IsHidden");

    let column = &table.columns[0];
    assert_eq!(column.lineage_tag.as_deref(), Some("column-guid-1"));
    assert_eq!(column.annotations.len(), 1);
    assert_eq!(column.annotations[0].name, "Format");

    let measure = &table.measures[0];
    assert_eq!(measure.lineage_tag.as_deref(), Some("measure-guid-1"));
    assert_eq!(measure.annotations.len(), 1);
    assert_eq!(measure.annotations[0].name, "DisplayFolder");
}
