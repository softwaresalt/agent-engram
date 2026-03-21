//! Hydration: load workspace state from `.engram/` files into SurrealDB.
//!
//! Reads version and flush metadata, collects file modification times, and
//! loads code-graph JSONL files. Task-specific parsing has been removed; this
//! module now serves pure code-intelligence hydration only.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use tracing::{info, warn};

use crate::errors::{EngramError, HydrationError};
use crate::models::registry::RegistryConfig;
use crate::services::dehydration::SCHEMA_VERSION;
use crate::services::registry::{load_registry, validate_sources};

#[derive(Debug, Clone, Default)]
pub struct HydrationSummary {
    pub last_flush: Option<String>,
    pub stale_files: bool,
    pub file_mtimes: HashMap<String, FileFingerprint>,
    /// Loaded content registry (None if no registry.yaml found).
    pub registry: Option<RegistryConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFingerprint {
    pub modified: SystemTime,
    pub len: u64,
}

/// Load workspace state summary from `.engram/` files (lightweight).
///
/// Creates the `.engram/` directory if missing. Does NOT load data into the
/// database; use [`hydrate_code_graph`] for that.
pub async fn hydrate_workspace(path: &Path) -> Result<HydrationSummary, EngramError> {
    let engram_dir = path.join(".engram");

    if !engram_dir.exists() {
        tokio::fs::create_dir_all(&engram_dir).await.map_err(|e| {
            EngramError::Hydration(HydrationError::Failed {
                reason: format!("failed to create .engram directory: {e}"),
            })
        })?;
        return Ok(HydrationSummary::default());
    }

    let file_mtimes = collect_file_mtimes(&engram_dir);
    let (last_flush, stale_files) = last_flush_state(&engram_dir, &file_mtimes);

    // Validate schema version if present
    let version_path = engram_dir.join(".version");
    if tokio::fs::try_exists(&version_path).await.map_err(|e| {
        EngramError::Hydration(HydrationError::Failed {
            reason: format!("failed to check .version existence: {e}"),
        })
    })? {
        let version = tokio::fs::read_to_string(&version_path)
            .await
            .map_err(|e| {
                EngramError::Hydration(HydrationError::Failed {
                    reason: format!("failed to read .version file: {e}"),
                })
            })?;
        let version = version.trim();
        if !version.is_empty() && version != SCHEMA_VERSION {
            return Err(EngramError::Hydration(HydrationError::SchemaMismatch {
                expected: SCHEMA_VERSION.to_string(),
                found: version.to_string(),
            }));
        }
    }

    // Load and validate content registry if present.
    let registry_path = engram_dir.join("registry.yaml");
    let registry = match load_registry(&registry_path) {
        Ok(Some(mut config)) => {
            match validate_sources(&mut config, path) {
                Ok(active) => {
                    info!(
                        active,
                        total = config.sources.len(),
                        "Registry loaded and validated"
                    );
                }
                Err(e) => {
                    warn!("Registry validation failed: {e}");
                }
            }
            Some(config)
        }
        Ok(None) => {
            info!("No registry.yaml found; using legacy hydration");
            None
        }
        Err(e) => {
            warn!("Failed to load registry.yaml: {e}");
            None
        }
    };

    Ok(HydrationSummary {
        last_flush,
        stale_files,
        file_mtimes,
        registry,
    })
}

/// Result of loading code graph JSONL files into the database.
#[derive(Debug, Clone, Default)]
pub struct CodeGraphHydrationResult {
    /// Total nodes loaded (code_files + functions + classes + interfaces).
    pub nodes_loaded: usize,
    /// Total edges loaded.
    pub edges_loaded: usize,
    /// Lines skipped due to parse errors (FR-135).
    pub lines_skipped: usize,
}

/// Hydrate code graph from `.engram/code-graph/` JSONL files (FR-132, FR-135).
///
/// Parses `nodes.jsonl` and `edges.jsonl`, upserting into SurrealDB.
/// Corrupt lines are logged and skipped (FR-135: graceful degradation).
pub async fn hydrate_code_graph(
    path: &Path,
    cg_queries: &crate::db::queries::CodeGraphQueries,
) -> Result<CodeGraphHydrationResult, EngramError> {
    let code_graph_dir = path.join(".engram").join("code-graph");
    let mut result = CodeGraphHydrationResult::default();

    // Parse nodes.jsonl
    let nodes_path = code_graph_dir.join("nodes.jsonl");
    if tokio::fs::try_exists(&nodes_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&nodes_path).await.map_err(|e| {
            EngramError::Hydration(HydrationError::Failed {
                reason: format!("failed to read nodes.jsonl: {e}"),
            })
        })?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(node) = parse_node_line(line) {
                if upsert_node(cg_queries, node, path).await {
                    result.nodes_loaded += 1;
                } else {
                    result.lines_skipped += 1;
                }
            } else {
                tracing::warn!(
                    line_preview = &line[..line.len().min(80)],
                    "skipping corrupt nodes.jsonl line (FR-135)"
                );
                result.lines_skipped += 1;
            }
        }
    }

    // Parse edges.jsonl
    let edges_path = code_graph_dir.join("edges.jsonl");
    if tokio::fs::try_exists(&edges_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&edges_path).await.map_err(|e| {
            EngramError::Hydration(HydrationError::Failed {
                reason: format!("failed to read edges.jsonl: {e}"),
            })
        })?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(edge) = parse_edge_line(line) {
                if upsert_edge(cg_queries, &edge).await {
                    result.edges_loaded += 1;
                } else {
                    result.lines_skipped += 1;
                }
            } else {
                tracing::warn!(
                    line_preview = &line[..line.len().min(80)],
                    "skipping corrupt edges.jsonl line (FR-135)"
                );
                result.lines_skipped += 1;
            }
        }
    }

    Ok(result)
}

