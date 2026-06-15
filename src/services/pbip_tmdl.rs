//! PBIP TMDL semantic-model assembly.
//!
//! Walks a Power BI project-definition `definition/` directory, parses each
//! `.tmdl` file through the dedicated [`powerbi-tmdl-parser`](powerbi_tmdl_parser)
//! crate (via the existing [`crate::services::powerbi_tmdl`] adapter), and
//! merges the per-file fragments into one canonical [`PowerBiSemanticModel`].
//!
//! The PBIP layout splits a single semantic model across multiple sibling
//! files (`model.tmdl`, `relationships.tmdl`, `expressions.tmdl`,
//! `tables/*.tmdl`, etc.), so merging at the model level lets downstream
//! consumers — the pbip indexer in particular — treat the project-definition
//! semantic model as a single entity rather than reasoning about per-file
//! fragments.
//!
//! # 062.003-T scope
//!
//! Task 062.003-T introduces this assembler and the `.pbism` descriptor parser
//! in [`crate::services::pbip_extract`]. Wiring the assembler into the
//! [`crate::services::pbip_indexer`] dispatch path so the result lands as
//! `powerbi_semantic_model` content records and graph nodes is the job of
//! 062.002-T.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tracing::warn;

use crate::models::powerbi::{
    PowerBiColumn, PowerBiDataSource, PowerBiExpression, PowerBiMeasure, PowerBiRelationship,
    PowerBiSemanticModel, PowerBiTable,
};
use crate::services::powerbi_tmdl::extract_tmdl_semantic_model;

/// Assemble a [`PowerBiSemanticModel`] from a PBIP `definition/` directory.
///
/// Recursively walks `definition_dir` for `.tmdl` files, hands each one to
/// [`extract_tmdl_semantic_model`] (which delegates to `powerbi-tmdl-parser`),
/// and merges the per-file results into one canonical semantic model.
///
/// # Merge semantics
///
/// * The model `id` and `path` are taken from the first parsed file. Because
///   `extract_tmdl_semantic_model` derives both from a canonical
///   `definition/` path, every per-file fragment yields the same `id` so the
///   merged model has a stable identity.
/// * The model `name` is taken from the first parsed file that carries an
///   explicit `model <Name>` declaration. Fragments whose name was inferred
///   from the file path (e.g. `tables/Sales.tmdl` -> `"Sales"`) defer to a
///   real model declaration when available.
/// * Tables are merged by synthetic ID. When the same table appears in
///   multiple files (e.g. a `ref table Sales` line in `model.tmdl` and a
///   full `table Sales` block in `tables/Sales.tmdl`), the columns and
///   measures lists are unioned by their synthetic IDs.
/// * Relationships, expressions, and data sources are deduplicated by
///   synthetic ID, keeping the first parsed occurrence.
///
/// # Returns
///
/// `None` when the directory is missing, does not exist as a directory, or
/// contains no parseable TMDL fragments. `Some(model)` otherwise.
#[must_use]
pub fn extract_semantic_model_from_definition(
    definition_dir: &Path,
) -> Option<PowerBiSemanticModel> {
    if !definition_dir.is_dir() {
        return None;
    }

    let mut tmdl_files = collect_tmdl_files_recursive(definition_dir);
    tmdl_files.sort();
    if tmdl_files.is_empty() {
        return None;
    }

    let mut merged_id: Option<String> = None;
    let mut merged_path: Option<String> = None;
    let mut explicit_name: Option<String> = None;
    let mut fallback_name: Option<String> = None;
    let mut tables: HashMap<String, PowerBiTable> = HashMap::new();
    let mut table_order: Vec<String> = Vec::new();
    let mut relationships: HashMap<String, PowerBiRelationship> = HashMap::new();
    let mut relationship_order: Vec<String> = Vec::new();
    let mut expressions: HashMap<String, PowerBiExpression> = HashMap::new();
    let mut expression_order: Vec<String> = Vec::new();
    let mut data_sources: HashMap<String, PowerBiDataSource> = HashMap::new();
    let mut data_source_order: Vec<String> = Vec::new();

    for file in tmdl_files {
        let Ok(content) = std::fs::read_to_string(&file) else {
            warn!(path = %file.display(), "skipping unreadable TMDL file");
            continue;
        };
        let Some(file_path) = file.to_str() else {
            warn!(path = %file.display(), "skipping TMDL file with non-UTF-8 path");
            continue;
        };
        let Some(fragment) = extract_tmdl_semantic_model(&content, file_path) else {
            continue;
        };

        if merged_id.is_none() {
            merged_id = Some(fragment.id.clone());
            merged_path = Some(fragment.path.clone());
        }

        record_name(
            &mut explicit_name,
            &mut fallback_name,
            file_path,
            fragment.name,
        );

        for table in fragment.tables {
            merge_table_entry(&mut tables, &mut table_order, table);
        }
        for relationship in fragment.relationships {
            dedupe_insert(
                &mut relationships,
                &mut relationship_order,
                relationship.id.clone(),
                relationship,
            );
        }
        for expression in fragment.expressions {
            dedupe_insert(
                &mut expressions,
                &mut expression_order,
                expression.id.clone(),
                expression,
            );
        }
        for data_source in fragment.data_sources {
            dedupe_insert(
                &mut data_sources,
                &mut data_source_order,
                data_source.id.clone(),
                data_source,
            );
        }
    }

    let id = merged_id?;
    let path = merged_path?;
    let name = explicit_name
        .or(fallback_name)
        .unwrap_or_else(|| "Unknown Model".to_string());

    Some(PowerBiSemanticModel {
        id,
        name,
        path,
        tables: drain_ordered(tables, &table_order),
        relationships: drain_ordered(relationships, &relationship_order),
        expressions: drain_ordered(expressions, &expression_order),
        data_sources: drain_ordered(data_sources, &data_source_order),
    })
}

