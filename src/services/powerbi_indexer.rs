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
//! Each supported JSON file (report JSON, `model.bim`) is hashed on every
//! indexer run.  Files whose hash matches an existing record are skipped.
//! On each run a deletion sweep removes records for files that no longer
//! exist on disk.
//!
//! # Supported file types
//!
//! * `*.json` — report page descriptors and project manifests
//! * `*.bim` — tabular model definitions (`model.bim`)

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use crate::db::queries::CodeGraphQueries;
use crate::errors::EngramError;
use crate::models::content::ContentRecord;
use crate::models::powerbi::PowerBiIndexResult;
use crate::models::registry::ContentSource;
use crate::services::ingestion::{compute_hash, content_record_identity_seed};
use crate::services::powerbi_extract::{extract_report, extract_semantic_model};

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
/// `.json` or `.bim`.
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
                .map(|ext| ext.eq_ignore_ascii_case("json") || ext.eq_ignore_ascii_case("bim"))
                .unwrap_or(false);
            if is_target {
                files.push(path);
            }
        }
    }
}

// ── Entity summary extraction ─────────────────────────────────────────────

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
    let Ok(json) = serde_json::from_str::<serde_json::Value>(json_content) else {
        return Vec::new();
    };

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
        extract_model_summaries(&json, file_path)
    } else if is_report {
        extract_report_summaries(&json, file_path)
    } else {
        Vec::new()
    }
}

fn extract_model_summaries(
    json: &serde_json::Value,
    file_path: &str,
) -> Vec<(String, String, String, String)> {
    let Some(model) = extract_semantic_model(json, file_path) else {
        return Vec::new();
    };

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

        let summaries = extract_entity_summaries(content_str, &rel_path);
        if summaries.is_empty() {
            debug!(path = %rel_path, "no Power BI entities found — skipping file");
            continue;
        }

        // Delete stale records for this file before upserting fresh ones.
        queries
            .delete_content_records_by_scope(&rel_path, "powerbi", &source.path)
            .await?;

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
        removed += 1;
    }

    Ok(removed)
}
