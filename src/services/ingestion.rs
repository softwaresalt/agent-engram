//! Content ingestion pipeline for multi-source workspace content.
//!
//! Walks registered content sources, reads files, computes content hashes,
//! and upserts [`ContentRecord`](crate::models::ContentRecord) entries into
//! SurrealDB. Supports incremental sync via content hash comparison and
//! respects configurable file size limits and batch sizes.

use std::collections::HashSet;
use std::path::Path;

use chrono::Utc;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use crate::db::queries::CodeGraphQueries;
use crate::errors::{EngramError, IngestionError};
use crate::models::content::ContentRecord;
use crate::models::registry::{ContentSourceStatus, RegistryConfig};

/// Result summary from an ingestion run.
#[derive(Debug, Clone, Default)]
pub struct IngestionSummary {
    /// Files successfully ingested or updated.
    pub ingested: usize,
    /// Files skipped because content hash was unchanged.
    pub unchanged: usize,
    /// Files skipped because they exceeded the size limit.
    pub oversized: usize,
    /// Files skipped because they appear to be binary.
    pub binary: usize,
    /// Files removed because they no longer exist on disk.
    pub removed: usize,
    /// Total files encountered during walk.
    pub total_files: usize,
}

/// Ingest content from all active sources in the registry.
///
/// For each source with [`ContentSourceStatus::Active`], walks the directory,
/// reads eligible files, computes SHA-256 hashes, and upserts content records.
/// Files exceeding `max_file_size_bytes` or detected as binary are skipped.
pub async fn ingest_all_sources(
    config: &RegistryConfig,
    workspace_root: &Path,
    queries: &CodeGraphQueries,
) -> Result<IngestionSummary, EngramError> {
    let mut total_summary = IngestionSummary::default();

    for source in &config.sources {
        if source.status != ContentSourceStatus::Active {
            continue;
        }

        // Skip code sources — they use the code graph indexer instead.
        if source.content_type == "code" {
            debug!(path = %source.path, "Skipping code source (uses code graph indexer)");
            continue;
        }

        let source_path = workspace_root.join(&source.path);
        let summary = ingest_directory(
            &source_path,
            workspace_root,
            &source.content_type,
            &source.path,
            config.max_file_size_bytes,
            config.batch_size,
            queries,
        )
        .await?;

        total_summary.ingested += summary.ingested;
        total_summary.unchanged += summary.unchanged;
        total_summary.oversized += summary.oversized;
        total_summary.binary += summary.binary;
        total_summary.removed += summary.removed;
        total_summary.total_files += summary.total_files;
    }

    info!(
        ingested = total_summary.ingested,
        unchanged = total_summary.unchanged,
        oversized = total_summary.oversized,
        binary = total_summary.binary,
        total = total_summary.total_files,
        "Ingestion complete"
    );

    Ok(total_summary)
}

