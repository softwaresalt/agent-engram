//! Extraction of JSON-backed PBIP entities into Power BI entity types.
//!
//! Parses PBIP JSON descriptors (report JSON, `model.bim`) into the
//! [`crate::models::powerbi`] entity types used by the search indexer.
//!
//! # JSON formats
//!
//! * **Report JSON**: top-level object with `displayName` and
//!   `reportSections` array.  Each section contains `displayName`, `ordinal`,
//!   and a `visualContainers` array.  Visual containers carry a `config`
//!   field that is either an escaped JSON string or a plain object; the
//!   extractor handles both forms.
//! * **Semantic model (`model.bim`)**: top-level object with a `model`
//!   key containing `tables` and `relationships` arrays.

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::models::powerbi::{
    PowerBiColumn, PowerBiDataSource, PowerBiExpression, PowerBiMeasure, PowerBiPage,
    PowerBiRelationship, PowerBiReport, PowerBiSemanticModel, PowerBiTable, PowerBiVisual,
};

// ── ID derivation ─────────────────────────────────────────────────────────

/// Derive a stable 16-character synthetic ID from a namespace string.
///
/// The ID is the first 16 hexadecimal characters of the SHA-256 digest of
/// `namespace`, which gives sufficient collision resistance for
/// workspace-relative entity identifiers while remaining readable in logs.
#[must_use]
pub fn synthetic_id(namespace: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(namespace.as_bytes());
    let hash = hex::encode(hasher.finalize());
    hash[..16].to_string()
}

// ── Report extraction ─────────────────────────────────────────────────────

