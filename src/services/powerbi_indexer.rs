//! Power BI content indexer for PBIP JSON-backed workspaces.
//!
//! Provides incremental, hash-based indexing of Power BI entities from PBIP
//! project folders into [`ContentRecord`](crate::models::ContentRecord) rows
//! in the `content_record` CozoDB relation.  The records use
//! `content_type = "powerbi"` and are returned by `unified_search` and
//! `query_memory` alongside records from other source types.
//!
//! # Incremental behaviour
//!
//! Each supported Power BI file (`report.json`, `model.bim`, `*.tmdl`) is hashed
//! on every indexer run. Files whose hash matches an existing record are skipped.
//! On each run a deletion sweep removes records for files that no longer exist
//! on disk.
//!
//! # Supported file types
//!
//! * `*.json` — report page descriptors and project manifests
//! * `*.bim` — tabular model definitions (`model.bim`)
//! * `*.tmdl` — folder-based semantic model assets

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use crate::db::queries::CodeGraphQueries;
use crate::errors::EngramError;
use crate::models::content::ContentRecord;
use crate::models::powerbi::PowerBiIndexResult;
use crate::models::powerbi_graph::{PowerBiEdge, PowerBiEdgeType, PowerBiNode, PowerBiNodeKind};
use crate::models::registry::ContentSource;
use crate::services::ingestion::{compute_hash, content_record_identity_seed};
use crate::services::powerbi_extract::{extract_report, extract_semantic_model};
use crate::services::powerbi_tmdl::{canonical_tmdl_model_path, extract_tmdl_semantic_model};

// ── Hash helpers ──────────────────────────────────────────────────────────

/// Compute a hex-encoded SHA-256 hash of `content` bytes.
///
/// Used for incremental change detection: a file whose hash matches the stored
/// value is skipped without re-indexing.
#[must_use]
pub fn compute_file_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Return the subset of `workspace_relative_paths` whose files no longer
/// exist under `workspace_root`.
///
/// Each path in `workspace_relative_paths` is joined to `workspace_root`
/// before the existence check so that workspace-relative record paths
/// (as stored in `ContentRecord.file_path`) are handled correctly.
#[must_use]
pub fn compute_deleted_paths(
    workspace_relative_paths: &[String],
    workspace_root: &Path,
) -> Vec<String> {
    workspace_relative_paths
        .iter()
        .filter_map(|rel| {
            let Some(relative_path) = workspace_relative_path(rel) else {
                warn!(
                    path = %rel,
                    "skipping Power BI deletion sweep path that escapes the workspace root"
                );
                return None;
            };

            (!workspace_root.join(relative_path).exists()).then(|| rel.clone())
        })
        .collect()
}

fn last_path_component(path: &str) -> &str {
    let trimmed = path.trim_end_matches(['/', '\\']);
    trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed)
}

fn workspace_relative_path(rel_path: &str) -> Option<PathBuf> {
    let path = Path::new(rel_path);
    if path.is_absolute()
        || path.components().any(|component| {
            component == Component::ParentDir || matches!(component, Component::Prefix(_))
        })
    {
        return None;
    }

    Some(path.to_path_buf())
}

// ── File collection ───────────────────────────────────────────────────────

/// Collect all indexable Power BI files under `dir` recursively.
///
/// Returns a sorted list of absolute paths to files with extensions
/// `.json`, `.bim`, or `.tmdl`.
#[must_use]
pub fn collect_powerbi_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_recursive(dir, &mut files);
    files.sort();
    files
}

fn collect_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, files);
        } else if path.is_file() {
            let is_target = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| {
                    ext.eq_ignore_ascii_case("json")
                        || ext.eq_ignore_ascii_case("bim")
                        || ext.eq_ignore_ascii_case("tmdl")
                })
                .unwrap_or(false);
            if is_target {
                files.push(path);
            }
        }
    }
}

// ── Entity summary extraction ─────────────────────────────────────────────