/// Intermediate node representation parsed from a JSONL line.
#[derive(Debug, serde::Deserialize)]
struct ParsedNode {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    // code_file fields
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    size_bytes: Option<u64>,
    #[serde(default)]
    content_hash: Option<String>,
    #[serde(default)]
    last_indexed_at: Option<String>,
    // symbol fields
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    file_path: Option<String>,
    #[serde(default)]
    line_start: Option<u32>,
    #[serde(default)]
    line_end: Option<u32>,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    docstring: Option<String>,
    #[serde(default)]
    body_hash: Option<String>,
    #[serde(default)]
    token_count: Option<u32>,
    #[serde(default)]
    embed_type: Option<String>,
    #[serde(default)]
    embedding: Option<Vec<f32>>,
    #[serde(default)]
    summary: Option<String>,
}

/// Intermediate edge representation parsed from a JSONL line.
#[derive(Debug, serde::Deserialize)]
struct ParsedEdge {
    #[serde(rename = "type")]
    edge_type: String,
    from: String,
    to: String,
    #[serde(default)]
    import_path: Option<String>,
    #[serde(default)]
    linked_by: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    created_at: Option<String>,
}

fn parse_node_line(line: &str) -> Result<ParsedNode, serde_json::Error> {
    serde_json::from_str(line)
}

fn parse_edge_line(line: &str) -> Result<ParsedEdge, serde_json::Error> {
    serde_json::from_str(line)
}

/// Read the source body for a symbol from its file on disk.
///
/// Extracts lines `line_start..=line_end` (1-based, inclusive) from `file_path`
/// (workspace-relative). Returns an empty `String` if the file cannot be read or
/// the line range is out of bounds; logs a warning in both cases.
///
/// Body re-derivation is best-effort — a missing body is a degraded but non-fatal
/// condition; the node is still hydrated and searchable by name and hash.
async fn read_body_lines(
    workspace: &Path,
    file_path: &str,
    line_start: u32,
    line_end: u32,
) -> String {
    if file_path.is_empty() || line_start == 0 {
        return String::new();
    }
    let abs = workspace.join(file_path);
    let content = match tokio::fs::read_to_string(&abs).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                file = %abs.display(),
                error = %e,
                "hydration: cannot read source file for body re-derivation"
            );
            return String::new();
        }
    };
    let lines: Vec<&str> = content.lines().collect();
    // Convert 1-based inclusive range to 0-based half-open slice indices.
    let start = (line_start as usize).saturating_sub(1);
    let end = (line_end as usize).min(lines.len());
    if start >= lines.len() || start >= end {
        tracing::warn!(
            file = %abs.display(),
            line_start,
            line_end,
            total_lines = lines.len(),
            "hydration: line range out of bounds during body re-derivation"
        );
        return String::new();
    }
    lines[start..end].join("\n")
}

