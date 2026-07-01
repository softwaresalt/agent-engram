//! Unit tests for Power BI JSON entity extraction (061.003-T).
//!
//! Validates [`powerbi_extract`] functions against inline JSON fixtures
//! representative of real PBIP report and model.bim structures.
//!
//! Tests: S-PEX-01 through S-PEX-14

use engram::services::powerbi_extract::{extract_report, extract_semantic_model, synthetic_id};

// ── Fixtures ──────────────────────────────────────────────────────────────

/// Minimal PBIP `report.json` with two pages and inline visual containers.
fn report_json_fixture() -> serde_json::Value {
    serde_json::json!({
        "id": "abc123",
        "displayName": "Sales Dashboard",
        "reportSections": [
            {
                "name": "section1",
                "displayName": "Overview",
                "ordinal": 1,
                "visualContainers": [
                    { "visualType": "barChart" },
                    { "visualType": "lineChart" }
                ]
            },
            {
                "name": "section2",
                "displayName": "Details",
                "ordinal": 2,
                "visualContainers": []
            }
        ]
    })
}

/// Minimal `model.bim` with one table, two columns, one measure, and
/// one relationship.
fn model_bim_fixture() -> serde_json::Value {
    serde_json::json!({
        "model": {
            "tables": [
                {
                    "name": "Sales",
                    "columns": [
                        { "name": "Date", "dataType": "dateTime" },
                        { "name": "Amount", "dataType": "double" }
                    ],
                    "measures": [
                        {
                            "name": "Total Sales",
                            "expression": "SUM(Sales[Amount])"
                        }
                    ]
                }
            ],
            "relationships": [
                {
                    "fromTable": "Sales",
                    "fromColumn": "ProductID",
                    "toTable": "Products",
                    "toColumn": "ID"
                }
            ]
        }
    })
}

// ── Report extraction tests ───────────────────────────────────────────────

/// S-PEX-01: Report extraction returns the correct display name and page count.
#[test]
fn extract_report_produces_correct_name_and_pages() {
    let json = report_json_fixture();
    let report = extract_report(&json, "reports/sales/report.json")
        .expect("fixture should produce a report");

    assert_eq!(report.name, "Sales Dashboard");
    assert_eq!(report.pages.len(), 2, "fixture has two pages");
}

/// S-PEX-02: Page display names and ordinals are extracted correctly.
#[test]
fn extract_report_page_names_and_ordinals() {
    let json = report_json_fixture();
    let report = extract_report(&json, "reports/sales/report.json").unwrap();

    assert_eq!(report.pages[0].name, "Overview");
    assert_eq!(report.pages[0].ordinal, 1);
    assert_eq!(report.pages[1].name, "Details");
    assert_eq!(report.pages[1].ordinal, 2);
}

/// S-PEX-03: Visual type is extracted from a plain `visualType` field.
#[test]
fn extract_report_visuals_from_plain_field() {
    let json = report_json_fixture();
    let report = extract_report(&json, "reports/sales/report.json").unwrap();

    let overview = &report.pages[0];
    assert_eq!(
        overview.visuals.len(),
        2,
        "Overview page should have 2 visuals"
    );
    assert_eq!(overview.visuals[0].visual_type, "barChart");
    assert_eq!(overview.visuals[1].visual_type, "lineChart");
}

/// S-PEX-04: Visual type is extracted from an escaped-JSON `config` string.
#[test]
fn extract_report_visuals_from_escaped_config_string() {
    let json = serde_json::json!({
        "displayName": "Config Report",
        "reportSections": [
            {
                "displayName": "Page 1",
                "ordinal": 1,
                "visualContainers": [
                    {
                        "config": r#"{"singleVisual":{"visualType":"card"}}"#
                    }
                ]
            }
        ]
    });
    let report = extract_report(&json, "reports/config/report.json").unwrap();
    assert_eq!(report.pages[0].visuals[0].visual_type, "card");
}

/// S-PEX-05: Page without visuals produces an empty `visuals` vec (not an error).
#[test]
fn extract_report_page_with_no_visuals_is_valid() {
    let json = report_json_fixture();
    let report = extract_report(&json, "reports/sales/report.json").unwrap();
    assert!(
        report.pages[1].visuals.is_empty(),
        "Details page has no visuals"
    );
}

/// S-PEX-06: JSON that lacks a report structure returns `None`.
#[test]
fn extract_report_returns_none_for_non_report_json() {
    let json = serde_json::json!({ "random_key": 42 });
    let result = extract_report(&json, "not/a/report.json");
    assert!(
        result.is_none(),
        "non-report JSON should produce None, not an error"
    );
}

// ── Semantic model extraction tests ───────────────────────────────────────

