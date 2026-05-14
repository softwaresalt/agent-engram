//! Content ingestion pipeline for multi-source workspace content.
//!
//! Walks registered content sources, reads files, computes content hashes,
//! and upserts [`ContentRecord`](crate::models::ContentRecord) entries into
//! SurrealDB. Supports incremental sync via content hash comparison and
//! respects configurable file size limits and batch sizes.

use std::collections::{HashMap, HashSet};
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
use crate::services::parsing::{MarkdownChunk, chunk_markdown_document_with_title_hint};

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
        // Skip sources that are explicitly known-bad: Missing or Error.
        // Active and Unknown (not yet validated) are allowed to proceed —
        // the indexers handle a missing directory gracefully.
        if source.status == ContentSourceStatus::Missing
            || source.status == ContentSourceStatus::Error
        {
            continue;
        }

        // Workspace containment check for Unknown-status sources (not yet validated).
        // Reject absolute paths, parent-directory traversal (`..`), and Windows
        // drive/UNC-prefixed paths that bypass `is_absolute()` (e.g. `C:foo`).
        if source.status == ContentSourceStatus::Unknown {
            let source_path = std::path::Path::new(&source.path);
            if source_path.is_absolute()
                || source_path.components().any(|c| {
                    c == std::path::Component::ParentDir
                        || matches!(c, std::path::Component::Prefix(_))
                })
            {
                warn!(
                    path = %source.path,
                    "Unknown-status source path may escape workspace root — skipping until validated"
                );
                continue;
            }
        }

        // Skip code sources — they use the code graph indexer instead.
        if source.content_type == "code" {
            debug!(path = %source.path, "Skipping code source (uses code graph indexer)");
            continue;
        }

        // Backlog sources use the dedicated backlog indexer.
        if source.content_type == "backlog" {
            use crate::services::backlog_indexer::{
                index_backlog_source, sweep_deleted_backlog_files,
            };

            let result =
                index_backlog_source(source, workspace_root, queries, config.max_file_size_bytes)
                    .await?;
            let removed = sweep_deleted_backlog_files(source, workspace_root, queries).await?;
            total_summary.ingested += result.ingested;
            total_summary.unchanged += result.unchanged;
            total_summary.removed += removed;
            total_summary.total_files += result.total_files;
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
#[allow(clippy::too_many_arguments)]
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
    let existing_by_path = group_content_records_by_path(existing);
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

            let desired_records = build_content_records(
                &rel_path,
                content_type,
                source_path,
                metadata.len(),
                &content_hash,
                &content_str,
            )?;

            if existing_by_path
                .get(&rel_path)
                .is_some_and(|records| content_records_match(records, &desired_records))
            {
                summary.unchanged += 1;
                continue;
            }

            queries.delete_content_record_by_path(&rel_path).await?;
            for record in &desired_records {
                queries.upsert_content_record(record).await?;
            }
            summary.ingested += 1;
        }
    }

    // Remove records for files that no longer exist.
    let mut removed_paths: HashSet<String> = HashSet::new();
    for existing_record in existing_by_path.values().flatten() {
        if existing_record.source_path == source_path
            && !seen_paths.contains(&existing_record.file_path)
            && removed_paths.insert(existing_record.file_path.clone())
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

fn group_content_records_by_path(
    records: Vec<ContentRecord>,
) -> HashMap<String, Vec<ContentRecord>> {
    let mut grouped: HashMap<String, Vec<ContentRecord>> = HashMap::new();
    for record in records {
        grouped
            .entry(record.file_path.clone())
            .or_default()
            .push(record);
    }
    grouped
}

fn content_records_match(existing: &[ContentRecord], desired: &[ContentRecord]) -> bool {
    if existing.len() != desired.len() {
        return false;
    }

    let existing_by_id: HashMap<&str, &ContentRecord> = existing
        .iter()
        .map(|record| (record.id.as_str(), record))
        .collect();

    desired.iter().all(|desired_record| {
        existing_by_id
            .get(desired_record.id.as_str())
            .is_some_and(|existing_record| {
                existing_record.content_hash == desired_record.content_hash
                    && existing_record.record_kind == desired_record.record_kind
                    && existing_record.chunk_id == desired_record.chunk_id
                    && existing_record.chunk_index == desired_record.chunk_index
                    && existing_record.heading_path == desired_record.heading_path
                    && existing_record.line_start == desired_record.line_start
                    && existing_record.line_end == desired_record.line_end
                    && existing_record.fallback_reason == desired_record.fallback_reason
                    && existing_record.lint_summary == desired_record.lint_summary
                    && existing_record.suggestions == desired_record.suggestions
            })
    })
}

fn build_content_records(
    rel_path: &str,
    content_type: &str,
    source_path: &str,
    file_size_bytes: u64,
    content_hash: &str,
    content: &str,
) -> Result<Vec<ContentRecord>, EngramError> {
    if is_markdown_path(rel_path) {
        return build_markdown_content_records(
            rel_path,
            content_type,
            source_path,
            file_size_bytes,
            content_hash,
            content,
        );
    }

    Ok(vec![file_content_record(
        rel_path,
        content_type,
        source_path,
        file_size_bytes,
        content_hash,
        content,
        None,
        None,
        Vec::new(),
    )])
}

fn build_markdown_content_records(
    rel_path: &str,
    content_type: &str,
    source_path: &str,
    file_size_bytes: u64,
    content_hash: &str,
    content: &str,
) -> Result<Vec<ContentRecord>, EngramError> {
    let title_hint = markdown_title_hint(rel_path);
    let chunks = chunk_markdown_document_with_title_hint(content, Some(&title_hint))?;

    if chunks.len() == 1 && chunks[0].record_kind == "file" {
        let fallback = &chunks[0];
        return Ok(vec![file_content_record(
            rel_path,
            content_type,
            source_path,
            file_size_bytes,
            content_hash,
            content,
            fallback.fallback_reason.clone(),
            fallback.lint_summary.clone(),
            fallback.suggestions.clone(),
        )]);
    }

    Ok(chunks
        .into_iter()
        .map(|chunk| {
            markdown_chunk_record(
                rel_path,
                content_type,
                source_path,
                file_size_bytes,
                content_hash,
                chunk,
            )
        })
        .collect())
}

#[allow(clippy::too_many_arguments)]
fn file_content_record(
    rel_path: &str,
    content_type: &str,
    source_path: &str,
    file_size_bytes: u64,
    content_hash: &str,
    content: &str,
    fallback_reason: Option<String>,
    lint_summary: Option<String>,
    suggestions: Vec<String>,
) -> ContentRecord {
    let identity_seed = content_record_identity_seed(rel_path, content_type, source_path, None);
    ContentRecord {
        id: format!("cr_{}", compute_hash(identity_seed.as_bytes())),
        content_type: content_type.to_owned(),
        file_path: rel_path.to_owned(),
        content_hash: content_hash.to_owned(),
        content: content.to_owned(),
        embedding: None,
        source_path: source_path.to_owned(),
        file_size_bytes,
        ingested_at: Utc::now(),
        record_kind: "file".to_owned(),
        chunk_id: None,
        chunk_index: None,
        heading_path: Vec::new(),
        line_start: None,
        line_end: None,
        fallback_reason,
        lint_summary,
        suggestions,
    }
}

fn markdown_chunk_record(
    rel_path: &str,
    content_type: &str,
    source_path: &str,
    file_size_bytes: u64,
    content_hash: &str,
    chunk: MarkdownChunk,
) -> ContentRecord {
    let chunk_key = content_record_identity_seed(
        rel_path,
        content_type,
        source_path,
        Some(chunk.chunk_id.as_str()),
    );
    ContentRecord {
        id: format!("cr_{}", compute_hash(chunk_key.as_bytes())),
        content_type: content_type.to_owned(),
        file_path: rel_path.to_owned(),
        content_hash: content_hash.to_owned(),
        content: chunk.content,
        embedding: None,
        source_path: source_path.to_owned(),
        file_size_bytes,
        ingested_at: Utc::now(),
        record_kind: chunk.record_kind,
        chunk_id: Some(chunk.chunk_id),
        chunk_index: Some(chunk.chunk_index),
        heading_path: chunk.heading_path,
        line_start: chunk.line_start,
        line_end: chunk.line_end,
        fallback_reason: chunk.fallback_reason,
        lint_summary: chunk.lint_summary,
        suggestions: chunk.suggestions,
    }
}

fn content_record_identity_seed(
    rel_path: &str,
    content_type: &str,
    source_path: &str,
    chunk_id: Option<&str>,
) -> String {
    match chunk_id {
        Some(chunk_id) => format!("{source_path}:{content_type}:{rel_path}:{chunk_id}"),
        None => format!("{source_path}:{content_type}:{rel_path}"),
    }
}

fn is_markdown_path(rel_path: &str) -> bool {
    Path::new(rel_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

fn markdown_title_hint(rel_path: &str) -> String {
    let filename = Path::new(rel_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("Notes");

    let mut titled = String::new();
    let mut capitalize_next = true;
    for character in filename.chars() {
        if matches!(character, '-' | '_' | ' ') {
            if !titled.is_empty() && !titled.ends_with(' ') {
                titled.push(' ');
            }
            capitalize_next = true;
            continue;
        }

        if capitalize_next {
            titled.extend(character.to_uppercase());
            capitalize_next = false;
        } else {
            titled.extend(character.to_lowercase());
        }
    }

    titled.trim().to_owned()
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
    let existing_by_path = group_content_records_by_path(existing);
    let desired_records = build_content_records(
        &rel_path,
        content_type,
        source_path,
        metadata.len(),
        &content_hash,
        &content_str,
    )?;
    let already_current = existing_by_path
        .get(&rel_path)
        .is_some_and(|records| content_records_match(records, &desired_records));

    if already_current {
        return Ok(false);
    }

    queries.delete_content_record_by_path(&rel_path).await?;
    for record in &desired_records {
        queries.upsert_content_record(record).await?;
    }
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
pub async fn backfill_content_embeddings(queries: &CodeGraphQueries) -> Result<usize, EngramError> {
    let records = queries.select_content_records(None).await?;

    let pending: Vec<_> = records
        .into_iter()
        .filter(|r| r.embedding.as_ref().is_none_or(Vec::is_empty))
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
    for (record, vector) in pending.iter().zip(vectors) {
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

#[cfg(test)]
mod tests {
    use super::content_record_identity_seed;

    #[test]
    fn content_record_identity_seed_scopes_file_records_by_source_and_type() {
        let docs = content_record_identity_seed("docs/guide.md", "docs", "docs", None);
        let specs = content_record_identity_seed("docs/guide.md", "spec", "specs", None);
        assert_ne!(docs, specs);
    }

    #[test]
    fn content_record_identity_seed_scopes_chunk_records_by_source_and_type() {
        let docs =
            content_record_identity_seed("docs/guide.md", "docs", "docs", Some("guide/install"));
        let specs =
            content_record_identity_seed("docs/guide.md", "spec", "specs", Some("guide/install"));
        assert_ne!(docs, specs);
    }
}