/// Upsert a parsed node into the database. Returns `true` on success.
async fn upsert_node(
    cg_queries: &crate::db::queries::CodeGraphQueries,
    node: ParsedNode,
    workspace: &Path,
) -> bool {
    use crate::models::{Class, CodeFile, Function, Interface};

    match node.node_type.as_str() {
        "code_file" => {
            let cf = CodeFile {
                id: node.id,
                path: node.path.unwrap_or_default(),
                language: node.language.unwrap_or_default(),
                size_bytes: node.size_bytes.unwrap_or(0),
                content_hash: node.content_hash.unwrap_or_default(),
                last_indexed_at: node.last_indexed_at.unwrap_or_default(),
            };
            cg_queries.upsert_code_file(&cf).await.is_ok()
        }
        "function" => {
            let file_path = node.file_path.unwrap_or_default();
            let line_start = node.line_start.unwrap_or(0);
            let line_end = node.line_end.unwrap_or(0);
            let body = read_body_lines(workspace, &file_path, line_start, line_end).await;
            let f = Function {
                id: node.id,
                name: node.name.unwrap_or_default(),
                file_path,
                line_start,
                line_end,
                signature: node.signature.unwrap_or_default(),
                docstring: node.docstring,
                body,
                body_hash: node.body_hash.unwrap_or_default(),
                token_count: node.token_count.unwrap_or(0),
                embed_type: node.embed_type.unwrap_or_default(),
                embedding: node.embedding.unwrap_or_default(),
                summary: node.summary.unwrap_or_default(),
            };
            cg_queries.upsert_function(&f).await.is_ok()
        }
        "class" => {
            let file_path = node.file_path.unwrap_or_default();
            let line_start = node.line_start.unwrap_or(0);
            let line_end = node.line_end.unwrap_or(0);
            let body = read_body_lines(workspace, &file_path, line_start, line_end).await;
            let c = Class {
                id: node.id,
                name: node.name.unwrap_or_default(),
                file_path,
                line_start,
                line_end,
                docstring: node.docstring,
                body,
                body_hash: node.body_hash.unwrap_or_default(),
                token_count: node.token_count.unwrap_or(0),
                embed_type: node.embed_type.unwrap_or_default(),
                embedding: node.embedding.unwrap_or_default(),
                summary: node.summary.unwrap_or_default(),
            };
            cg_queries.upsert_class(&c).await.is_ok()
        }
        "interface" => {
            let file_path = node.file_path.unwrap_or_default();
            let line_start = node.line_start.unwrap_or(0);
            let line_end = node.line_end.unwrap_or(0);
            let body = read_body_lines(workspace, &file_path, line_start, line_end).await;
            let i = Interface {
                id: node.id,
                name: node.name.unwrap_or_default(),
                file_path,
                line_start,
                line_end,
                docstring: node.docstring,
                body,
                body_hash: node.body_hash.unwrap_or_default(),
                token_count: node.token_count.unwrap_or(0),
                embed_type: node.embed_type.unwrap_or_default(),
                embedding: node.embedding.unwrap_or_default(),
                summary: node.summary.unwrap_or_default(),
            };
            cg_queries.upsert_interface(&i).await.is_ok()
        }
        _ => {
            tracing::warn!(
                node_type = node.node_type,
                "unknown node type in nodes.jsonl"
            );
            false
        }
    }
}