/// Classify `json` as a semantic model or report and dispatch to the appropriate
/// summary builder.
///
/// Private helper that avoids a second JSON parse when the caller already holds
/// the parsed value.
fn extract_entity_summaries_from_value(
    json: &serde_json::Value,
    file_path: &str,
) -> Vec<(String, String, String, String)> {
    let filename = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);

    // Detect file type: model.bim / report.json / other.
    let is_model = filename.eq_ignore_ascii_case("model.bim")
        || json.get("model").is_some()
        || (json.get("tables").is_some() && json.get("reportSections").is_none());

    let is_report = !is_model
        && (filename.eq_ignore_ascii_case("report.json")
            || json.get("reportSections").is_some()
            || json.get("displayName").is_some());

    if is_model {
        extract_model_summaries(json, file_path)
    } else if is_report {
        extract_report_summaries(json, file_path)
    } else {
        Vec::new()
    }
}

/// Build indexable entity summaries from a Power BI JSON-backed file's text content.
///
/// Detects the file type from the filename and JSON structure, then extracts
/// one tuple per indexable entity in the form
/// `(record_kind, object_name, parent_context, content_text)`.
///
/// Returns an empty `Vec` when the file content is not valid JSON or does not
/// contain recognisable Power BI structure.
#[must_use]
pub fn extract_entity_summaries(
    json_content: &str,
    file_path: &str,
) -> Vec<(String, String, String, String)> {
    let is_tmdl_path = Path::new(file_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("tmdl"));
    let is_tmdl_definition = last_path_component(file_path).eq_ignore_ascii_case("definition");
    if is_tmdl_path || is_tmdl_definition {
        return extract_tmdl_semantic_model(json_content, file_path)
            .map(|model| extract_model_summaries_from_model(&model))
            .unwrap_or_default();
    }

    let Ok(json) = serde_json::from_str::<serde_json::Value>(json_content) else {
        return Vec::new();
    };
    extract_entity_summaries_from_value(&json, file_path)
}

fn extract_model_summaries(
    json: &serde_json::Value,
    file_path: &str,
) -> Vec<(String, String, String, String)> {
    let Some(model) = extract_semantic_model(json, file_path) else {
        return Vec::new();
    };

    extract_model_summaries_from_model(&model)
}

fn extract_model_summaries_from_model(
    model: &crate::models::powerbi::PowerBiSemanticModel,
) -> Vec<(String, String, String, String)> {
    let mut summaries = Vec::new();

    for table in &model.tables {
        // One record per table.
        let col_hint = if table.columns.is_empty() {
            String::new()
        } else {
            let names: Vec<_> = table.columns.iter().map(|c| c.name.as_str()).collect();
            format!(" Columns: {}.", names.join(", "))
        };
        let meas_hint = if table.measures.is_empty() {
            String::new()
        } else {
            let names: Vec<_> = table.measures.iter().map(|m| m.name.as_str()).collect();
            format!(" Measures: {}.", names.join(", "))
        };
        summaries.push((
            "powerbi_table".to_string(),
            table.name.clone(),
            model.name.clone(),
            format!("Table {}{}{}", table.name, col_hint, meas_hint),
        ));

        // One record per measure.
        for measure in &table.measures {
            let expr_hint = measure
                .expression
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();
            summaries.push((
                "powerbi_measure".to_string(),
                measure.name.clone(),
                format!("{}.{}", model.name, table.name),
                format!(
                    "Measure {} in table {}. Expression: {}",
                    measure.name, table.name, expr_hint
                ),
            ));
        }
    }

    summaries
}

fn extract_report_summaries(
    json: &serde_json::Value,
    file_path: &str,
) -> Vec<(String, String, String, String)> {
    let Some(report) = extract_report(json, file_path) else {
        return Vec::new();
    };

    let mut summaries = Vec::new();

    for page in &report.pages {
        let visual_hint = if page.visuals.is_empty() {
            "No visuals.".to_string()
        } else {
            let types: Vec<_> = page
                .visuals
                .iter()
                .map(|v| v.visual_type.as_str())
                .collect();
            format!("Visuals: {}.", types.join(", "))
        };
        summaries.push((
            "powerbi_page".to_string(),
            page.name.clone(),
            report.name.clone(),
            format!(
                "Page {} in report {}. {}",
                page.name, report.name, visual_hint
            ),
        ));

        for visual in &page.visuals {
            summaries.push((
                "powerbi_visual".to_string(),
                visual.name.clone(),
                format!("{}/{}", report.name, page.name),
                format!(
                    "Visual {} of type {} on page {} in report {}.",
                    visual.name, visual.visual_type, page.name, report.name
                ),
            ));
        }
    }

    summaries
}

