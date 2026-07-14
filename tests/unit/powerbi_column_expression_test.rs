//! Unit tests for calculated-column DAX carried onto `PowerBiColumn` (085.002-T, P2).
//!
//! Verifies the TMDL adapter carries `TmdlColumn.expression` onto the engram
//! `PowerBiColumn` model, and that the additive `expression` field is
//! serialization back-compatible (`serde(default)` / skip-when-`None`).

use engram::models::powerbi::PowerBiColumn;
use engram::services::powerbi_tmdl::extract_tmdl_semantic_model;

/// A TMDL table with one plain column and one single-line calculated column.
fn tmdl_with_calculated_column() -> &'static str {
    "
model Sales Model

table Sales
  column Amount
    dataType: double
  column FullName = Sales[First] & Sales[Last]
    dataType: string
"
}

/// P2-01: a calculated column carries its DAX expression; a plain column carries
/// `None`.
#[test]
fn adapter_carries_calculated_column_expression() {
    let model = extract_tmdl_semantic_model(
        tmdl_with_calculated_column(),
        "models/Sales.SemanticModel/definition",
    )
    .expect("fixture should produce a semantic model");

    let table = &model.tables[0];
    let amount = table
        .columns
        .iter()
        .find(|c| c.name == "Amount")
        .expect("Amount column present");
    let full_name = table
        .columns
        .iter()
        .find(|c| c.name == "FullName")
        .expect("FullName column present");

    assert_eq!(
        amount.expression, None,
        "plain column carries no expression"
    );
    assert_eq!(
        full_name.expression.as_deref(),
        Some("Sales[First] & Sales[Last]"),
        "calculated column carries its DAX expression",
    );
}

/// P2-02: the additive `expression` field is serialization back-compatible — a
/// legacy payload without it deserializes as `None`, and `None` is omitted on
/// serialization.
#[test]
fn power_bi_column_expression_is_additive_and_skipped_when_none() {
    // Legacy payload (no `expression` key) deserializes with `expression = None`.
    let legacy = r#"{"id":"col1","name":"Amount","dataType":"double"}"#;
    let column: PowerBiColumn = serde_json::from_str(legacy).expect("legacy payload deserializes");
    assert_eq!(column.expression, None);

    // `None` is omitted on serialization (skip_serializing_if).
    let json = serde_json::to_string(&column).expect("serialize None expression");
    assert!(
        !json.contains("expression"),
        "None expression must be omitted from JSON: {json}"
    );

    // A populated expression round-trips.
    let with_expr = PowerBiColumn {
        expression: Some("SUM(Sales[Amount])".to_string()),
        ..column
    };
    let json = serde_json::to_string(&with_expr).expect("serialize populated expression");
    assert!(json.contains("expression"));
    let round_tripped: PowerBiColumn =
        serde_json::from_str(&json).expect("round-trip populated expression");
    assert_eq!(
        round_tripped.expression.as_deref(),
        Some("SUM(Sales[Amount])")
    );
}
