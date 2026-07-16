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

use std::collections::{HashMap, HashSet};
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
use crate::services::source_traversal::collect_files_in_workspace;
use powerbi_tmdl_parser::{DaxColumnRef, extract_dax_references};

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

/// Version namespace for `.tmdl` DAX graph indexing semantics.
///
/// Bump this when unchanged TMDL bytes must be reprocessed because the extracted
/// DAX graph shape changed. Monitoring: observe one-time Power BI re-index
/// duration and ingested/unchanged counts after upgrade. Rollback: revert the
/// bump and redeploy.
pub const TMDL_DAX_INDEX_VERSION: u32 = 1;

/// Compute the version-fingerprinted hash stored for `.tmdl` Power BI records.
///
/// Folding [`TMDL_DAX_INDEX_VERSION`] into the persisted hash gives the
/// incremental indexer a one-time migration path: a DAX-capable upgrade can
/// reprocess unchanged files without requiring `--force` or file edits.
#[must_use]
pub fn compute_tmdl_dax_index_hash(content: &[u8]) -> String {
    compute_tmdl_dax_index_hash_for_version(content, TMDL_DAX_INDEX_VERSION)
}

/// Compute a `.tmdl` DAX index hash for an explicit format `version`.
#[must_use]
pub fn compute_tmdl_dax_index_hash_for_version(content: &[u8], version: u32) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"tmdl-dax-index-version\0");
    hasher.update(version.to_be_bytes());
    hasher.update(b"\0content\0");
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
    let Ok(canonical_root) = workspace_root.canonicalize() else {
        return workspace_relative_paths.to_vec();
    };

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

            let candidate = workspace_root.join(relative_path);
            let is_deleted = candidate
                .canonicalize()
                .map_or(true, |canonical| !canonical.starts_with(&canonical_root));
            is_deleted.then(|| rel.clone())
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
    collect_powerbi_files_in_workspace(dir, dir)
}

/// Collect all indexable Power BI files under `dir`, resolving symlinked
/// directories only when their canonical target stays under `workspace_root`.
///
/// A visited set of canonical directory targets prevents symlink cycles from
/// recursing indefinitely while still allowing legitimate in-workspace symlinked
/// source directories to participate in indexing.
#[must_use]
pub fn collect_powerbi_files_in_workspace(dir: &Path, workspace_root: &Path) -> Vec<PathBuf> {
    collect_files_in_workspace(dir, workspace_root, is_powerbi_file)
}

fn is_powerbi_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case("json")
                || ext.eq_ignore_ascii_case("bim")
                || ext.eq_ignore_ascii_case("tmdl")
        })
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

/// Build a trailing hint fragment carrying a `lineageTag` and/or annotation
/// names for a table or measure summary. Returns an empty string when neither is
/// present so existing summary text is unchanged for models without the metadata.
fn annotation_lineage_hint(
    lineage_tag: Option<&str>,
    annotations: &[crate::models::powerbi::PowerBiAnnotation],
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(lineage) = lineage_tag {
        parts.push(format!("Lineage tag: {lineage}."));
    }
    if !annotations.is_empty() {
        let names: Vec<_> = annotations.iter().map(|a| a.name.as_str()).collect();
        parts.push(format!("Annotations: {}.", names.join(", ")));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" {}", parts.join(" "))
    }
}