/// Upsert a parsed edge into the database. Returns `true` on success.
async fn upsert_edge(cg_queries: &crate::db::queries::CodeGraphQueries, edge: &ParsedEdge) -> bool {
    match edge.edge_type.as_str() {
        "calls" => cg_queries
            .create_calls_edge(&edge.from, &edge.to)
            .await
            .is_ok(),
        "imports" => {
            let import_path = edge.import_path.as_deref().unwrap_or("");
            cg_queries
                .create_imports_edge(&edge.from, &edge.to, import_path)
                .await
                .is_ok()
        }
        "defines" => {
            // create_defines_edge(file_id, symbol_table, symbol_id)
            let (to_table, to_id) = split_thing_id(&edge.to);
            cg_queries
                .create_defines_edge(&edge.from, to_table, to_id)
                .await
                .is_ok()
        }
        "inherits_from" => {
            // create_inherits_edge(child_table, child_id, parent_table, parent_id)
            let (from_table, from_id) = split_thing_id(&edge.from);
            let (to_table, to_id) = split_thing_id(&edge.to);
            cg_queries
                .create_inherits_edge(from_table, from_id, to_table, to_id)
                .await
                .is_ok()
        }
        "concerns" => {
            // create_concerns_edge(task_id, symbol_table, symbol_id, linked_by)
            let (to_table, to_id) = split_thing_id(&edge.to);
            let linked_by = edge.linked_by.as_deref().unwrap_or("hydration");
            cg_queries
                .create_concerns_edge(&edge.from, to_table, to_id, linked_by)
                .await
                .is_ok()
        }
        _ => {
            tracing::warn!(
                edge_type = edge.edge_type,
                "unknown edge type in edges.jsonl"
            );
            false
        }
    }
}

/// Split a `"table:id"` string into `("table", "id")`.
/// Falls back to `("unknown", full_id)` if no colon is found.
fn split_thing_id(id: &str) -> (&str, &str) {
    id.split_once(':').unwrap_or(("unknown", id))
}

/// Detect whether `.engram/` files have been modified since the last flush.
pub fn detect_stale(engram_dir: &Path) -> bool {
    let file_mtimes = collect_file_mtimes(engram_dir);
    let (_, stale) = last_flush_state(engram_dir, &file_mtimes);
    stale
}

/// Read the `.engram/.version` file contents.
pub fn read_version(engram_dir: &Path) -> Option<String> {
    let path = engram_dir.join(".version");
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read the `.engram/.lastflush` timestamp.
pub fn read_lastflush(engram_dir: &Path) -> Option<DateTime<Utc>> {
    let path = engram_dir.join(".lastflush");
    fs::read_to_string(path)
        .ok()
        .and_then(|s| DateTime::parse_from_rfc3339(s.trim()).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

/// Capture modification times for known `.engram/` files to support stale detection.
pub fn collect_file_mtimes(engram_dir: &Path) -> HashMap<String, FileFingerprint> {
    let mut mtimes = HashMap::new();

    let candidates = [
        ("tasks.md", engram_dir.join("tasks.md")),
        ("graph.surql", engram_dir.join("graph.surql")),
        (".version", engram_dir.join(".version")),
        (".lastflush", engram_dir.join(".lastflush")),
    ];

    for (name, path) in candidates {
        if let Ok(meta) = fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                mtimes.insert(
                    name.to_string(),
                    FileFingerprint {
                        modified,
                        len: meta.len(),
                    },
                );
            }
        }
    }

    mtimes
}

/// Determine if any tracked `.engram/` file has changed since the recorded mtimes.
pub fn detect_stale_since(recorded: &HashMap<String, FileFingerprint>, engram_dir: &Path) -> bool {
    if recorded.is_empty() {
        return false;
    }

    let current = collect_file_mtimes(engram_dir);

    // Changed or missing files compared to baseline
    for (name, recorded_time) in recorded {
        match current.get(name) {
            Some(current_time) => {
                if current_time.modified > recorded_time.modified
                    || current_time.len != recorded_time.len
                {
                    return true;
                }
            }
            None => return true, // file was removed
        }
    }

    // New files not in baseline
    for name in current.keys() {
        if !recorded.contains_key(name) {
            return true;
        }
    }

    false
}

// ─── Internal helpers ───────────────────────────────────────────────────────

pub fn last_flush_state(
    engram_dir: &Path,
    file_mtimes: &HashMap<String, FileFingerprint>,
) -> (Option<String>, bool) {
    let last_flush_path = engram_dir.join(".lastflush");

    let last_flush_str = fs::read_to_string(&last_flush_path).ok();
    let last_flush = last_flush_str
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s.trim()).ok())
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339());

    let mut stale_files = false;
    if let Some(ref flush) = last_flush {
        stale_files = file_mtimes
            .values()
            .any(|fingerprint| is_newer(&fingerprint.modified, flush));
    }

    (last_flush, stale_files)
}