/// Extract a [`PowerBiReport`] from a parsed report JSON `Value`.
///
/// Accepts the top-level object of a PBIP `report.json` descriptor and a
/// workspace-relative `report_path` used to derive stable entity IDs.
///
/// Returns `None` when the JSON contains neither a `displayName`/`name` field
/// nor a recognisable `reportSections` array and therefore cannot be
/// meaningfully indexed.
#[must_use]
pub fn extract_report(json: &Value, report_path: &str) -> Option<PowerBiReport> {
    // A report object must have either a display name or a sections array to be
    // considered a valid report descriptor.
    let has_name = json
        .get("displayName")
        .or_else(|| json.get("name"))
        .is_some();
    let has_sections = json.get("reportSections").is_some();
    if !has_name && !has_sections {
        return None;
    }

    let name = json
        .get("displayName")
        .or_else(|| json.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown Report")
        .to_string();

    let id = synthetic_id(&format!("report:{report_path}"));

    let pages = json
        .get("reportSections")
        .and_then(Value::as_array)
        .map(|sections| {
            sections
                .iter()
                .filter_map(|s| extract_page(s, report_path))
                .collect()
        })
        .unwrap_or_default();

    Some(PowerBiReport {
        id,
        name,
        path: report_path.to_string(),
        pages,
    })
}

/// Extract a [`PowerBiPage`] from a `reportSection` JSON object.
fn extract_page(section: &Value, report_path: &str) -> Option<PowerBiPage> {
    // Require at least one name source.
    let name = section
        .get("displayName")
        .and_then(Value::as_str)
        .or_else(|| section.get("name").and_then(Value::as_str))?
        .to_string();

    let ordinal = section
        .get("ordinal")
        .and_then(Value::as_u64)
        .map(|n| u32::try_from(n).unwrap_or(u32::MAX))
        .unwrap_or(0);

    let id = synthetic_id(&format!("page:{report_path}:{name}:{ordinal}"));

    let visuals = section
        .get("visualContainers")
        .and_then(Value::as_array)
        .map(|containers| {
            containers
                .iter()
                .enumerate()
                .map(|(i, vc)| extract_visual(vc, &id, i))
                .collect()
        })
        .unwrap_or_default();

    Some(PowerBiPage {
        id,
        name,
        ordinal,
        visuals,
    })
}

/// Extract a [`PowerBiVisual`] from a `visualContainer` JSON object.
///
/// The `config` field can be either an escaped JSON string (as produced by
/// Power BI Desktop) or a plain object.  Both forms are supported.
fn extract_visual(container: &Value, page_id: &str, index: usize) -> PowerBiVisual {
    let visual_type = resolve_visual_type(container);
    let name = format!("{visual_type} {}", index + 1);
    let id = synthetic_id(&format!("visual:{page_id}:{visual_type}:{index}"));

    PowerBiVisual {
        id,
        name,
        visual_type,
    }
}

/// Resolve the `visualType` string from a visual container, handling both
/// escaped-string and plain-object `config` fields.
fn resolve_visual_type(container: &Value) -> String {
    // Try the `config` field as an escaped JSON string first (production format).
    if let Some(config_str) = container.get("config").and_then(Value::as_str) {
        if let Ok(config_json) = serde_json::from_str::<Value>(config_str) {
            if let Some(vt) = config_json
                .pointer("/singleVisual/visualType")
                .and_then(Value::as_str)
            {
                return vt.to_string();
            }
        }
    }

    // Try `config` as a plain object (test fixtures and some export formats).
    if let Some(vt) = container
        .pointer("/config/singleVisual/visualType")
        .and_then(Value::as_str)
    {
        return vt.to_string();
    }

    // Direct `visualType` field (simplified fixtures).
    if let Some(vt) = container.get("visualType").and_then(Value::as_str) {
        return vt.to_string();
    }

    "unknown".to_string()
}

// ── Semantic model extraction ─────────────────────────────────────────────

/// Extract a [`PowerBiSemanticModel`] from a parsed `model.bim` JSON `Value`.
///
/// Accepts the top-level object of a `model.bim` file and a workspace-relative
/// `model_path` used to derive stable entity IDs.
///
/// Returns `None` when the JSON contains neither a `model` key nor a `tables`
/// array at the top level and therefore cannot be meaningfully indexed.
#[must_use]
pub fn extract_semantic_model(json: &Value, model_path: &str) -> Option<PowerBiSemanticModel> {
    // Accept either `{ "model": { ... } }` (standard model.bim) or a raw object
    // with `tables` directly (simplified test fixtures).
    let model_root = json.get("model").unwrap_or(json);

    model_root.get("tables")?;

    // Derive a human-readable model name from the parent folder or an explicit
    // `name` field on the root object.
    let name = json
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| {
            std::path::Path::new(model_path)
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
        })
        .unwrap_or("Unknown Model")
        .to_string();

    let id = synthetic_id(&format!("model:{model_path}"));

    let tables = model_root
        .get("tables")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(|t| extract_table(t, &id)).collect())
        .unwrap_or_default();

    let relationships = model_root
        .get("relationships")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|r| extract_relationship(r, &id))
                .collect()
        })
        .unwrap_or_default();

    let expressions = model_root
        .get("expressions")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|expression| extract_expression(expression, &id))
                .collect()
        })
        .unwrap_or_default();

    let data_sources = model_root
        .get("dataSources")
        .or_else(|| model_root.get("datasources"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .enumerate()
                .map(|(i, ds)| {
                    let ds_name = ds
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("DataSource")
                        .to_string();
                    let source_type = ds.get("type").and_then(Value::as_str).map(String::from);
                    PowerBiDataSource {
                        id: synthetic_id(&format!("datasource:{id}:{i}")),
                        name: ds_name,
                        source_type,
                        kind: None,
                        provider: None,
                        connection_string: None,
                        server: None,
                        database: None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Some(PowerBiSemanticModel {
        id,
        name,
        path: model_path.to_string(),
        tables,
        relationships,
        expressions,
        data_sources,
    })
}

/// Extract a [`PowerBiTable`] from a model table JSON object.
fn extract_table(table: &Value, model_id: &str) -> Option<PowerBiTable> {
    let name = table.get("name").and_then(Value::as_str)?.to_string();
    let id = synthetic_id(&format!("table:{model_id}:{name}"));

    let columns = table
        .get("columns")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(|c| extract_column(c, &id)).collect())
        .unwrap_or_default();

    let measures = table
        .get("measures")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(|m| extract_measure(m, &id)).collect())
        .unwrap_or_default();

    Some(PowerBiTable {
        id,
        name,
        columns,
        measures,
        partitions: Vec::new(),
    })
}

/// Extract a [`PowerBiColumn`] from a table column JSON object.
fn extract_column(column: &Value, table_id: &str) -> Option<PowerBiColumn> {
    let name = column.get("name").and_then(Value::as_str)?.to_string();
    let id = synthetic_id(&format!("column:{table_id}:{name}"));
    let data_type = column
        .get("dataType")
        .or_else(|| column.get("type"))
        .and_then(Value::as_str)
        .map(String::from);

    Some(PowerBiColumn {
        id,
        name,
        data_type,
    })
}

/// Extract a [`PowerBiMeasure`] from a table measure JSON object.
fn extract_measure(measure: &Value, table_id: &str) -> Option<PowerBiMeasure> {
    let name = measure.get("name").and_then(Value::as_str)?.to_string();
    let id = synthetic_id(&format!("measure:{table_id}:{name}"));
    let expression = measure
        .get("expression")
        .and_then(Value::as_str)
        .map(String::from);

    Some(PowerBiMeasure {
        id,
        name,
        expression,
    })
}

/// Extract a [`PowerBiExpression`] from a model expression JSON object.
fn extract_expression(expression: &Value, model_id: &str) -> Option<PowerBiExpression> {
    let name = expression.get("name").and_then(Value::as_str)?.to_string();
    let id = synthetic_id(&format!("expression:{model_id}:{name}"));
    let expression_text = expression
        .get("expression")
        .and_then(Value::as_str)
        .map(String::from);

    Some(PowerBiExpression {
        id,
        name,
        expression: expression_text,
    })
}

/// Extract a [`PowerBiRelationship`] from a model relationship JSON object.
fn extract_relationship(rel: &Value, model_id: &str) -> Option<PowerBiRelationship> {
    let from_table = rel.get("fromTable").and_then(Value::as_str)?.to_string();
    let from_column = rel.get("fromColumn").and_then(Value::as_str)?.to_string();
    let to_table = rel.get("toTable").and_then(Value::as_str)?.to_string();
    let to_column = rel.get("toColumn").and_then(Value::as_str)?.to_string();

    let id = synthetic_id(&format!(
        "rel:{model_id}:{from_table}:{from_column}:{to_table}:{to_column}"
    ));

    Some(PowerBiRelationship {
        id,
        from_table,
        from_column,
        to_table,
        to_column,
    })
}