pub(crate) fn extract_model_summaries_from_model(
    model: &crate::models::powerbi::PowerBiSemanticModel,
) -> Vec<(String, String, String, String)> {
    let mut summaries = Vec::new();
    let mut model_meta_parts: Vec<String> = Vec::new();
    if let Some(culture) = model.culture.as_deref() {
        model_meta_parts.push(format!("Culture: {culture}."));
    }
    if let Some(default_mode) = model.default_mode.as_deref() {
        model_meta_parts.push(format!("Default mode: {default_mode}."));
    }
    if let Some(lineage) = model.lineage_tag.as_deref() {
        model_meta_parts.push(format!("Lineage tag: {lineage}."));
    }
    if !model.refs.is_empty() {
        let refs: Vec<String> = model
            .refs
            .iter()
            .map(|reference| format!("{} {}", reference.kind, reference.name))
            .collect();
        model_meta_parts.push(format!("Refs: {}.", refs.join(", ")));
    }
    if !model.annotations.is_empty() {
        let names: Vec<_> = model.annotations.iter().map(|a| a.name.as_str()).collect();
        model_meta_parts.push(format!("Annotations: {}.", names.join(", ")));
    }
    let model_meta = if model_meta_parts.is_empty() {
        String::new()
    } else {
        format!(" {}", model_meta_parts.join(" "))
    };
    summaries.push((
        "powerbi_semantic_model".to_string(),
        model.name.clone(),
        model.path.clone(),
        format!(
            "Semantic model {}. Tables: {}. Relationships: {}. Expressions: {}. Data sources: {}.{}",
            model.name,
            model.tables.len(),
            model.relationships.len(),
            model.expressions.len(),
            model.data_sources.len(),
            model_meta
        ),
    ));

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
            format!(
                "Table {}{}{}{}",
                table.name,
                col_hint,
                meas_hint,
                annotation_lineage_hint(table.lineage_tag.as_deref(), &table.annotations)
            ),
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
                    "Measure {} in table {}. Expression: {}{}",
                    measure.name,
                    table.name,
                    expr_hint,
                    annotation_lineage_hint(measure.lineage_tag.as_deref(), &measure.annotations)
                ),
            ));
        }

        // One record per partition.
        for partition in &table.partitions {
            let kind_hint = partition
                .source_kind
                .as_deref()
                .map(|kind| format!(" Source kind: {kind}."))
                .unwrap_or_default();
            let mode_hint = partition
                .mode
                .as_deref()
                .map(|mode| format!(" Mode: {mode}."))
                .unwrap_or_default();
            let body_hint = partition
                .source_expression
                .as_deref()
                .map(|body| {
                    // Do NOT embed the raw M body in the searchable summary: M
                    // partition source can contain hard-coded secrets (tokens,
                    // keys, credentials). The full body stays in the structured
                    // `source_expression` model field; the summary carries only a
                    // non-sensitive size hint so the partition is still findable.
                    format!(" Source length: {} chars.", body.chars().count())
                })
                .unwrap_or_default();
            summaries.push((
                "powerbi_partition".to_string(),
                partition.name.clone(),
                format!("{}.{}", model.name, table.name),
                format!(
                    "Partition {} in table {}.{}{}{}",
                    partition.name, table.name, kind_hint, mode_hint, body_hint
                ),
            ));
        }
    }

    for expression in &model.expressions {
        let expr_hint = expression
            .expression
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(200)
            .collect::<String>();
        summaries.push((
            "powerbi_expression".to_string(),
            expression.name.clone(),
            model.name.clone(),
            format!(
                "Expression {} in semantic model {}. Definition: {}",
                expression.name, model.name, expr_hint
            ),
        ));
    }

    for data_source in &model.data_sources {
        let mut details: Vec<String> = Vec::new();
        if let Some(kind) = data_source
            .kind
            .as_deref()
            .or(data_source.source_type.as_deref())
        {
            details.push(format!("Kind: {kind}."));
        }
        if let Some(provider) = data_source.provider.as_deref() {
            details.push(format!("Provider: {provider}."));
        }
        if let Some(server) = data_source.server.as_deref() {
            details.push(format!("Server: {server}."));
        }
        if let Some(database) = data_source.database.as_deref() {
            details.push(format!("Database: {database}."));
        }
        if let Some(connection) = data_source.connection_string.as_deref() {
            // Connection strings frequently embed credentials (`Password=`,
            // `User ID=`, tokens, account keys). Keep the raw value in the
            // structured `connection_string` model field but emit only a
            // non-sensitive size hint into the searchable summary; the
            // non-secret server/database/provider context above stays searchable.
            details.push(format!(
                "Connection length: {} chars.",
                connection.chars().count()
            ));
        }
        let detail_hint = if details.is_empty() {
            String::new()
        } else {
            format!(" {}", details.join(" "))
        };
        summaries.push((
            "powerbi_data_source".to_string(),
            data_source.name.clone(),
            model.name.clone(),
            format!(
                "Data source {} in semantic model {}.{}",
                data_source.name, model.name, detail_hint
            ),
        ));
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
///
/// Exposed to `pub(crate)` so the dedicated PBIP indexer
/// ([`crate::services::pbip_indexer`]) can compute matching node IDs when
/// linking report and visual nodes to the shared semantic-model subgraph.
pub(crate) fn make_node_id(
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
            None,
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

pub(crate) fn build_powerbi_graph_data_from_model(
    model: &crate::models::powerbi::PowerBiSemanticModel,
    identity_scope: &str,
    file_path: &str,
    source_path: &str,
    content_hash: &str,
    reference_schema: Option<&ModelScopeSchema>,
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

        for partition in &table.partitions {
            let partition_id = make_node_id(
                source_path,
                identity_scope,
                PowerBiNodeKind::Partition,
                &format!("{}.{}", table.name, partition.name),
            );
            nodes.push(PowerBiNode {
                id: partition_id.clone(),
                name: partition.name.clone(),
                kind: PowerBiNodeKind::Partition,
                file_path: file_path.to_owned(),
                source_path: source_path.to_owned(),
                content_hash: content_hash.to_owned(),
                ingested_at: now,
            });
            edges.push(PowerBiEdge {
                from_id: table_id.clone(),
                to_id: partition_id,
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

    for expression in &model.expressions {
        let expression_id = make_node_id(
            source_path,
            identity_scope,
            PowerBiNodeKind::Expression,
            &expression.name,
        );
        nodes.push(PowerBiNode {
            id: expression_id.clone(),
            name: expression.name.clone(),
            kind: PowerBiNodeKind::Expression,
            file_path: file_path.to_owned(),
            source_path: source_path.to_owned(),
            content_hash: content_hash.to_owned(),
            ingested_at: now,
        });
        edges.push(PowerBiEdge {
            from_id: model_id.clone(),
            to_id: expression_id,
            edge_type: PowerBiEdgeType::Contains,
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

    // Emit `pbi_uses_field` reference edges from measure / calculated-column DAX
    // (P3). Resolution uses the model-scope-aggregated schema when supplied
    // (TMDL, whose files are parsed independently); otherwise the single model
    // is its own scope (`model.bim`, or a pre-merged PBIP model).
    let schema_fallback;
    let schema = if let Some(schema) = reference_schema {
        schema
    } else {
        schema_fallback = ModelScopeSchema::from_model(model);
        &schema_fallback
    };
    append_dax_reference_edges(model, schema, identity_scope, source_path, &mut edges);

    (nodes, edges)
}

/// Aggregated table / column / measure schema for one Power BI model scope.
///
/// TMDL ingestion parses each `.tmdl` file independently, so DAX reference
/// resolution must union all sibling files of one model (keyed by
/// `canonical_tmdl_model_path`) before resolving — otherwise a cross-table
/// reference such as a `Sales.tmdl` measure using `'Date'[Date]` would be
/// silently dropped. For a single-file model (`model.bim`) or a pre-merged PBIP
/// model, the model is its own scope.
#[derive(Debug, Default, Clone)]
pub(crate) struct ModelScopeSchema {
    /// Case-folded table name → the table's declared casing and columns. DAX
    /// identifiers are case-insensitive, so lookups fold to lowercase while edge
    /// construction recovers the declared casing to match the graph's
    /// `powerbi_node` ids (which are built from declared names).
    tables: HashMap<String, ScopeTable>,
    /// Case-folded measure name → its declared casing and owning table (measures
    /// are unique by name within a model).
    measures_by_name: HashMap<String, ScopeMeasureEntry>,
    /// Case-folded `(table, measure)` → declared-case `(table, measure)` for
    /// qualified-measure resolution.
    table_measures: HashMap<(String, String), (String, String)>,
}

/// One table's declared-case name and its case-folded column lookup.
#[derive(Debug, Default, Clone)]
struct ScopeTable {
    /// Declared-case table name (as first seen in the model scope).
    canonical: String,
    /// Case-folded column name → declared-case column name.
    columns: HashMap<String, String>,
}

/// A measure's declared-case name and its declared-case owning table.
#[derive(Debug, Clone)]
struct ScopeMeasureEntry {
    /// Declared-case measure name (as first seen in the model scope).
    canonical: String,
    /// Declared-case owning table name.
    owner_table: String,
}

impl ModelScopeSchema {
    /// Fold one parsed model's tables / columns / measures into the scope schema.
    ///
    /// Lookup keys are lowercased because DAX table/column/measure identifiers are
    /// case-insensitive; the first-seen declared casing is retained so resolved
    /// edges reference the same node ids the indexer emits.
    pub(crate) fn add_model(&mut self, model: &crate::models::powerbi::PowerBiSemanticModel) {
        for table in &model.tables {
            let table_key = table.name.to_lowercase();
            let entry = self
                .tables
                .entry(table_key.clone())
                .or_insert_with(|| ScopeTable {
                    canonical: table.name.clone(),
                    columns: HashMap::new(),
                });
            for column in &table.columns {
                entry
                    .columns
                    .entry(column.name.to_lowercase())
                    .or_insert_with(|| column.name.clone());
            }
            for measure in &table.measures {
                let measure_key = measure.name.to_lowercase();
                self.measures_by_name
                    .entry(measure_key.clone())
                    .or_insert_with(|| ScopeMeasureEntry {
                        canonical: measure.name.clone(),
                        owner_table: table.name.clone(),
                    });
                self.table_measures
                    .entry((table_key.clone(), measure_key))
                    .or_insert_with(|| (table.name.clone(), measure.name.clone()));
            }
        }
    }

    /// Build a scope schema from a single model (single-file / pre-merged case).
    fn from_model(model: &crate::models::powerbi::PowerBiSemanticModel) -> Self {
        let mut schema = Self::default();
        schema.add_model(model);
        schema
    }

    /// Whether `table` declares a column named `column` (case-insensitive).
    pub(crate) fn has_column(&self, table: &str, column: &str) -> bool {
        self.resolve_column(table, column).is_some()
    }

    /// Resolve `Table[Column]` to its declared-case `(table, column)` pair,
    /// folding DAX's case-insensitive identifier matching. `None` when unknown.
    pub(crate) fn resolve_column(&self, table: &str, column: &str) -> Option<(&str, &str)> {
        let table_entry = self.tables.get(&table.to_lowercase())?;
        let column_name = table_entry.columns.get(&column.to_lowercase())?;
        Some((table_entry.canonical.as_str(), column_name.as_str()))
    }

    /// Return the declared-case owning table of `measure`, if any (case-insensitive).
    pub(crate) fn measure_owner(&self, measure: &str) -> Option<&str> {
        self.measures_by_name
            .get(&measure.to_lowercase())
            .map(|entry| entry.owner_table.as_str())
    }

    /// Resolve a bare `[Measure]` to its declared-case `(owner_table, measure)`
    /// pair (case-insensitive). `None` when the model scope has no such measure.
    pub(crate) fn resolve_measure(&self, measure: &str) -> Option<(&str, &str)> {
        self.measures_by_name
            .get(&measure.to_lowercase())
            .map(|entry| (entry.owner_table.as_str(), entry.canonical.as_str()))
    }

    /// Whether `table` declares a measure named `measure` (case-insensitive).
    pub(crate) fn has_table_measure(&self, table: &str, measure: &str) -> bool {
        self.resolve_table_measure(table, measure).is_some()
    }

    /// Resolve `Table[Measure]` to its declared-case `(table, measure)` pair
    /// (case-insensitive). `None` when the table declares no such measure.
    pub(crate) fn resolve_table_measure(&self, table: &str, measure: &str) -> Option<(&str, &str)> {
        self.table_measures
            .get(&(table.to_lowercase(), measure.to_lowercase()))
            .map(|(table_name, measure_name)| (table_name.as_str(), measure_name.as_str()))
    }

    /// Whether any table in the model scope declares a column named `column`
    /// (case-insensitive).
    ///
    /// Used by the Tier-2 DAX linter to distinguish an unqualified reference
    /// that resolves to a real column on *another* table (a broken reference
    /// that must be qualified) from one that resolves to nothing at all.
    pub(crate) fn column_exists_anywhere(&self, column: &str) -> bool {
        let needle = column.to_lowercase();
        self.tables
            .values()
            .any(|table| table.columns.contains_key(&needle))
    }
}

/// Emit `pbi_uses_field` reference edges for every measure and calculated column
/// in `model`, resolving each extracted DAX reference against `schema`.
fn append_dax_reference_edges(
    model: &crate::models::powerbi::PowerBiSemanticModel,
    schema: &ModelScopeSchema,
    identity_scope: &str,
    source_path: &str,
    edges: &mut Vec<PowerBiEdge>,
) {
    for table in &model.tables {
        for measure in &table.measures {
            if let Some(expression) = measure.expression.as_deref() {
                let source_id = make_node_id(
                    source_path,
                    identity_scope,
                    PowerBiNodeKind::Measure,
                    &format!("{}.{}", table.name, measure.name),
                );
                append_resolved_reference_edges(
                    expression,
                    &table.name,
                    &source_id,
                    schema,
                    identity_scope,
                    source_path,
                    edges,
                );
            }
        }
        for column in &table.columns {
            if let Some(expression) = column.expression.as_deref() {
                let source_id = make_node_id(
                    source_path,
                    identity_scope,
                    PowerBiNodeKind::Column,
                    &format!("{}.{}", table.name, column.name),
                );
                append_resolved_reference_edges(
                    expression,
                    &table.name,
                    &source_id,
                    schema,
                    identity_scope,
                    source_path,
                    edges,
                );
            }
        }
    }
}

/// Resolve each reference in `expression` against `schema` and append a
/// `pbi_uses_field` edge from `source_id` to each resolved node (deduplicated,
/// self-edges excluded). Unresolved references are dropped, never fabricated.
#[allow(clippy::too_many_arguments)]
fn append_resolved_reference_edges(
    expression: &str,
    current_table: &str,
    source_id: &str,
    schema: &ModelScopeSchema,
    identity_scope: &str,
    source_path: &str,
    edges: &mut Vec<PowerBiEdge>,
) {
    let references = extract_dax_references(expression);
    let mut seen: HashSet<String> = HashSet::new();
    for reference in &references.columns {
        let Some(target_id) = resolve_reference(
            reference,
            current_table,
            schema,
            identity_scope,
            source_path,
        ) else {
            continue;
        };
        if target_id == source_id {
            continue;
        }
        if seen.insert(target_id.clone()) {
            edges.push(PowerBiEdge {
                from_id: source_id.to_owned(),
                to_id: target_id,
                edge_type: PowerBiEdgeType::UsesField,
                source_path: source_path.to_owned(),
            });
        }
    }
}

/// Resolve a single DAX column / bracket reference to an existing `pbi_` node id,
/// or `None` when it does not match the model schema (recorded as unresolved).
///
/// A qualified `Table[Name]` resolves to a column, else a measure on that table.
/// A bare `[Name]` resolves to a measure first (measures are model-unique), else
/// a column on the referencing (`current_table`) — never guessed across tables.
fn resolve_reference(
    reference: &DaxColumnRef,
    current_table: &str,
    schema: &ModelScopeSchema,
    identity_scope: &str,
    source_path: &str,
) -> Option<String> {
    match reference.table.as_deref() {
        Some(table) => {
            if let Some((canonical_table, canonical_column)) =
                schema.resolve_column(table, &reference.column)
            {
                Some(make_node_id(
                    source_path,
                    identity_scope,
                    PowerBiNodeKind::Column,
                    &format!("{canonical_table}.{canonical_column}"),
                ))
            } else if let Some((canonical_table, canonical_measure)) =
                schema.resolve_table_measure(table, &reference.column)
            {
                Some(make_node_id(
                    source_path,
                    identity_scope,
                    PowerBiNodeKind::Measure,
                    &format!("{canonical_table}.{canonical_measure}"),
                ))
            } else {
                None
            }
        }
        None => {
            if let Some((owner_table, canonical_measure)) =
                schema.resolve_measure(&reference.column)
            {
                Some(make_node_id(
                    source_path,
                    identity_scope,
                    PowerBiNodeKind::Measure,
                    &format!("{owner_table}.{canonical_measure}"),
                ))
            } else if let Some((canonical_table, canonical_column)) =
                schema.resolve_column(current_table, &reference.column)
            {
                Some(make_node_id(
                    source_path,
                    identity_scope,
                    PowerBiNodeKind::Column,
                    &format!("{canonical_table}.{canonical_column}"),
                ))
            } else {
                None
            }
        }
    }
}

/// Build one [`ModelScopeSchema`] per Power BI model scope by unioning the
/// tables / columns / measures of every `.tmdl` file, keyed by
/// `canonical_tmdl_model_path`. Non-TMDL, unreadable, and oversized
/// (`> max_file_size`) files are skipped so the schema only reflects files the
/// main indexing loop actually materialises.
fn build_model_scope_schemas(
    files: &[PathBuf],
    workspace_root: &Path,
    max_file_size: u64,
) -> HashMap<String, ModelScopeSchema> {
    let mut schemas: HashMap<String, ModelScopeSchema> = HashMap::new();
    for file_path in files {
        let is_tmdl = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tmdl"));
        if !is_tmdl {
            continue;
        }
        // Skip oversized files so an over-limit sibling never contributes columns
        // to resolution: the main indexing loop skips it too, so resolving a
        // reference against it would emit a `pbi_uses_field` edge to a node that
        // is never upserted (a dangling edge) while bypassing `max_file_size`.
        match file_path.metadata() {
            Ok(metadata) if metadata.len() > max_file_size => continue,
            Ok(_) => {}
            Err(_) => continue,
        }
        let Ok(content_bytes) = std::fs::read(file_path) else {
            continue;
        };
        let Ok(content_str) = std::str::from_utf8(&content_bytes) else {
            continue;
        };
        let rel_path = file_path
            .strip_prefix(workspace_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");
        let Some(model) = extract_tmdl_semantic_model(content_str, &rel_path) else {
            continue;
        };
        let scope = canonical_tmdl_model_path(&rel_path);
        schemas.entry(scope).or_default().add_model(&model);
    }
    schemas
}

/// Return `true` when `rel_path` names a `.tmdl` file (case-insensitive).
fn is_tmdl_rel_path(rel_path: &str) -> bool {
    std::path::Path::new(rel_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("tmdl"))
}

/// Determine which `canonical_tmdl_model_path` scopes changed on this
/// incremental pass.
///
/// A model scope is dirty when any of its `.tmdl` files is new, has a content
/// hash that differs from `existing_hashes` (changed), or was previously
/// indexed but is no longer on disk (deleted).
///
/// Model-scope invalidation (P3b, `085.008-T`): the incremental indexer skips
/// files whose bytes are unchanged, but a sibling's cross-file `pbi_uses_field`
/// reference edges can go stale when a peer file in the same model changes. When
/// a scope is dirty, every sibling `.tmdl` file in that scope must be
/// reprocessed so its references re-resolve against the current model-scope
/// schema — even if the sibling's own bytes did not change.
fn compute_dirty_model_scopes(
    files: &[PathBuf],
    workspace_root: &Path,
    existing_hashes: &HashMap<String, String>,
    max_file_size: u64,
) -> HashSet<String> {
    let mut dirty: HashSet<String> = HashSet::new();
    let mut seen_tmdl_rel_paths: HashSet<String> = HashSet::new();

    for file_path in files {
        if !file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tmdl"))
        {
            continue;
        }
        let Ok(metadata) = file_path.metadata() else {
            continue;
        };
        if metadata.len() > max_file_size {
            continue;
        }
        let Ok(content_bytes) = std::fs::read(file_path) else {
            continue;
        };
        if std::str::from_utf8(&content_bytes).is_err() {
            continue;
        }
        let rel_path = file_path
            .strip_prefix(workspace_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");
        seen_tmdl_rel_paths.insert(rel_path.clone());

        let hash = compute_tmdl_dax_index_hash(&content_bytes);
        let unchanged = existing_hashes.get(&rel_path).map(String::as_str) == Some(hash.as_str());
        if !unchanged {
            dirty.insert(canonical_tmdl_model_path(&rel_path));
        }
    }

    // A previously-indexed `.tmdl` file that has vanished from disk dirties its
    // scope so stale sibling reference edges get pruned.
    for prior_rel in existing_hashes.keys() {
        if is_tmdl_rel_path(prior_rel) && !seen_tmdl_rel_paths.contains(prior_rel) {
            dirty.insert(canonical_tmdl_model_path(prior_rel));
        }
    }

    dirty
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

    let files = collect_powerbi_files_in_workspace(&source_dir, workspace_root);
    result.total_files = files.len();

    // Pre-pass: union every `.tmdl` file into a per-model-scope schema so DAX
    // references that cross sibling files (e.g. a measure in `Sales.tmdl` using
    // `'Date'[Date]`) resolve against the whole model rather than the single
    // file currently being indexed (P3).
    let model_scope_schemas = build_model_scope_schemas(&files, workspace_root, max_file_size);

    // Build a map of existing content hashes for change detection.
    let existing_hashes: HashMap<String, String> = queries
        .select_content_records(Some("powerbi"))
        .await?
        .into_iter()
        .filter(|record| record.source_path == source.path)
        .map(|record| (record.file_path, record.content_hash))
        .collect();

    // P3b (`085.008-T`): model-scope invalidation. Determine which model scopes
    // changed on this pass so unchanged siblings are reprocessed and their
    // cross-file reference edges re-resolve against the current schema.
    let dirty_scopes =
        compute_dirty_model_scopes(&files, workspace_root, &existing_hashes, max_file_size);

    // Pre-delete every previously-indexed `.tmdl` artifact belonging to a dirty
    // scope BEFORE rebuilding any sibling (all-deletes-before-all-builds). Node
    // ids are stable across reindex, and `delete_powerbi_nodes_by_file_path`
    // cascades to every edge touching a deleted node. Deleting a sibling inside
    // the build loop could therefore collaterally remove a cross-file reference
    // edge that an earlier sibling just built; deleting all dirty-scope files up
    // front avoids that hazard and also prunes files that were deleted on disk.
    for prior_rel in existing_hashes.keys() {
        if is_tmdl_rel_path(prior_rel)
            && dirty_scopes.contains(&canonical_tmdl_model_path(prior_rel))
        {
            queries
                .delete_content_records_by_scope(prior_rel, "powerbi", &source.path)
                .await?;
            queries.delete_powerbi_nodes_by_file_path(prior_rel).await?;
        }
    }

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

        let rel_path = file_path
            .strip_prefix(workspace_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");

        // Skip unchanged files — unless the file is a `.tmdl` in a dirty model
        // scope, in which case it must be reprocessed so its cross-file
        // reference edges re-resolve against the updated schema (P3b).
        let is_tmdl = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("tmdl"))
            .unwrap_or(false);
        let hash = if is_tmdl {
            compute_tmdl_dax_index_hash(&content_bytes)
        } else {
            compute_file_hash(&content_bytes)
        };
        let unchanged = existing_hashes.get(&rel_path).map(String::as_str) == Some(hash.as_str());
        if unchanged && !(is_tmdl && dirty_scopes.contains(&canonical_tmdl_model_path(&rel_path))) {
            result.unchanged += 1;
            continue;
        }

        if is_tmdl {
            let Some(model) = extract_tmdl_semantic_model(content_str, &rel_path) else {
                debug!(path = %rel_path, "no TMDL semantic model entities found — skipping file");
                continue;
            };

            // Stale artifacts for dirty-scope `.tmdl` files were already removed
            // by the pre-delete pass above (all-deletes-before-all-builds), so
            // no per-file delete is needed here.
            let summaries = extract_model_summaries_from_model(&model);
            let file_size = metadata.len();
            let now = Utc::now();

            let mut tmdl_records = Vec::with_capacity(summaries.len());
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

                tmdl_records.push(record);
            }

            let identity_scope = canonical_tmdl_model_path(&rel_path);
            let (graph_nodes, graph_edges) = build_powerbi_graph_data_from_model(
                &model,
                &identity_scope,
                &rel_path,
                &source.path,
                &hash,
                model_scope_schemas.get(&identity_scope),
            );
            if !graph_nodes.is_empty() {
                queries.upsert_powerbi_nodes(&graph_nodes).await?;
            }
            if !graph_edges.is_empty() {
                queries.upsert_powerbi_edges(&graph_edges).await?;
            }
            for record in &tmdl_records {
                queries.upsert_content_record(record).await?;
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
            PowerBiNodeKind::Partition,
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

    /// S-PBI-06: `build_powerbi_graph_data_from_model` emits a `Partition` node
    /// for each partition and a `pbi_contains` edge from its table.
    #[test]
    fn build_graph_emits_partition_node_and_contains_edge() {
        let tmdl = "
table Sales
  column Amount
    dataType: double
  partition Sales = m
    mode: import
    source = ```
        let Source = 1 in Source
        ```
";
        let model = crate::services::powerbi_tmdl::extract_tmdl_semantic_model(
            tmdl,
            "models/Sales.SemanticModel/definition/tables/Sales.tmdl",
        )
        .expect("tmdl fixture should produce a semantic model");

        let (nodes, edges) = build_powerbi_graph_data_from_model(
            &model,
            "Sales.SemanticModel/definition",
            "Sales.SemanticModel/definition/tables/Sales.tmdl",
            "models",
            "hash1",
            None,
        );

        let partition_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n.kind == PowerBiNodeKind::Partition)
            .collect();
        assert_eq!(
            partition_nodes.len(),
            1,
            "expected exactly one Partition node; got {}: {partition_nodes:?}",
            partition_nodes.len()
        );
        assert_eq!(partition_nodes[0].name, "Sales");

        let table_id = make_node_id(
            "models",
            "Sales.SemanticModel/definition",
            PowerBiNodeKind::Table,
            "Sales",
        );
        let contains = edges.iter().any(|e| {
            e.edge_type == PowerBiEdgeType::Contains
                && e.from_id == table_id
                && e.to_id == partition_nodes[0].id
        });
        assert!(
            contains,
            "expected a pbi_contains edge from the table to its partition node"
        );
    }

    /// S-PBI-07: `extract_model_summaries_from_model` emits a `powerbi_data_source`
    /// summary record carrying the captured connection properties.
    #[test]
    fn extract_model_summaries_emits_data_source_record() {
        let tmdl = "
dataSource SqlWarehouse
  kind: sql
  provider: System.Data.SqlClient
  connectionString: Data Source=myserver;Initial Catalog=EDW
  server: myserver
  database: EDW
";
        let model = crate::services::powerbi_tmdl::extract_tmdl_semantic_model(
            tmdl,
            "models/Sales.SemanticModel/definition/dataSources.tmdl",
        )
        .expect("tmdl fixture should produce a semantic model");

        let summaries = extract_model_summaries_from_model(&model);
        let ds_summaries: Vec<_> = summaries
            .iter()
            .filter(|(kind, _, _, _)| kind == "powerbi_data_source")
            .collect();
        assert_eq!(
            ds_summaries.len(),
            1,
            "expected exactly one powerbi_data_source summary; got {}: {ds_summaries:?}",
            ds_summaries.len()
        );
        let (_, name, _, content) = ds_summaries[0];
        assert_eq!(name, "SqlWarehouse");
        assert!(
            content.contains("sql") && content.contains("myserver"),
            "data source summary should carry connection context: {content}"
        );
    }

    /// S-PBI-09: the data source summary must NOT embed the raw connection string
    /// (credential-exposure guard); it emits only a non-sensitive size hint while
    /// keeping non-secret server/database context searchable.
    #[test]
    fn extract_model_summaries_redacts_connection_string_secrets() {
        let tmdl = "
dataSource SecretWarehouse
  kind: sql
  server: myserver
  database: EDW
  connectionString: Data Source=myserver;Initial Catalog=EDW;User ID=admin;Password=sup3rs3cret
";
        let model = crate::services::powerbi_tmdl::extract_tmdl_semantic_model(
            tmdl,
            "models/Sales.SemanticModel/definition/dataSources.tmdl",
        )
        .expect("tmdl fixture should produce a semantic model");

        let summaries = extract_model_summaries_from_model(&model);
        let (_, _, _, content) = summaries
            .iter()
            .find(|(kind, _, _, _)| kind == "powerbi_data_source")
            .expect("expected a powerbi_data_source summary");

        assert!(
            !content.contains("sup3rs3cret") && !content.to_lowercase().contains("password"),
            "connection string secrets must not leak into the search summary: {content}"
        );
        // Non-secret context stays searchable.
        assert!(
            content.contains("myserver") && content.contains("EDW"),
            "non-secret server/database context should remain: {content}"
        );
    }

    /// S-PBI-11: ordinary recursive collection still returns supported file
    /// types and ignores unsupported extensions.
    #[test]
    fn collect_powerbi_files_keeps_non_symlink_behaviour() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(root.path().join("nested")).expect("create nested dir");
        std::fs::write(root.path().join("report.json"), "{}").expect("write json");
        std::fs::write(root.path().join("model.bim"), "{}").expect("write bim");
        std::fs::write(root.path().join("nested").join("table.tmdl"), "table T")
            .expect("write tmdl");
        std::fs::write(root.path().join("notes.txt"), "ignore").expect("write ignored file");

        let files = collect_powerbi_files_in_workspace(root.path(), root.path());
        let mut names: Vec<_> = files
            .iter()
            .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
            .collect();
        names.sort_unstable();

        assert_eq!(names, vec!["model.bim", "report.json", "table.tmdl"]);
    }

    /// S-PBI-12: a symlinked directory that points outside the workspace root is
    /// skipped before any files under it are collected.
    #[cfg(unix)]
    #[test]
    fn collect_powerbi_files_skips_symlinked_dir_escaping_workspace() {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let source = workspace.path().join("models");
        std::fs::create_dir_all(&source).expect("create source");
        std::fs::write(source.join("local.tmdl"), "table Local").expect("write local");

        let outside = tempfile::tempdir().expect("outside tempdir");
        std::fs::write(outside.path().join("evil.tmdl"), "table Evil").expect("write outside");
        std::os::unix::fs::symlink(outside.path(), source.join("escape"))
            .expect("create escaping symlink");

        let files = collect_powerbi_files_in_workspace(&source, workspace.path());
        let mut names: Vec<_> = files
            .iter()
            .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
            .collect();
        names.sort_unstable();

        assert_eq!(names, vec!["local.tmdl"]);
    }

    /// S-PBI-13: an in-workspace symlinked source directory is traversed, and a
    /// symlink cycle terminates because real directories are visited once.
    #[cfg(unix)]
    #[test]
    fn collect_powerbi_files_traverses_in_workspace_symlink_once_and_breaks_cycle() {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let source = workspace.path().join("models");
        let shared = workspace.path().join("shared");
        std::fs::create_dir_all(&source).expect("create source");
        std::fs::create_dir_all(&shared).expect("create shared");
        std::fs::write(shared.join("shared.tmdl"), "table Shared").expect("write shared");
        std::fs::write(source.join("local.tmdl"), "table Local").expect("write local");

        std::os::unix::fs::symlink(&shared, source.join("shared-link"))
            .expect("create in-workspace symlink");
        std::os::unix::fs::symlink(&source, shared.join("cycle")).expect("create symlink cycle");

        let files = collect_powerbi_files_in_workspace(&source, workspace.path());
        let mut names: Vec<_> = files
            .iter()
            .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
            .collect();
        names.sort_unstable();

        assert_eq!(names, vec!["local.tmdl", "shared.tmdl"]);
    }

    /// S-PBI-10: the partition summary must NOT embed the raw M source body
    /// (which can contain hard-coded secrets); it emits only a non-sensitive size
    /// hint while keeping the partition findable by name/table.
    #[test]
    fn extract_model_summaries_omits_partition_source_body() {
        let tmdl = "
table Secrets
  column A
    dataType: string

  partition SecretLoad = m
    mode: import
    source = ```
      let Source = Web.Contents(\"https://api.example.com\", [ApiKey=\"leaked-token-xyz\"])
      in Source
    ```
";
        let model = crate::services::powerbi_tmdl::extract_tmdl_semantic_model(
            tmdl,
            "models/Sales.SemanticModel/definition/tables/Secrets.tmdl",
        )
        .expect("tmdl fixture should produce a semantic model");

        let summaries = extract_model_summaries_from_model(&model);
        let (_, name, _, content) = summaries
            .iter()
            .find(|(kind, _, _, _)| kind == "powerbi_partition")
            .expect("expected a powerbi_partition summary");

        assert_eq!(name, "SecretLoad");
        assert!(
            !content.contains("leaked-token-xyz") && !content.contains("ApiKey"),
            "partition M-body secrets must not leak into the search summary: {content}"
        );
        // Partition remains findable by name/table.
        assert!(
            content.contains("SecretLoad") && content.contains("Secrets"),
            "partition summary should still reference name/table: {content}"
        );
    }

    /// S-PBI-08: `extract_model_summaries_from_model` folds `ref`/`annotation`/
    /// `lineageTag`/`culture` metadata into the existing parent summaries rather
    /// than emitting standalone records.
    #[test]
    fn extract_model_summaries_fold_refs_annotations_and_lineage() {
        let tmdl = "
model Sales Model
  culture: en-US
  lineageTag: model-guid-1
  annotation PBI_QueryOrder = [\"Sales\"]

  ref table Sales

table Sales
  lineageTag: table-guid-1

  measure 'Total' = SUM(Sales[Amount])
    lineageTag: measure-guid-1
    annotation DisplayFolder = KPIs
";
        let model = crate::services::powerbi_tmdl::extract_tmdl_semantic_model(
            tmdl,
            "models/Sales.SemanticModel/definition/model.tmdl",
        )
        .expect("tmdl fixture should produce a semantic model");

        let summaries = extract_model_summaries_from_model(&model);

        // No standalone annotation/ref/lineage record kinds are introduced.
        assert!(
            summaries
                .iter()
                .all(|(kind, _, _, _)| kind != "powerbi_annotation"
                    && kind != "powerbi_ref"
                    && kind != "powerbi_lineage_tag"),
            "task 003 metadata must fold into parent summaries, not new record kinds"
        );

        let model_summary = summaries
            .iter()
            .find(|(kind, _, _, _)| kind == "powerbi_semantic_model")
            .expect("a semantic model summary should exist");
        assert!(
            model_summary.3.contains("en-US"),
            "model summary should carry culture: {}",
            model_summary.3
        );
        assert!(
            model_summary.3.contains("model-guid-1"),
            "model summary should carry lineage tag: {}",
            model_summary.3
        );

        let table_summary = summaries
            .iter()
            .find(|(kind, name, _, _)| kind == "powerbi_table" && name == "Sales")
            .expect("a table summary should exist");
        assert!(
            table_summary.3.contains("table-guid-1"),
            "table summary should carry lineage tag: {}",
            table_summary.3
        );

        let measure_summary = summaries
            .iter()
            .find(|(kind, name, _, _)| kind == "powerbi_measure" && name == "Total")
            .expect("a measure summary should exist");
        assert!(
            measure_summary.3.contains("measure-guid-1"),
            "measure summary should carry lineage tag: {}",
            measure_summary.3
        );
        assert!(
            measure_summary.3.contains("DisplayFolder"),
            "measure summary should carry annotation context: {}",
            measure_summary.3
        );
    }
}