/// Record the candidate model name. Prefer the first explicit declaration
/// (a name that does not match the file-derived fallback) and otherwise
/// fall back to the first parsed fragment's name.
fn record_name(
    explicit_name: &mut Option<String>,
    fallback_name: &mut Option<String>,
    file_path: &str,
    fragment_name: String,
) {
    if fallback_name.is_none() {
        *fallback_name = Some(fragment_name.clone());
    }

    if explicit_name.is_some() {
        return;
    }

    let inferred = inferred_name_from_file_path(file_path);
    let is_explicit = inferred.as_deref() != Some(fragment_name.as_str())
        || file_name(file_path).eq_ignore_ascii_case("model.tmdl");

    if is_explicit {
        *explicit_name = Some(fragment_name);
    }
}

fn file_name(file_path: &str) -> &str {
    Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path)
}

fn inferred_name_from_file_path(file_path: &str) -> Option<String> {
    let path = Path::new(file_path);
    for ancestor in path.ancestors() {
        let name = ancestor.file_name()?.to_str()?;
        if name == "definition" {
            let parent = ancestor.parent()?.file_name()?.to_str()?;
            return Some(parent.trim_end_matches(".SemanticModel").to_string());
        }
    }
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.trim_end_matches(".tmdl").to_string())
}

fn merge_table_entry(
    tables: &mut HashMap<String, PowerBiTable>,
    table_order: &mut Vec<String>,
    incoming: PowerBiTable,
) {
    if let Some(existing) = tables.get_mut(&incoming.id) {
        union_columns(&mut existing.columns, &incoming.columns);
        union_measures(&mut existing.measures, &incoming.measures);
    } else {
        table_order.push(incoming.id.clone());
        tables.insert(incoming.id.clone(), incoming);
    }
}

fn union_columns(existing: &mut Vec<PowerBiColumn>, incoming: &[PowerBiColumn]) {
    let existing_ids: HashSet<String> = existing.iter().map(|c| c.id.clone()).collect();
    for column in incoming {
        if !existing_ids.contains(&column.id) {
            existing.push(column.clone());
        }
    }
}

fn union_measures(existing: &mut Vec<PowerBiMeasure>, incoming: &[PowerBiMeasure]) {
    let existing_ids: HashSet<String> = existing.iter().map(|m| m.id.clone()).collect();
    for measure in incoming {
        if !existing_ids.contains(&measure.id) {
            existing.push(measure.clone());
        }
    }
}

fn dedupe_insert<T>(map: &mut HashMap<String, T>, order: &mut Vec<String>, id: String, value: T) {
    map.entry(id.clone()).or_insert_with(|| {
        order.push(id);
        value
    });
}

fn drain_ordered<T>(mut map: HashMap<String, T>, order: &[String]) -> Vec<T> {
    let mut result = Vec::with_capacity(order.len());
    for id in order {
        if let Some(value) = map.remove(id) {
            result.push(value);
        }
    }
    result
}

fn collect_tmdl_files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            warn!(
                path = %dir.display(),
                error = %err,
                "skipping TMDL directory that could not be read"
            );
            return files;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_tmdl_files_recursive(&path));
        } else if path.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tmdl"))
        {
            files.push(path);
        }
    }

    files
}