// ── Graph data builders ───────────────────────────────────────────────────

/// Derive a stable, content-addressable ID for a Power BI graph node.
///
/// The ID is a hex-encoded SHA-256 hash of the concatenation of `source_path`,
/// `file_path`, `kind_str`, and `unique_name`, prefixed with `pbi_`.
fn make_node_id(
    source_path: &str,
    file_path: &str,
    kind: PowerBiNodeKind,
    unique_name: &str,
) -> String {
    let seed = format!(
        "{source_path}:{file_path}:{k}:{unique_name}",
        k = kind.as_str()
    );
    format!("pbi_{}", compute_hash(seed.as_bytes()))
}

/// Build Power BI graph nodes and edges from a parsed JSON file.
///
/// Detects whether the JSON describes a semantic model (`model.bim`) or a
/// report, then constructs one node per entity and one `pbi_contains` edge
/// per parent → child relationship.
///
/// Returns `(nodes, edges)`. Both `Vec`s are empty when the JSON does not
/// contain recognisable Power BI structure or the extract helpers return `None`.
fn build_powerbi_graph_data(
    json: &serde_json::Value,
    file_path: &str,
    source_path: &str,
    content_hash: &str,
) -> (Vec<PowerBiNode>, Vec<PowerBiEdge>) {
    let mut nodes: Vec<PowerBiNode> = Vec::new();
    let mut edges: Vec<PowerBiEdge> = Vec::new();
    let now = chrono::Utc::now();

    let filename = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);

    let is_model = filename.eq_ignore_ascii_case("model.bim")
        || json.get("model").is_some()
        || (json.get("tables").is_some() && json.get("reportSections").is_none());

    if is_model {
        let Some(model) = extract_semantic_model(json, file_path) else {
            return (nodes, edges);
        };
        return build_powerbi_graph_data_from_model(
            &model,
            file_path,
            file_path,
            source_path,
            content_hash,
        );
    }

    let Some(report) = extract_report(json, file_path) else {
        return (nodes, edges);
    };
    let report_id = make_node_id(
        source_path,
        file_path,
        PowerBiNodeKind::Report,
        &report.name,
    );
    nodes.push(PowerBiNode {
        id: report_id.clone(),
        name: report.name.clone(),
        kind: PowerBiNodeKind::Report,
        file_path: file_path.to_owned(),
        source_path: source_path.to_owned(),
        content_hash: content_hash.to_owned(),
        ingested_at: now,
    });

    for page in &report.pages {
        let page_id = make_node_id(
            source_path,
            file_path,
            PowerBiNodeKind::Page,
            &format!("{}/{}", report.name, page.name),
        );
        nodes.push(PowerBiNode {
            id: page_id.clone(),
            name: page.name.clone(),
            kind: PowerBiNodeKind::Page,
            file_path: file_path.to_owned(),
            source_path: source_path.to_owned(),
            content_hash: content_hash.to_owned(),
            ingested_at: now,
        });
        edges.push(PowerBiEdge {
            from_id: report_id.clone(),
            to_id: page_id.clone(),
            edge_type: PowerBiEdgeType::Contains,
            source_path: source_path.to_owned(),
        });

        for visual in &page.visuals {
            let v_id = make_node_id(
                source_path,
                file_path,
                PowerBiNodeKind::Visual,
                &format!("{}/{}/{}", report.name, page.name, visual.name),
            );
            nodes.push(PowerBiNode {
                id: v_id.clone(),
                name: visual.name.clone(),
                kind: PowerBiNodeKind::Visual,
                file_path: file_path.to_owned(),
                source_path: source_path.to_owned(),
                content_hash: content_hash.to_owned(),
                ingested_at: now,
            });
            edges.push(PowerBiEdge {
                from_id: page_id.clone(),
                to_id: v_id,
                edge_type: PowerBiEdgeType::Contains,
                source_path: source_path.to_owned(),
            });
        }
    }

    (nodes, edges)
}

