//! Backlog content indexer for `.backlogit/` markdown files.
//!
//! Provides incremental, hash-based indexing of backlog artifacts into the
//! CozoDB graph as [`BacklogNode`], [`BacklogEdge`], and
//! [`BacklogContentRecord`] rows.  Used by the ingestion pipeline when a
//! registry source has `content_type == "backlog"`.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use crate::db::queries::CodeGraphQueries;
use crate::errors::EngramError;
use crate::models::backlog_graph::{
    BacklogContentRecord, BacklogEdge, BacklogEdgeType, BacklogIndexResult, BacklogNode,
};
use crate::models::registry::ContentSource;
use crate::services::parsing::frontmatter;
use crate::services::source_traversal::{collect_files_in_workspace, is_regular_file_in_workspace};

// ── Helpers ───────────────────────────────────────────────────────────────

/// Compute a hex-encoded SHA-256 hash of `content` bytes.
///
/// Used for incremental change detection: if the hash matches the value
/// stored in the DB, the file is skipped.
#[must_use]
pub fn compute_file_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Return the subset of `known_paths` whose files no longer exist under
/// `workspace_root`.
///
/// Each entry is validated as a workspace-relative candidate before any
/// filesystem probe: absolute paths, root-relative paths, `..` traversal, and
/// drive prefixes cannot name a live in-workspace file, so they are skipped
/// without touching the filesystem outside the workspace root.
///
/// Final-component symlinks are treated as deleted, matching the backlog
/// collector's file-symlink skip behavior. Files reached through an
/// in-workspace directory symlink are preserved when their canonical target
/// remains under `workspace_root`.
#[must_use]
pub fn compute_deleted_paths(known_paths: &[String], workspace_root: &Path) -> Vec<String> {
    let Ok(canonical_root) = workspace_root.canonicalize() else {
        return known_paths.to_vec();
    };

    known_paths
        .iter()
        .filter_map(|raw| {
            let Some(relative) = workspace_relative_path(raw) else {
                warn!(
                    path = %raw,
                    "skipping backlog deletion sweep path that escapes the workspace root"
                );
                return None;
            };
            let candidate = workspace_root.join(relative);
            let is_deleted = !is_regular_file_in_workspace(&candidate, &canonical_root);
            is_deleted.then(|| raw.clone())
        })
        .collect()
}

/// Reject paths that cannot name a live in-workspace file.
///
/// Mirrors the Power BI, PBIP, and notebook indexers: absolute paths,
/// root-relative paths (`\foo` / `/foo` on Windows), `..` traversal, and drive
/// prefixes are refused so the deletion sweep never probes outside the
/// workspace root.
fn workspace_relative_path(rel_path: &str) -> Option<PathBuf> {
    let path = Path::new(rel_path);
    if path.is_absolute()
        || path.has_root()
        || path.components().any(|component| {
            component == Component::ParentDir || matches!(component, Component::Prefix(_))
        })
    {
        return None;
    }

    Some(path.to_path_buf())
}

/// Collect all backlog markdown files under `dir` recursively, sorted by path.
#[must_use]
pub fn collect_backlog_files(dir: &Path) -> Vec<PathBuf> {
    collect_backlog_files_in_workspace(dir, dir)
}

/// Collect backlog markdown files under `dir`, traversing only symlinked
/// directories whose canonical target remains under `workspace_root`.
#[must_use]
pub fn collect_backlog_files_in_workspace(dir: &Path, workspace_root: &Path) -> Vec<PathBuf> {
    collect_files_in_workspace(dir, workspace_root, is_backlog_file)
}

fn is_backlog_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

// ── Core extraction ───────────────────────────────────────────────────────