/// Ingest all eligible files from a single directory.
async fn ingest_directory(
    dir_path: &Path,
    workspace_root: &Path,
    content_type: &str,
    source_path: &str,
    max_file_size: u64,
    batch_size: usize,
    queries: &CodeGraphQueries,
) -> Result<IngestionSummary, EngramError> {
    let mut summary = IngestionSummary::default();

    if !dir_path.is_dir() {
        return Ok(summary);
    }

    // Collect all files recursively.
    let files = collect_files(dir_path);
    summary.total_files = files.len();

    // Get existing records to detect changes.
    let existing: Vec<crate::models::ContentRecord> = queries.select_content_records(Some(content_type)).await?;
    let existing_by_path: std::collections::HashMap<String, String> = existing
        .iter()
        .map(|r| (r.file_path.clone(), r.content_hash.clone()))
        .collect();
    let mut seen_paths: HashSet<String> = HashSet::new();

    // Process in batches.
    for chunk in files.chunks(batch_size) {
        for file_path in chunk {
            let rel_path = file_path
                .strip_prefix(workspace_root)
                .unwrap_or(file_path)
                .to_string_lossy()
                .replace('\\', "/");

            seen_paths.insert(rel_path.clone());

            // Check file size.
            let metadata = match std::fs::metadata(file_path) {
                Ok(m) => m,
                Err(e) => {
                    warn!(path = %rel_path, "Cannot read file metadata: {e}");
                    continue;
                }
            };

            if metadata.len() > max_file_size {
                debug!(path = %rel_path, size = metadata.len(), "Skipping oversized file");
                summary.oversized += 1;
                continue;
            }

            // Read file content.
            let content = match std::fs::read(file_path) {
                Ok(bytes) => bytes,
                Err(e) => {
                    warn!(path = %rel_path, "Cannot read file: {e}");
                    continue;
                }
            };

            // Skip binary files (simple heuristic: contains null bytes in first 8KB).
            if is_binary(&content) {
                debug!(path = %rel_path, "Skipping binary file");
                summary.binary += 1;
                continue;
            }

            let content_str = String::from_utf8_lossy(&content).to_string();
            let content_hash = compute_hash(&content);

            // Check if content has changed.
            if let Some(existing_hash) = existing_by_path.get(&rel_path) {
                if *existing_hash == content_hash {
                    summary.unchanged += 1;
                    continue;
                }
            }

            // Upsert the content record.
            let record = ContentRecord {
                id: format!("cr_{}", compute_hash(rel_path.as_bytes())),
                content_type: content_type.to_owned(),
                file_path: rel_path.clone(),
                content_hash,
                content: content_str,
                embedding: None,
                source_path: source_path.to_owned(),
                file_size_bytes: metadata.len(),
                ingested_at: Utc::now(),
            };

            queries.upsert_content_record(&record).await?;
            summary.ingested += 1;
        }
    }

    // Remove records for files that no longer exist.
    for existing_record in &existing {
        if existing_record.source_path == source_path
            && !seen_paths.contains(&existing_record.file_path)
        {
            queries
                .delete_content_record_by_path(&existing_record.file_path)
                .await?;
            summary.removed += 1;
        }
    }

    Ok(summary)
}

/// Recursively collect all file paths in a directory.
fn collect_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_files(&path));
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Compute SHA-256 hash of content, returning a hex string.
fn compute_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Simple binary detection: check for null bytes in the first 8KB.
fn is_binary(content: &[u8]) -> bool {
    let check_len = content.len().min(8192);
    content[..check_len].contains(&0)
}

/// Ingest a single file that changed (for file watcher integration).
///
/// Computes the hash, checks against the existing record, and upserts
/// if changed. Returns `true` if the record was updated.
pub async fn ingest_single_file(
    file_path: &Path,
    workspace_root: &Path,
    content_type: &str,
    source_path: &str,
    max_file_size: u64,
    queries: &CodeGraphQueries,
) -> Result<bool, EngramError> {
    let rel_path = file_path
        .strip_prefix(workspace_root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .replace('\\', "/");

    // Check if file still exists (may have been deleted).
    if !file_path.exists() {
        queries.delete_content_record_by_path(&rel_path).await?;
        return Ok(true);
    }

    let metadata = std::fs::metadata(file_path).map_err(|e| IngestionError::Failed {
        path: rel_path.clone(),
        reason: format!("cannot read metadata: {e}"),
    })?;

    if metadata.len() > max_file_size {
        return Ok(false);
    }

    let content = std::fs::read(file_path).map_err(|e| IngestionError::Failed {
        path: rel_path.clone(),
        reason: format!("cannot read file: {e}"),
    })?;

    if is_binary(&content) {
        return Ok(false);
    }

    let content_str = String::from_utf8_lossy(&content).to_string();
    let content_hash = compute_hash(&content);

    // Check existing record for change detection.
    let existing: Vec<crate::models::ContentRecord> = queries.select_content_records(Some(content_type)).await?;
    let already_current = existing
        .iter()
        .any(|r| r.file_path == rel_path && r.content_hash == content_hash);

    if already_current {
        return Ok(false);
    }

    let record = ContentRecord {
        id: format!("cr_{}", compute_hash(rel_path.as_bytes())),
        content_type: content_type.to_owned(),
        file_path: rel_path,
        content_hash,
        content: content_str,
        embedding: None,
        source_path: source_path.to_owned(),
        file_size_bytes: metadata.len(),
        ingested_at: Utc::now(),
    };

    queries.upsert_content_record(&record).await?;
    Ok(true)
}