fn build_powerbi_graph_data_from_model(
    model: &crate::models::powerbi::PowerBiSemanticModel,
    identity_scope: &str,
    file_path: &str,
    source_path: &str,
    content_hash: &str,
) -> (Vec<PowerBiNode>, Vec<PowerBiEdge>) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let now = chrono::Utc::now();

    let model_id = make_node_id(
        source_path,
        identity_scope,
        PowerBiNodeKind::SemanticModel,
        &model.id,
    );
    nodes.push(PowerBiNode {
        id: model_id.clone(),
        name: model.name.clone(),
        kind: PowerBiNodeKind::SemanticModel,
        file_path: file_path.to_owned(),
        source_path: source_path.to_owned(),
        content_hash: content_hash.to_owned(),
        ingested_at: now,
    });

    for table in &model.tables {
        let table_id = make_node_id(
            source_path,
            identity_scope,
            PowerBiNodeKind::Table,
            &table.name,
        );
        nodes.push(PowerBiNode {
            id: table_id.clone(),
            name: table.name.clone(),
            kind: PowerBiNodeKind::Table,
            file_path: file_path.to_owned(),
            source_path: source_path.to_owned(),
            content_hash: content_hash.to_owned(),
            ingested_at: now,
        });
        edges.push(PowerBiEdge {
            from_id: model_id.clone(),
            to_id: table_id.clone(),
            edge_type: PowerBiEdgeType::Contains,
            source_path: source_path.to_owned(),
        });

        for col in &table.columns {
            let col_id = make_node_id(
                source_path,
                identity_scope,
                PowerBiNodeKind::Column,
                &format!("{}.{}", table.name, col.name),
            );
            nodes.push(PowerBiNode {
                id: col_id.clone(),
                name: col.name.clone(),
                kind: PowerBiNodeKind::Column,
                file_path: file_path.to_owned(),
                source_path: source_path.to_owned(),
                content_hash: content_hash.to_owned(),
                ingested_at: now,
            });
            edges.push(PowerBiEdge {
                from_id: table_id.clone(),
                to_id: col_id,
                edge_type: PowerBiEdgeType::Contains,
                source_path: source_path.to_owned(),
            });
        }

        for measure in &table.measures {
            let measure_id = make_node_id(
                source_path,
                identity_scope,
                PowerBiNodeKind::Measure,
                &format!("{}.{}", table.name, measure.name),
            );
            nodes.push(PowerBiNode {
                id: measure_id.clone(),
                name: measure.name.clone(),
                kind: PowerBiNodeKind::Measure,
                file_path: file_path.to_owned(),
                source_path: source_path.to_owned(),
                content_hash: content_hash.to_owned(),
                ingested_at: now,
            });
            edges.push(PowerBiEdge {
                from_id: table_id.clone(),
                to_id: measure_id,
                edge_type: PowerBiEdgeType::Contains,
                source_path: source_path.to_owned(),
            });
        }
    }

    for rel in &model.relationships {
        let rel_name = format!(
            "{}.{}→{}.{}",
            rel.from_table, rel.from_column, rel.to_table, rel.to_column
        );
        let rel_id = make_node_id(
            source_path,
            identity_scope,
            PowerBiNodeKind::Relationship,
            &rel_name,
        );
        nodes.push(PowerBiNode {
            id: rel_id.clone(),
            name: rel_name,
            kind: PowerBiNodeKind::Relationship,
            file_path: file_path.to_owned(),
            source_path: source_path.to_owned(),
            content_hash: content_hash.to_owned(),
            ingested_at: now,
        });
        edges.push(PowerBiEdge {
            from_id: model_id.clone(),
            to_id: rel_id.clone(),
            edge_type: PowerBiEdgeType::Contains,
            source_path: source_path.to_owned(),
        });

        let from_table_id = make_node_id(
            source_path,
            identity_scope,
            PowerBiNodeKind::Table,
            &rel.from_table,
        );
        edges.push(PowerBiEdge {
            from_id: rel_id.clone(),
            to_id: from_table_id,
            edge_type: PowerBiEdgeType::RelatesToTable,
            source_path: source_path.to_owned(),
        });

        let to_table_id = make_node_id(
            source_path,
            identity_scope,
            PowerBiNodeKind::Table,
            &rel.to_table,
        );
        edges.push(PowerBiEdge {
            from_id: rel_id,
            to_id: to_table_id,
            edge_type: PowerBiEdgeType::RelatesToTable,
            source_path: source_path.to_owned(),
        });
    }

    for ds in &model.data_sources {
        let ds_id = make_node_id(
            source_path,
            identity_scope,
            PowerBiNodeKind::DataSource,
            &ds.name,
        );
        nodes.push(PowerBiNode {
            id: ds_id.clone(),
            name: ds.name.clone(),
            kind: PowerBiNodeKind::DataSource,
            file_path: file_path.to_owned(),
            source_path: source_path.to_owned(),
            content_hash: content_hash.to_owned(),
            ingested_at: now,
        });
        edges.push(PowerBiEdge {
            from_id: model_id.clone(),
            to_id: ds_id,
            edge_type: PowerBiEdgeType::Contains,
            source_path: source_path.to_owned(),
        });
    }

    (nodes, edges)
}