/// Inner extraction logic shared by [`extract_backlog_data`] and
/// [`index_backlog_source`].
///
/// Accepts pre-read text and a pre-computed content hash so the caller can
/// avoid a second filesystem read when the hash check is already done.
///
/// Returns `None` when the file should be skipped (no frontmatter or no `id`).
fn extract_from_content(
    text: &str,
    content_hash: String,
    file_path: &Path,
    workspace_root: &Path,
    source_path: &str,
) -> Option<(BacklogNode, Vec<BacklogEdge>, BacklogContentRecord)> {
    let doc = frontmatter::parse(text);

    let meta = doc.metadata?;

    // Require non-empty `id` field.
    let id = match meta.get("id").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            debug!(
                path = %file_path.display(),
                "backlog file missing `id` field — skipped"
            );
            return None;
        }
    };

    // Optional fields with fallback defaults.
    let title = meta
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let kind = meta
        .get("artifact_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let status = meta
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let labels: Vec<String> = meta
        .get("labels")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|e| e.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    // Workspace-relative file path with forward slashes.
    let rel_path = file_path
        .strip_prefix(workspace_root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .replace('\\', "/");

    let node = BacklogNode {
        id: id.clone(),
        title: title.clone(),
        kind,
        status,
        labels,
        file_path: rel_path.clone(),
        content_hash: content_hash.clone(),
        source_path: source_path.to_string(),
        ingested_at: chrono::Utc::now(),
    };

    // ── Edges ────────────────────────────────────────────────────────────

    let mut edges: Vec<BacklogEdge> = Vec::new();

    // `parent_id` → parent_of edge (parent→child direction).
    if let Some(parent_id) = meta.get("parent_id").and_then(|v| v.as_str()) {
        if !parent_id.is_empty() {
            edges.push(BacklogEdge {
                from_id: parent_id.to_string(),
                to_id: id.clone(),
                edge_type: BacklogEdgeType::ParentOf,
                source_path: source_path.to_string(),
            });
        }
    }

    // `dependencies` → depends_on edges (artifact depends on each dep).
    if let Some(deps) = meta.get("dependencies").and_then(|v| v.as_sequence()) {
        for dep in deps {
            if let Some(dep_id) = dep.as_str() {
                if !dep_id.is_empty() {
                    edges.push(BacklogEdge {
                        from_id: id.clone(),
                        to_id: dep_id.to_string(),
                        edge_type: BacklogEdgeType::DependsOn,
                        source_path: source_path.to_string(),
                    });
                }
            }
        }
    }

    // ── Content record ───────────────────────────────────────────────────

    // Body text enriched with title for searchability.
    let content = if title.is_empty() {
        doc.body.clone()
    } else {
        format!("{title}\n\n{}", doc.body)
    };

    let record = BacklogContentRecord {
        file_path: rel_path,
        content_type: "backlog".to_string(),
        content_hash,
        content,
        source_path: source_path.to_string(),
        ingested_at: chrono::Utc::now(),
    };

    Some((node, edges, record))
}

/// Extract graph data from a single backlog markdown file.
///
/// Returns `None` (without error) when the file should be skipped:
/// - No YAML frontmatter was found.
/// - The frontmatter does not contain a non-empty `id` field.
///
/// On success returns `Some((node, edges, record))`:
/// - `node` — a [`BacklogNode`] for this artifact.
/// - `edges` — zero or more [`BacklogEdge`] entries inferred from
///   `parent_id` and `dependencies` frontmatter fields.
/// - `record` — a [`BacklogContentRecord`] containing the full file body.
///
/// # Errors
///
/// Returns an error only for I/O failures (unreadable file).
pub fn extract_backlog_data(
    file_path: &Path,
    workspace_root: &Path,
    source_path: &str,
) -> Result<Option<(BacklogNode, Vec<BacklogEdge>, BacklogContentRecord)>, EngramError> {
    let raw = std::fs::read(file_path).map_err(|e| crate::errors::IngestionError::Failed {
        path: file_path.display().to_string(),
        reason: e.to_string(),
    })?;
    let content_hash = compute_file_hash(&raw);
    let text = String::from_utf8_lossy(&raw);
    Ok(extract_from_content(
        &text,
        content_hash,
        file_path,
        workspace_root,
        source_path,
    ))
}

// ── Source-level indexing ─────────────────────────────────────────────────

/// Index all markdown files under a backlog content source.
///
/// Walks the source directory for `*.md` files, extracts graph data from
/// each, compares content hashes against existing DB records, and upserts
/// only changed files.  Returns a [`BacklogIndexResult`] with counts.
///
/// Files exceeding `max_file_size_bytes` are skipped (same limit as
/// the generic ingestion pipeline).
///
/// # Errors
///
/// Returns errors for DB write failures.  Per-file read or parse errors are
/// logged as warnings and result in the file being skipped (not a hard error).
pub async fn index_backlog_source(
    source: &ContentSource,
    workspace_root: &Path,
    queries: &CodeGraphQueries,
    max_file_size_bytes: u64,
) -> Result<BacklogIndexResult, EngramError> {
    let source_dir = workspace_root.join(&source.path);
    let source_path = &source.path;

    // Load existing nodes for this source to build a hash-map for change detection.
    let existing_nodes = queries.select_backlog_nodes(Some(source_path)).await?;
    let hash_by_path: HashMap<String, String> = existing_nodes
        .iter()
        .map(|n| (n.file_path.clone(), n.content_hash.clone()))
        .collect();

    let mut result = BacklogIndexResult::default();

    if !source_dir.exists() {
        warn!(
            path = %source_dir.display(),
            "backlog source directory does not exist — skipping"
        );
        return Ok(result);
    }

    let files = collect_backlog_files_in_workspace(&source_dir, workspace_root);
    result.total_files = files.len();

    for file_path in &files {
        // Skip files that exceed the configured size limit.
        if let Ok(meta) = std::fs::metadata(file_path) {
            if meta.len() > max_file_size_bytes {
                warn!(
                    path = %file_path.display(),
                    size = meta.len(),
                    limit = max_file_size_bytes,
                    "backlog file exceeds size limit — skipped"
                );
                continue;
            }
        }

        // Quick hash check: read raw bytes first.
        let raw = match std::fs::read(file_path) {
            Ok(b) => b,
            Err(e) => {
                warn!(path = %file_path.display(), error = %e, "failed to read backlog file");
                continue;
            }
        };
        let new_hash = compute_file_hash(&raw);

        // Derive relative path for the hash map lookup.
        let rel_path = file_path
            .strip_prefix(workspace_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");

        if hash_by_path.get(&rel_path) == Some(&new_hash) {
            result.unchanged += 1;
            continue;
        }

        let text = String::from_utf8_lossy(&raw);
        if let Some((node, edges, record)) = extract_from_content(
            &text,
            new_hash.clone(),
            file_path,
            workspace_root,
            source_path,
        ) {
            queries
                .upsert_backlog_nodes(std::slice::from_ref(&node))
                .await?;
            if !edges.is_empty() {
                queries.upsert_backlog_edges(&edges).await?;
            }
            queries
                .upsert_backlog_content_records(std::slice::from_ref(&record))
                .await?;

            result.ingested += 1;
        }
        // else: file skipped (no id / no frontmatter) — not counted.
    }

    info!(
        source = %source_path,
        ingested = result.ingested,
        unchanged = result.unchanged,
        total_files = result.total_files,
        "backlog source indexed"
    );

    Ok(result)
}

/// Remove backlog nodes (and their content records) for files that no longer
/// exist on disk.
///
/// Queries the DB for all nodes in `source`, derives absolute paths from
/// `file_path` fields, and deletes any entry whose file is missing.
///
/// Returns the number of files removed.
///
/// # Errors
///
/// Returns errors for DB read/write failures.
pub async fn sweep_deleted_backlog_files(
    source: &ContentSource,
    workspace_root: &Path,
    queries: &CodeGraphQueries,
) -> Result<usize, EngramError> {
    let existing = queries.select_backlog_nodes(Some(&source.path)).await?;

    let known_paths: Vec<String> = existing.iter().map(|n| n.file_path.clone()).collect();

    let deleted = compute_deleted_paths(&known_paths, workspace_root);
    let mut removed = 0_usize;

    for rel in &deleted {
        debug!(path = %rel, "sweeping deleted backlog file");
        queries.delete_backlog_node_by_file_path(rel).await?;
        queries.delete_backlog_content_record_by_path(rel).await?;
        removed += 1;
    }

    if removed > 0 {
        info!(removed, source = %source.path, "backlog deletion sweep complete");
    }

    Ok(removed)
}