/// S-PEX-07: Semantic model extraction returns the correct table and measure count.
#[test]
fn extract_semantic_model_tables_and_measures() {
    let json = model_bim_fixture();
    let model = extract_semantic_model(&json, "semantic/model.bim")
        .expect("fixture should produce a model");

    assert_eq!(model.tables.len(), 1);
    let table = &model.tables[0];
    assert_eq!(table.name, "Sales");
    assert_eq!(table.columns.len(), 2);
    assert_eq!(table.measures.len(), 1);
    assert_eq!(table.measures[0].name, "Total Sales");
}

/// S-PEX-08: Column data types are extracted when present.
#[test]
fn extract_semantic_model_column_data_types() {
    let json = model_bim_fixture();
    let model = extract_semantic_model(&json, "semantic/model.bim").unwrap();
    let table = &model.tables[0];

    let date_col = table.columns.iter().find(|c| c.name == "Date").unwrap();
    assert_eq!(date_col.data_type.as_deref(), Some("dateTime"));

    let amt_col = table.columns.iter().find(|c| c.name == "Amount").unwrap();
    assert_eq!(amt_col.data_type.as_deref(), Some("double"));
}

/// S-PEX-09: DAX measure expression is preserved.
#[test]
fn extract_semantic_model_measure_expression() {
    let json = model_bim_fixture();
    let model = extract_semantic_model(&json, "semantic/model.bim").unwrap();
    let measure = &model.tables[0].measures[0];
    assert_eq!(measure.expression.as_deref(), Some("SUM(Sales[Amount])"));
}

/// S-PEX-10: Relationship endpoints are extracted correctly.
#[test]
fn extract_semantic_model_relationships() {
    let json = model_bim_fixture();
    let model = extract_semantic_model(&json, "semantic/model.bim").unwrap();

    assert_eq!(model.relationships.len(), 1);
    let rel = &model.relationships[0];
    assert_eq!(rel.from_table, "Sales");
    assert_eq!(rel.from_column, "ProductID");
    assert_eq!(rel.to_table, "Products");
    assert_eq!(rel.to_column, "ID");
}

/// S-PEX-11: Model with missing optional fields (no relationships, no columns)
/// does not panic and returns partial data.
#[test]
fn extract_semantic_model_tolerates_missing_optional_fields() {
    let json = serde_json::json!({
        "model": {
            "tables": [
                {
                    "name": "Minimal"
                }
            ]
        }
    });
    let model = extract_semantic_model(&json, "path/model.bim")
        .expect("model with minimal table should parse");

    assert_eq!(model.tables.len(), 1);
    assert!(model.tables[0].columns.is_empty());
    assert!(model.tables[0].measures.is_empty());
    assert!(model.relationships.is_empty());
}

/// S-PEX-12: JSON without a `tables` key returns `None`.
#[test]
fn extract_semantic_model_returns_none_for_non_model_json() {
    let json = serde_json::json!({ "random_key": "value" });
    let result = extract_semantic_model(&json, "not/a/model.bim");
    assert!(
        result.is_none(),
        "non-model JSON should produce None, not an error"
    );
}

/// S-PEX-15: Top-level semantic-model expressions are extracted when present.
#[test]
fn extract_semantic_model_top_level_expressions() {
    let json = serde_json::json!({
        "model": {
            "tables": [
                { "name": "Sales" }
            ],
            "expressions": [
                {
                    "name": "SynapseDatabase",
                    "expression": "\"ILSOS_EDW\""
                }
            ]
        }
    });
    let model = extract_semantic_model(&json, "semantic/model.bim")
        .expect("fixture should produce a model");

    assert_eq!(model.expressions.len(), 1);
    assert_eq!(model.expressions[0].name, "SynapseDatabase");
    assert_eq!(
        model.expressions[0].expression.as_deref(),
        Some("\"ILSOS_EDW\"")
    );
}

// ── Synthetic ID stability tests ──────────────────────────────────────────

/// S-PEX-13: `synthetic_id` is deterministic — the same input always produces
/// the same 16-character ID.
#[test]
fn synthetic_id_is_deterministic() {
    let id1 = synthetic_id("report:reports/sales/report.json");
    let id2 = synthetic_id("report:reports/sales/report.json");
    assert_eq!(id1, id2, "synthetic_id must be deterministic");
    assert_eq!(id1.len(), 16, "synthetic_id must be 16 characters");
}

/// S-PEX-14: `synthetic_id` produces distinct IDs for distinct namespaces.
#[test]
fn synthetic_id_is_distinct_for_different_inputs() {
    let id_a = synthetic_id("report:reports/sales/report.json");
    let id_b = synthetic_id("report:reports/marketing/report.json");
    assert_ne!(id_a, id_b, "distinct namespaces must produce distinct IDs");
}