fn is_newer(modified: &SystemTime, flush: &str) -> bool {
    if let Ok(flush_time) = DateTime::parse_from_rfc3339(flush) {
        if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
            let modified_time = DateTime::<Utc>::from(SystemTime::UNIX_EPOCH + duration);
            return modified_time > flush_time.with_timezone(&Utc);
        }
    }
    false
}

// ── SpecKit-aware rehydration (006-workspace-content-intelligence) ────────────

use crate::models::backlog::{BacklogArtifacts, BacklogFile, BacklogRef, ProjectManifest};

/// Scan `specs/` for SpecKit feature directories matching `NNN-feature-name`.
///
/// Returns a list of [`BacklogFile`] structs, one per feature directory found.
/// Directories not matching the `NNN-` prefix pattern are ignored (S039).
/// Returns an empty vec if `specs/` does not exist (S038 legacy fallback).
pub fn scan_speckit_features(workspace_root: &Path) -> Vec<BacklogFile> {
    let specs_dir = workspace_root.join("specs");
    if !specs_dir.is_dir() {
        return Vec::new();
    }

    let mut features = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(&specs_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let dir_name = entry.file_name().to_string_lossy().to_string();

        // Match NNN-feature-name pattern.
        let Some((num_str, rest)) = dir_name.split_once('-') else {
            continue;
        };
        if num_str.len() != 3 || !num_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let feature_dir = entry.path();
        let artifacts = read_speckit_artifacts(&feature_dir);

        // Extract title from spec.md first line if available.
        let title = artifacts
            .spec
            .as_deref()
            .and_then(|s| s.lines().next())
            .map(|l| l.trim_start_matches('#').trim().to_owned())
            .unwrap_or_else(|| rest.replace('-', " "));

        features.push(BacklogFile {
            id: num_str.to_owned(),
            name: rest.to_owned(),
            title,
            git_branch: dir_name.clone(),
            spec_path: format!("specs/{dir_name}"),
            description: String::new(),
            status: "draft".to_owned(),
            spec_status: "draft".to_owned(),
            artifacts,
            items: Vec::new(),
        });
    }

    features
}

/// Read all known SpecKit artifact files from a feature directory.
fn read_speckit_artifacts(feature_dir: &Path) -> BacklogArtifacts {
    let read = |filename: &str| -> Option<String> {
        let path = feature_dir.join(filename);
        std::fs::read_to_string(&path).ok()
    };

    BacklogArtifacts {
        spec: read("spec.md"),
        plan: read("plan.md"),
        tasks: read("tasks.md"),
        scenarios: read("SCENARIOS.md"),
        research: read("research.md"),
        analysis: read("ANALYSIS.md"),
        data_model: read("data-model.md"),
        quickstart: read("quickstart.md"),
    }
}

/// Public accessor for `read_speckit_artifacts` used by dehydration.
pub fn read_speckit_artifacts_pub(feature_dir: &Path) -> BacklogArtifacts {
    read_speckit_artifacts(feature_dir)
}

/// Read existing backlog JSON files from `.engram/` and return them.
///
/// Malformed JSON files are logged and skipped (S040).
pub fn read_backlog_files(engram_dir: &Path) -> Vec<BacklogFile> {
    let mut backlogs = Vec::new();

    if !engram_dir.is_dir() {
        return backlogs;
    }

    let mut entries: Vec<_> = std::fs::read_dir(engram_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("backlog-")
                && e.path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        })
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<BacklogFile>(&content) {
                Ok(backlog) => backlogs.push(backlog),
                Err(e) => {
                    warn!(
                        path = %path.display(),
                        "Malformed backlog JSON, skipping: {e}"
                    );
                }
            },
            Err(e) => {
                warn!(path = %path.display(), "Cannot read backlog file: {e}");
            }
        }
    }

    backlogs
}

/// Build a [`ProjectManifest`] from workspace metadata and backlog files.
pub fn build_project_manifest(workspace_root: &Path, backlogs: &[BacklogFile]) -> ProjectManifest {
    // Try to get project name from Cargo.toml or directory name.
    let name = workspace_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_owned());

    // Try to get git remote URL.
    let repository_url = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace_root)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_owned())
            } else {
                None
            }
        });

    // Try to get default branch.
    let default_branch = std::process::Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(workspace_root)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_owned())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "main".to_owned());

    let backlog_refs: Vec<BacklogRef> = backlogs
        .iter()
        .map(|b| BacklogRef {
            id: b.id.clone(),
            path: format!(".engram/backlog-{}.json", b.id),
        })
        .collect();

    ProjectManifest {
        name,
        description: String::new(),
        repository_url,
        default_branch,
        backlogs: backlog_refs,
    }
}

