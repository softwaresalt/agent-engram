//! Extraction of TMDL semantic model assets into Power BI entity types.
//!
//! The parsing boundary lives in the dedicated `powerbi-tmdl-parser` crate so
//! Engram can consume TMDL structure through a focused Power BI interface rather
//! than hard-coding all parsing inside the main daemon crate.

use std::path::Path;

use powerbi_tmdl_parser::{
    TmdlAnnotation, TmdlDataSource, TmdlExpression, TmdlModel, TmdlRef, TmdlRelationship, TmdlTable,
    parse_tmdl_document,
};

use crate::models::powerbi::{
    PowerBiAnnotation, PowerBiColumn, PowerBiDataSource, PowerBiExpression, PowerBiMeasure,
    PowerBiPartition, PowerBiRef, PowerBiRelationship, PowerBiSemanticModel, PowerBiTable,
};
use crate::services::powerbi_extract::synthetic_id;

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
    let parsed = parse_tmdl_document(tmdl_content)?;
    let TmdlModel {
        model_name,
        tables,
        relationships,
        expressions,
        data_sources,
        refs,
        annotations,
        culture,
        default_mode,
        lineage_tag,
    } = parsed;
    let model_name = model_name.unwrap_or_else(|| {
        infer_model_name(&model_scope).unwrap_or_else(|| "Unknown Model".to_string())
    });
    let model_id = synthetic_id(&format!("model:{model_scope}"));

    let tables = tables
        .into_iter()
        .map(|table| build_table(table, &model_id))
        .collect();
    let relationships = relationships
        .into_iter()
        .map(|relationship| build_relationship(relationship, &model_id))
        .collect();
    let expressions = expressions
        .into_iter()
        .map(|expression| build_expression(expression, &model_id))
        .collect();
    let data_sources = data_sources
        .into_iter()
        .map(|data_source| build_data_source(data_source, &model_id))
        .collect();
    let refs = refs.into_iter().map(build_ref).collect();
    let annotations = annotations.into_iter().map(build_annotation).collect();

    Some(PowerBiSemanticModel {
        id: model_id,
        name: model_name,
        path: model_scope,
        tables,
        relationships,
        expressions,
        data_sources,
        refs,
        annotations,
        culture,
        default_mode,
        lineage_tag,
    })
}

/// Normalize a TMDL file path to the semantic-model `definition` directory.
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

fn build_table(table: TmdlTable, model_id: &str) -> PowerBiTable {
    let table_id = synthetic_id(&format!("table:{model_id}:{}", table.name));
    let columns = table
        .columns
        .into_iter()
        .map(|column| PowerBiColumn {
            id: synthetic_id(&format!("column:{table_id}:{}", column.name)),
            name: column.name,
            data_type: column.data_type,
            annotations: column.annotations.into_iter().map(build_annotation).collect(),
            lineage_tag: column.lineage_tag,
        })
        .collect();
    let measures = table
        .measures
        .into_iter()
        .map(|measure| PowerBiMeasure {
            id: synthetic_id(&format!("measure:{table_id}:{}", measure.name)),
            name: measure.name,
            expression: measure.expression,
            annotations: measure
                .annotations
                .into_iter()
                .map(build_annotation)
                .collect(),
            lineage_tag: measure.lineage_tag,
        })
        .collect();
    let partitions = table
        .partitions
        .into_iter()
        .map(|partition| PowerBiPartition {
            id: synthetic_id(&format!("partition:{table_id}:{}", partition.name)),
            name: partition.name,
            source_kind: partition.source_kind,
            mode: partition.mode,
            source_expression: partition.source_expression,
        })
        .collect();

    PowerBiTable {
        id: table_id,
        name: table.name,
        columns,
        measures,
        partitions,
        annotations: table.annotations.into_iter().map(build_annotation).collect(),
        lineage_tag: table.lineage_tag,
    }
}

fn build_annotation(annotation: TmdlAnnotation) -> PowerBiAnnotation {
    PowerBiAnnotation {
        name: annotation.name,
        value: annotation.value,
    }
}

fn build_ref(reference: TmdlRef) -> PowerBiRef {
    PowerBiRef {
        kind: reference.kind,
        name: reference.name,
    }
}

fn build_relationship(relationship: TmdlRelationship, model_id: &str) -> PowerBiRelationship {
    let rel_name = format!(
        "{}.{}->{}.{}",
        relationship.from_table,
        relationship.from_column,
        relationship.to_table,
        relationship.to_column
    );

    PowerBiRelationship {
        id: synthetic_id(&format!("relationship:{model_id}:{rel_name}")),
        from_table: relationship.from_table,
        from_column: relationship.from_column,
        to_table: relationship.to_table,
        to_column: relationship.to_column,
    }
}

fn build_expression(expression: TmdlExpression, model_id: &str) -> PowerBiExpression {
    PowerBiExpression {
        id: synthetic_id(&format!("expression:{model_id}:{}", expression.name)),
        name: expression.name,
        expression: expression.expression,
    }
}

fn build_data_source(data_source: TmdlDataSource, model_id: &str) -> PowerBiDataSource {
    PowerBiDataSource {
        id: synthetic_id(&format!("datasource:{model_id}:{}", data_source.name)),
        name: data_source.name,
        source_type: None,
        kind: data_source.kind,
        provider: data_source.provider,
        connection_string: data_source.connection_string,
        server: data_source.server,
        database: data_source.database,
    }
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
