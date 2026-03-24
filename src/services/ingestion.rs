//! Content ingestion pipeline for multi-source workspace content.
//!
//! Walks registered content sources, reads files, computes content hashes,
//! and upserts [`ContentRecord`](crate::models::ContentRecord) entries into
//! SurrealDB. Supports incremental sync via content hash comparison and
//! respects configurable file size limits and batch sizes.

use std::collections::HashSet;
use std::path::Path;

use chrono::Utc;
use globset::{Glob, GlobSet, GlobSetBuilder};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

/// Maximum characters of content passed to the embedding model per record.
///
/// Content is truncated to this limit before embedding to stay within
/// model token budgets and prevent excessive memory usage during backfill.
const MAX_EMBED_CHARS: usize = 4_096;

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
/// When a source declares a `pattern`, only files matching that glob are ingested.
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

        // Build a glob filter from the optional pattern field.
        let glob_filter = build_glob_filter(source.pattern.as_deref());

        let source_path = workspace_root.join(&source.path);
        let summary = ingest_directory(
            &source_path,
            workspace_root,
            &source.content_type,
            &source.path,
            config.max_file_size_bytes,
            config.batch_size,
            glob_filter.as_ref(),
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

/// Build a [`GlobSet`] from an optional pattern string.
///
/// Returns `None` when no pattern is provided or when the pattern is invalid
/// (a warning is logged for invalid patterns so the source is ingested in full
/// rather than silently dropped).
fn build_glob_filter(pattern: Option<&str>) -> Option<GlobSet> {
    let pat = pattern?;
    let glob = match Glob::new(pat) {
        Ok(g) => g,
        Err(e) => {
            warn!(pattern = %pat, error = %e, "invalid glob pattern in registry source — pattern filter disabled for this source");
            return None;
        }
    };
    let mut builder = GlobSetBuilder::new();
    builder.add(glob);
    match builder.build() {
        Ok(gs) => Some(gs),
        Err(e) => {
            warn!(pattern = %pat, error = %e, "failed to build glob set — pattern filter disabled for this source");
            None
        }
    }
}

/// Ingest all eligible files from a single directory.
async fn ingest_directory(
    dir_path: &Path,
    workspace_root: &Path,
    content_type: &str,
    source_path: &str,
    max_file_size: u64,
    batch_size: usize,
    glob_filter: Option<&GlobSet>,
    queries: &CodeGraphQueries,
) -> Result<IngestionSummary, EngramError> {
    let mut summary = IngestionSummary::default();

    if !dir_path.is_dir() {
        return Ok(summary);
    }

    // Collect all files recursively then apply the glob filter.
    let files: Vec<_> = collect_files(dir_path)
        .into_iter()
        .filter(|p| {
            if let Some(gs) = glob_filter {
                let rel = p
                    .strip_prefix(dir_path)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .replace('\\', "/");
                gs.is_match(rel.as_str())
            } else {
                true
            }
        })
        .collect();
    summary.total_files = files.len();

    // Get existing records to detect changes.
    let existing: Vec<crate::models::ContentRecord> =
        queries.select_content_records(Some(content_type)).await?;
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
/// if changed. When `glob_filter` is `Some`, the file is only ingested if its
/// name matches the pattern. Returns `true` if the record was updated.
pub async fn ingest_single_file(
    file_path: &Path,
    workspace_root: &Path,
    content_type: &str,
    source_path: &str,
    max_file_size: u64,
    glob_filter: Option<&GlobSet>,
    queries: &CodeGraphQueries,
) -> Result<bool, EngramError> {
    let rel_path = file_path
        .strip_prefix(workspace_root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .replace('\\', "/");

    // Apply glob pattern filter if configured.
    if let Some(gs) = glob_filter {
        let filename = file_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !gs.is_match(filename.as_str()) && !gs.is_match(rel_path.as_str()) {
            return Ok(false);
        }
    }

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
    let existing: Vec<crate::models::ContentRecord> =
        queries.select_content_records(Some(content_type)).await?;
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

/// Generate and store embeddings for content records that currently have none.
///
/// Queries all content records, filters those lacking an embedding vector,
/// truncates content to [`MAX_EMBED_CHARS`] characters, batch-embeds via
/// [`crate::services::embedding::embed_texts`], and writes each vector back
/// using [`CodeGraphQueries::update_content_record_embedding`].
///
/// Non-fatal: if the `embeddings` feature is disabled or the ONNX model
/// cannot be loaded, the function returns `Ok(0)` immediately after a
/// debug-level trace event.
///
/// Returns the number of records that received a new embedding.
///
/// # Errors
///
/// Returns `EngramError` only on database query failures.
pub async fn backfill_content_embeddings(
    queries: &CodeGraphQueries,
) -> Result<usize, EngramError> {
    let records = queries.select_content_records(None).await?;

    let pending: Vec<_> = records
        .into_iter()
        .filter(|r| r.embedding.as_ref().map_or(true, Vec::is_empty))
        .collect();

    if pending.is_empty() {
        debug!("content embedding backfill: all records already have embeddings");
        return Ok(0);
    }

    info!(
        count = pending.len(),
        "content embedding backfill: generating embeddings for content records"
    );

    let texts: Vec<String> = pending
        .iter()
        .map(|r| r.content.chars().take(MAX_EMBED_CHARS).collect())
        .collect();

    let vectors = match crate::services::embedding::embed_texts(&texts) {
        Ok(vecs) => vecs,
        Err(e) => {
            debug!(
                error = %e,
                "content embedding model unavailable — backfill skipped"
            );
            return Ok(0);
        }
    };

    let mut updated = 0usize;
    for (record, vector) in pending.iter().zip(vectors.into_iter()) {
        if let Err(e) = queries
            .update_content_record_embedding(&record.id, vector)
            .await
        {
            debug!(
                error = %e,
                record_id = %record.id,
                "content embedding write-back failed"
            );
        } else {
            updated += 1;
        }
    }

    info!(updated, "content embedding backfill complete");
    Ok(updated)
}