/// Check whether the workspace has SpecKit feature directories.
///
/// Returns `true` if `specs/` exists and contains at least one
/// directory matching the `NNN-feature-name` pattern.
pub fn has_speckit_features(workspace_root: &Path) -> bool {
    !scan_speckit_features(workspace_root).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_detection_no_lastflush() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tasks = dir.path().join("tasks.md");
        fs::write(&tasks, "# Tasks").expect("write");
        let mtimes = collect_file_mtimes(dir.path());
        let (flush, stale) = last_flush_state(dir.path(), &mtimes);
        assert!(flush.is_none());
        assert!(!stale, "no lastflush means not stale");
    }

    #[test]
    fn stale_detection_fresh() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tasks = dir.path().join("tasks.md");
        fs::write(&tasks, "# Tasks").expect("write tasks");
        // Write lastflush with a future timestamp so tasks.md appears older
        let future = Utc::now() + chrono::Duration::hours(1);
        fs::write(dir.path().join(".lastflush"), future.to_rfc3339()).expect("write flush");
        let mtimes = collect_file_mtimes(dir.path());
        let (flush, stale) = last_flush_state(dir.path(), &mtimes);
        assert!(flush.is_some());
        assert!(!stale, "tasks.md older than lastflush is not stale");
    }

    #[test]
    fn read_version_returns_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join(".version"), format!("{SCHEMA_VERSION}\n")).expect("write");
        assert_eq!(read_version(dir.path()), Some(SCHEMA_VERSION.to_string()));
    }

    #[test]
    fn read_version_missing_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(read_version(dir.path()), None);
    }

    // ── GAP-001: read_body_lines unit tests ─────────────────────────

    #[tokio::test]
    async fn read_body_lines_extracts_correct_lines() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let content = "line1\nline2\nline3\nline4\n";
        tokio::fs::write(tmp.path().join("src.rs"), content)
            .await
            .expect("write");
        let body = read_body_lines(tmp.path(), "src.rs", 2, 3).await;
        assert_eq!(body, "line2\nline3");
    }

    #[tokio::test]
    async fn read_body_lines_single_line() {
        let tmp = tempfile::tempdir().expect("tempdir");
        tokio::fs::write(tmp.path().join("f.rs"), "alpha\nbeta\ngamma\n")
            .await
            .expect("write");
        let body = read_body_lines(tmp.path(), "f.rs", 2, 2).await;
        assert_eq!(body, "beta");
    }

    #[tokio::test]
    async fn read_body_lines_missing_file_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let body = read_body_lines(tmp.path(), "nonexistent.rs", 1, 5).await;
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn read_body_lines_out_of_bounds_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        tokio::fs::write(tmp.path().join("f.rs"), "only one line\n")
            .await
            .expect("write");
        let body = read_body_lines(tmp.path(), "f.rs", 99, 100).await;
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn read_body_lines_empty_file_path_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let body = read_body_lines(tmp.path(), "", 1, 5).await;
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn read_body_lines_zero_line_start_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        tokio::fs::write(tmp.path().join("f.rs"), "content\n")
            .await
            .expect("write");
        let body = read_body_lines(tmp.path(), "f.rs", 0, 1).await;
        assert!(body.is_empty());
    }

    #[test]
    fn null_embedding_in_jsonl_deserializes_to_none() {
        // When embedding is absent from JSONL, ParsedNode.embedding is None.
        let line = r#"{"id":"function:x","type":"function","name":"x","file_path":"src/lib.rs","line_start":1,"line_end":2,"body_hash":"abc","token_count":0,"embed_type":"summary_pointer","summary":"fn x()"}"#;
        let node: ParsedNode = serde_json::from_str(line).expect("parse");
        // embedding absent (null) → unwrap_or_default in upsert_node → empty vec
        assert!(node.embedding.is_none());
    }
}
