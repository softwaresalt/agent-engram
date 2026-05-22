//! Extraction of TMDL semantic model assets into Power BI entity types.
//!
//! TMDL semantic models are folder-based and store tabular objects in plain-text
//! `*.tmdl` files rather than a single `model.bim` JSON file. This module parses
//! the subset of TMDL needed for structural indexing: model names, tables,
//! columns, measures, relationships, and data sources.

use std::path::Path;

use crate::models::powerbi::{
    PowerBiColumn, PowerBiDataSource, PowerBiMeasure, PowerBiRelationship, PowerBiSemanticModel,
    PowerBiTable,
};
use crate::services::powerbi_extract::synthetic_id;

#[derive(Debug, Default)]
struct TmdlTableDraft {
    name: String,
    columns: Vec<PowerBiColumnDraft>,
    measures: Vec<PowerBiMeasureDraft>,
    last_member: Option<TmdlMemberKind>,
}

#[derive(Debug)]
enum TmdlMemberKind {
    Column(usize),
    Measure(usize),
}

#[derive(Debug)]
struct PowerBiColumnDraft {
    name: String,
    data_type: Option<String>,
}

#[derive(Debug)]
struct PowerBiMeasureDraft {
    name: String,
    expression: Option<String>,
}

/// Extract a [`PowerBiSemanticModel`] from TMDL text content.
///
/// `model_path` should identify the semantic model folder or canonical TMDL
/// grouping path so stable IDs remain deterministic across re-indexing runs.
#[must_use]
pub fn extract_tmdl_semantic_model(
    tmdl_content: &str,
    model_path: &str,
) -> Option<PowerBiSemanticModel> {
    let model_scope = canonical_tmdl_model_path(model_path);
    let mut explicit_model_name = None;
    let mut tables: Vec<TmdlTableDraft> = Vec::new();
    let mut current_table = None;
    let mut relationships = Vec::new();
    let mut data_sources = Vec::new();

    for line in tmdl_content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
    {
        if let Some(rest) = line.strip_prefix("model ") {
            explicit_model_name = Some(parse_identifier(rest));
            continue;
        }

        if let Some(rest) = line.strip_prefix("table ") {
            flush_table(&mut tables, &mut current_table);
            current_table = Some(TmdlTableDraft {
                name: parse_identifier(rest),
                ..TmdlTableDraft::default()
            });
            continue;
        }

        if let Some(rest) = line.strip_prefix("column ") {
            let Some(table) = current_table.as_mut() else {
                continue;
            };

            let column_name = parse_identifier(rest);
            table.columns.push(PowerBiColumnDraft {
                name: column_name,
                data_type: None,
            });
            table.last_member = Some(TmdlMemberKind::Column(table.columns.len() - 1));
            continue;
        }

        if let Some(rest) = line.strip_prefix("measure ") {
            let Some(table) = current_table.as_mut() else {
                continue;
            };

            let (name, expression) = parse_measure(rest);
            table
                .measures
                .push(PowerBiMeasureDraft { name, expression });
            table.last_member = Some(TmdlMemberKind::Measure(table.measures.len() - 1));
            continue;
        }

        if let Some(rest) = line.strip_prefix("dataType:") {
            let Some(table) = current_table.as_mut() else {
                continue;
            };
            if let Some(TmdlMemberKind::Column(index)) = table.last_member {
                table.columns[index].data_type = Some(parse_identifier(rest));
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("expression:") {
            let Some(table) = current_table.as_mut() else {
                continue;
            };
            if let Some(TmdlMemberKind::Measure(index)) = table.last_member {
                table.measures[index].expression = Some(rest.trim().to_string());
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("relationship ") {
            if let Some(relationship) = parse_relationship(rest, &model_scope) {
                relationships.push(relationship);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("dataSource ") {
            let name = parse_identifier(rest);
            let model_id = synthetic_id(&format!("model:{model_scope}"));
            data_sources.push(PowerBiDataSource {
                id: synthetic_id(&format!("datasource:{model_id}:{name}")),
                name,
                source_type: None,
            });
        }
    }

    flush_table(&mut tables, &mut current_table);

    if tables.is_empty() && relationships.is_empty() && data_sources.is_empty() {
        return None;
    }

    let model_name = explicit_model_name.unwrap_or_else(|| {
        infer_model_name(&model_scope).unwrap_or_else(|| "Unknown Model".to_string())
    });
    let model_id = synthetic_id(&format!("model:{model_scope}"));

    let tables = tables
        .into_iter()
        .map(|table| build_table(table, &model_id))
        .collect();

    Some(PowerBiSemanticModel {
        id: model_id,
        name: model_name,
        path: model_scope,
        tables,
        relationships,
        data_sources,
    })
}

#[must_use]
pub fn canonical_tmdl_model_path(model_path: &str) -> String {
    let path = Path::new(model_path);
    for ancestor in path.ancestors() {
        if ancestor
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "definition")
        {
            return ancestor.to_string_lossy().replace('\\', "/");
        }
    }

    model_path.replace('\\', "/")
}

fn flush_table(tables: &mut Vec<TmdlTableDraft>, current_table: &mut Option<TmdlTableDraft>) {
    if let Some(table) = current_table.take() {
        tables.push(table);
    }
}

fn build_table(table: TmdlTableDraft, model_id: &str) -> PowerBiTable {
    let table_id = synthetic_id(&format!("table:{model_id}:{}", table.name));
    let columns = table
        .columns
        .into_iter()
        .map(|column| PowerBiColumn {
            id: synthetic_id(&format!("column:{table_id}:{}", column.name)),
            name: column.name,
            data_type: column.data_type,
        })
        .collect();
    let measures = table
        .measures
        .into_iter()
        .map(|measure| PowerBiMeasure {
            id: synthetic_id(&format!("measure:{table_id}:{}", measure.name)),
            name: measure.name,
            expression: measure.expression,
        })
        .collect();

    PowerBiTable {
        id: table_id,
        name: table.name,
        columns,
        measures,
    }
}

fn parse_measure(rest: &str) -> (String, Option<String>) {
    let mut parts = rest.splitn(2, '=');
    let name = parse_identifier(parts.next().unwrap_or_default());
    let expression = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    (name, expression)
}

fn parse_relationship(rest: &str, model_path: &str) -> Option<PowerBiRelationship> {
    let mut parts = rest.splitn(2, "->");
    let lhs = parts.next()?.trim();
    let rhs = parts.next()?.trim();

    let (from_table, from_column) = parse_table_column(lhs)?;
    let (to_table, to_column) = parse_table_column(rhs)?;
    let model_id = synthetic_id(&format!("model:{model_path}"));
    let rel_name = format!("{from_table}.{from_column}->{to_table}.{to_column}");

    Some(PowerBiRelationship {
        id: synthetic_id(&format!("relationship:{model_id}:{rel_name}")),
        from_table,
        from_column,
        to_table,
        to_column,
    })
}

fn parse_table_column(value: &str) -> Option<(String, String)> {
    let (table, column) = value.split_once('.')?;
    Some((parse_identifier(table), parse_identifier(column)))
}

fn parse_identifier(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn infer_model_name(model_path: &str) -> Option<String> {
    let path = Path::new(model_path);
    for ancestor in path.ancestors() {
        let name = ancestor.file_name()?.to_str()?;
        if name == "definition" {
            let parent = ancestor.parent()?.file_name()?.to_str()?;
            return Some(parent.trim_end_matches(".SemanticModel").to_string());
        }
    }

    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.trim_end_matches(".tmdl").to_string())
}