// ── Async indexer ─────────────────────────────────────────────────────────

/// Index all Power BI files from a single content source.
///
/// Walks the source directory, reads eligible JSON/BIM files, extracts
/// entity summaries, and upserts [`ContentRecord`] rows into the
/// `content_record` relation.  Files whose hash has not changed since the
/// last run are skipped.
///
/// # Errors
///
/// Returns `Err` when the underlying database query fails unrecoverably.
pub async fn index_powerbi_source(
    source: &ContentSource,
    workspace_root: &Path,
    queries: &CodeGraphQueries,
    max_file_size: u64,
) -> Result<PowerBiIndexResult, EngramError> {
    let mut result = PowerBiIndexResult::default();

    let source_dir = workspace_root.join(&source.path);
    if !source_dir.is_dir() {
        debug!(
            path = %source.path,
            "Power BI source directory does not exist — skipping"
        );
        return Ok(result);
    }

    let files = collect_powerbi_files(&source_dir);
    result.total_files = files.len();

    // Build a map of existing content hashes for change detection.
    let existing_hashes: HashMap<String, String> = queries
        .select_content_records(Some("powerbi"))
        .await?
        .into_iter()
        .filter(|record| record.source_path == source.path)
        .map(|record| (record.file_path, record.content_hash))
        .collect();

    for file_path in &files {
        let Ok(metadata) = file_path.metadata() else {
            continue;
        };

        if metadata.len() > max_file_size {
            debug!(
                path = %file_path.display(),
                "Power BI file exceeds max_file_size — skipping"
            );
            continue;
        }

        let Ok(content_bytes) = std::fs::read(file_path) else {
            continue;
        };

        // Skip binary files.
        let Ok(content_str) = std::str::from_utf8(&content_bytes) else {
            continue;
        };

        let hash = compute_file_hash(&content_bytes);

        let rel_path = file_path
            .strip_prefix(workspace_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");

        // Skip unchanged files.
        if existing_hashes.get(&rel_path).map(String::as_str) == Some(hash.as_str()) {
            result.unchanged += 1;
            continue;
        }

        let is_tmdl = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("tmdl"))
            .unwrap_or(false);

        if is_tmdl {
            let Some(model) = extract_tmdl_semantic_model(content_str, &rel_path) else {
                debug!(path = %rel_path, "no TMDL semantic model entities found — skipping file");
                continue;
            };

            if existing_hashes.contains_key(&rel_path) {
                queries
                    .delete_content_records_by_scope(&rel_path, "powerbi", &source.path)
                    .await?;
                queries.delete_powerbi_nodes_by_file_path(&rel_path).await?;
            }

            let summaries = extract_model_summaries_from_model(&model);
            let file_size = metadata.len();
            let now = Utc::now();

            for (object_kind, object_name, parent_context, content_text) in &summaries {
                let chunk_id = format!("{object_name}:{object_kind}");
                let identity_seed = content_record_identity_seed(
                    &rel_path,
                    "powerbi",
                    &source.path,
                    Some(&format!("{parent_context}:{chunk_id}")),
                );
                let record_id = format!("cr_{}", compute_hash(identity_seed.as_bytes()));

                let record = ContentRecord {
                    id: record_id,
                    content_type: "powerbi".to_string(),
                    file_path: rel_path.clone(),
                    content_hash: hash.clone(),
                    content: format!(
                        "Kind: {object_kind}. Name: {object_name}. \
                         Context: {parent_context}. {content_text}"
                    ),
                    embedding: None,
                    source_path: source.path.clone(),
                    file_size_bytes: file_size,
                    ingested_at: now,
                    record_kind: object_kind.clone(),
                    chunk_id: Some(chunk_id),
                    chunk_index: None,
                    heading_path: Vec::new(),
                    line_start: None,
                    line_end: None,
                    fallback_reason: None,
                    lint_summary: None,
                    suggestions: Vec::new(),
                };

                queries.upsert_content_record(&record).await?;
            }

            let identity_scope = canonical_tmdl_model_path(&rel_path);
            let (graph_nodes, graph_edges) = build_powerbi_graph_data_from_model(
                &model,
                &identity_scope,
                &rel_path,
                &source.path,
                &hash,
            );
            if !graph_nodes.is_empty() {
                queries.upsert_powerbi_nodes(&graph_nodes).await?;
            }
            if !graph_edges.is_empty() {
                queries.upsert_powerbi_edges(&graph_edges).await?;
            }

            result.ingested += 1;
            continue;
        }

        // Parse JSON once; both summary extraction and graph building consume it.
        let Ok(json) = serde_json::from_str::<serde_json::Value>(content_str) else {
            debug!(path = %rel_path, "not valid JSON — skipping file");
            continue;
        };

        // Delete stale records whenever the hash changed (even if the new content
        // produces no recognisable entities), so orphaned rows do not accumulate.
        if existing_hashes.contains_key(&rel_path) {
            queries
                .delete_content_records_by_scope(&rel_path, "powerbi", &source.path)
                .await?;
            queries.delete_powerbi_nodes_by_file_path(&rel_path).await?;
        }

        let summaries = extract_entity_summaries_from_value(&json, &rel_path);
        if summaries.is_empty() {
            debug!(path = %rel_path, "no Power BI entities found — skipping file");
            continue;
        }

        let file_size = metadata.len();
        let now = Utc::now();

        for (object_kind, object_name, parent_context, content_text) in &summaries {
            let chunk_id = format!("{object_name}:{object_kind}");
            let identity_seed = content_record_identity_seed(
                &rel_path,
                "powerbi",
                &source.path,
                Some(&format!("{parent_context}:{chunk_id}")),
            );
            let record_id = format!("cr_{}", compute_hash(identity_seed.as_bytes()));

            let record = ContentRecord {
                id: record_id,
                content_type: "powerbi".to_string(),
                file_path: rel_path.clone(),
                content_hash: hash.clone(),
                content: format!(
                    "Kind: {object_kind}. Name: {object_name}. \
                     Context: {parent_context}. {content_text}"
                ),
                embedding: None,
                source_path: source.path.clone(),
                file_size_bytes: file_size,
                ingested_at: now,
                record_kind: object_kind.clone(),
                chunk_id: Some(chunk_id),
                chunk_index: None,
                heading_path: Vec::new(),
                line_start: None,
                line_end: None,
                fallback_reason: None,
                lint_summary: None,
                suggestions: Vec::new(),
            };

            queries.upsert_content_record(&record).await?;
        }

        // Build and persist Power BI graph nodes and edges for this file.
        let (graph_nodes, graph_edges) =
            build_powerbi_graph_data(&json, &rel_path, &source.path, &hash);
        if !graph_nodes.is_empty() {
            queries.upsert_powerbi_nodes(&graph_nodes).await?;
            debug!(
                path = %rel_path,
                nodes = graph_nodes.len(),
                edges = graph_edges.len(),
                "Power BI graph nodes upserted"
            );
        }
        if !graph_edges.is_empty() {
            queries.upsert_powerbi_edges(&graph_edges).await?;
        }

        result.ingested += 1;
    }

    info!(
        ingested = result.ingested,
        unchanged = result.unchanged,
        total = result.total_files,
        source = %source.path,
        "Power BI indexing complete"
    );

    Ok(result)
}

/// Remove content records for Power BI files that no longer exist on disk.
///
/// Queries all existing `content_type = "powerbi"` records for the source,
/// checks each file against the filesystem, and deletes records for absent
/// files.
///
/// # Errors
///
/// Returns `Err` when the database query fails unrecoverably.
pub async fn sweep_deleted_powerbi_files(
    source: &ContentSource,
    workspace_root: &Path,
    queries: &CodeGraphQueries,
) -> Result<usize, EngramError> {
    let records = queries.select_content_records(Some("powerbi")).await?;

    // Collect unique workspace-relative file paths for this source.
    let known_paths: Vec<String> = records
        .into_iter()
        .filter(|r| r.source_path == source.path)
        .map(|r| r.file_path)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let deleted = compute_deleted_paths(&known_paths, workspace_root);
    let mut removed = 0_usize;

    for path in &deleted {
        queries
            .delete_content_records_by_scope(path, "powerbi", &source.path)
            .await?;
        queries.delete_powerbi_nodes_by_file_path(path).await?;
        removed += 1;
    }

    Ok(removed)
}

// ── Unit tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture: model.bim JSON with one table, one relationship, and one
    /// data source.
    fn model_bim_with_rel_and_ds() -> serde_json::Value {
        serde_json::json!({
            "model": {
                "tables": [
                    {
                        "name": "Sales",
                        "columns": [{ "name": "ProductID", "dataType": "int64" }]
                    },
                    {
                        "name": "Products",
                        "columns": [{ "name": "ID", "dataType": "int64" }]
                    }
                ],
                "relationships": [
                    {
                        "fromTable": "Sales",
                        "fromColumn": "ProductID",
                        "toTable": "Products",
                        "toColumn": "ID"
                    }
                ],
                "dataSources": [
                    { "name": "SqlWarehouse", "type": "sql" }
                ]
            }
        })
    }

    /// S-PBI-01: `build_powerbi_graph_data` emits a `Relationship` node for each
    /// relationship declared in the semantic model.
    #[test]
    fn build_graph_emits_relationship_node() {
        let json = model_bim_with_rel_and_ds();
        let (nodes, _edges) =
            build_powerbi_graph_data(&json, "Sales.SemanticModel/model.bim", "models", "hash1");

        let rel_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n.kind == PowerBiNodeKind::Relationship)
            .collect();
        assert_eq!(
            rel_nodes.len(),
            1,
            "expected exactly one Relationship node; got {}: {rel_nodes:?}",
            rel_nodes.len()
        );
        assert!(
            rel_nodes[0].name.contains("Sales"),
            "relationship name should reference the from-table"
        );
        assert!(
            rel_nodes[0].name.contains("Products"),
            "relationship name should reference the to-table"
        );
    }

    /// S-PBI-02: `build_powerbi_graph_data` emits a `DataSource` node for each
    /// data source declared in the semantic model.
    #[test]
    fn build_graph_emits_data_source_node() {
        let json = model_bim_with_rel_and_ds();
        let (nodes, _edges) =
            build_powerbi_graph_data(&json, "Sales.SemanticModel/model.bim", "models", "hash1");

        let ds_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n.kind == PowerBiNodeKind::DataSource)
            .collect();
        assert_eq!(
            ds_nodes.len(),
            1,
            "expected exactly one DataSource node; got {}: {ds_nodes:?}",
            ds_nodes.len()
        );
        assert_eq!(
            ds_nodes[0].name, "SqlWarehouse",
            "DataSource node name should match the declared data source"
        );
    }

    /// S-PBI-03: `build_powerbi_graph_data` emits `pbi_relates_to_table` edges
    /// from the Relationship node to both endpoint tables.
    #[test]
    fn build_graph_emits_relates_to_table_edges() {
        let json = model_bim_with_rel_and_ds();
        let (nodes, edges) =
            build_powerbi_graph_data(&json, "Sales.SemanticModel/model.bim", "models", "hash1");

        let rel_node = nodes
            .iter()
            .find(|n| n.kind == PowerBiNodeKind::Relationship)
            .expect("Relationship node must be present");

        let rel_edges: Vec<_> = edges
            .iter()
            .filter(|e| e.edge_type == PowerBiEdgeType::RelatesToTable && e.from_id == rel_node.id)
            .collect();

        assert_eq!(
            rel_edges.len(),
            2,
            "expected two pbi_relates_to_table edges (one per endpoint table); got {}: {rel_edges:?}",
            rel_edges.len()
        );

        // Both endpoint table IDs should appear as edge targets.
        let sales_id = make_node_id(
            "models",
            "Sales.SemanticModel/model.bim",
            PowerBiNodeKind::Table,
            "Sales",
        );
        let products_id = make_node_id(
            "models",
            "Sales.SemanticModel/model.bim",
            PowerBiNodeKind::Table,
            "Products",
        );
        let target_ids: Vec<&str> = rel_edges.iter().map(|e| e.to_id.as_str()).collect();
        assert!(
            target_ids.contains(&sales_id.as_str()),
            "Sales table should be a pbi_relates_to_table target"
        );
        assert!(
            target_ids.contains(&products_id.as_str()),
            "Products table should be a pbi_relates_to_table target"
        );
    }

    /// S-PBI-04: `extract_entity_summaries` and `extract_entity_summaries_from_value`
    /// produce identical results for valid JSON (verifies the parse-once refactor).
    #[test]
    fn extract_entity_summaries_from_value_matches_string_variant() {
        let json_str = r#"{
            "model": {
                "tables": [
                    {"name": "Sales", "columns": [{"name": "ID", "dataType": "int64"}], "measures": []}
                ]
            }
        }"#;
        let json: serde_json::Value = serde_json::from_str(json_str).expect("valid fixture JSON");
        let from_str = extract_entity_summaries(json_str, "model.bim");
        let from_value = extract_entity_summaries_from_value(&json, "model.bim");
        assert_eq!(
            from_str, from_value,
            "both functions must produce identical summaries"
        );
        assert!(
            !from_str.is_empty(),
            "fixture should produce at least one summary"
        );
    }

    /// S-PBI-05: `make_node_id` encodes the `PowerBiNodeKind::as_str()` string in
    /// the hash seed, not an ad-hoc literal.
    #[test]
    fn make_node_id_encodes_canonical_kind_string() {
        for kind in [
            PowerBiNodeKind::Report,
            PowerBiNodeKind::Page,
            PowerBiNodeKind::Visual,
            PowerBiNodeKind::SemanticModel,
            PowerBiNodeKind::Table,
            PowerBiNodeKind::Column,
            PowerBiNodeKind::Measure,
            PowerBiNodeKind::Relationship,
            PowerBiNodeKind::DataSource,
        ] {
            let id = make_node_id("src", "file.json", kind, "Entity");
            let seed = format!("src:file.json:{}:Entity", kind.as_str());
            let expected = format!("pbi_{}", compute_hash(seed.as_bytes()));
            assert_eq!(
                id, expected,
                "make_node_id must use PowerBiNodeKind::as_str() for kind={kind:?}"
            );
        }
    }
}
