//! Code graph indexing orchestration service.
//!
//! Coordinates file discovery, parallel parsing, tiered embedding,
//! incremental sync, and concerns edge management.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::connect_db;
use crate::db::queries::CodeGraphQueries;
use crate::errors::EngramError;
use crate::models::code_file::CodeFile;
use crate::models::config::CodeGraphConfig;
use crate::services::embedding;
use crate::services::parsing::canonical;
use crate::services::parsing::{ExtractedEdge, ExtractedSymbol, Language, parse_source};

/// Build the per-file canonical resolution context for a Rust source file, or
/// `None` for non-Rust files and for layouts whose module path is not
/// deterministically derivable (fail-closed → empty `canonical_path`).
///
/// Part of Option C Unit A / A6 (precision-neutral): produces identity data
/// only; no call edges are emitted.
fn rust_canonical_ctx(
    crates: &canonical::WorkspaceCrates,
    lang: Language,
    rel_path: &str,
    source: &str,
) -> Option<(canonical::ModulePath, canonical::UseGraph)> {
    if lang != Language::Rust {
        return None;
    }
    let module = canonical::module_path_for_file(crates, rel_path)?;
    Some((module, canonical::extract_use_graph(source)))
}

/// Resolve a parsed function/method's additive `canonical_path`, or `""` when it
/// cannot be resolved (never a canonical match target — D4).
fn canonical_path_for_function(
    crates: &canonical::WorkspaceCrates,
    ctx: Option<&(canonical::ModulePath, canonical::UseGraph)>,
    name: &str,
) -> String {
    let Some((module, use_graph)) = ctx else {
        return String::new();
    };
    let rctx = canonical::ResolveContext {
        module,
        crates,
        use_graph,
    };
    canonical::canonical_path_for_def(&rctx, name)
        .map(canonical::CanonicalId::into_string)
        .unwrap_or_default()
}

/// Summary returned by [`index_workspace`] after indexing completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    /// Number of source files successfully parsed.
    pub files_parsed: usize,
    /// Number of files skipped (unsupported, too large, or unchanged).
    pub files_skipped: usize,
    /// Number of files skipped specifically because they exceeded
    /// [`CodeGraphConfig::max_file_size_bytes`].
    ///
    /// Counted separately from `files_skipped` so callers can distinguish
    /// capacity-policy skips from parse errors and unchanged-file skips.
    /// Oversized files are not added to `errors` — they are a normal
    /// policy outcome, not a processing failure.
    pub oversized_files_skipped: usize,
    /// Number of function records upserted.
    pub functions_indexed: usize,
    /// Number of class (struct) records upserted.
    pub classes_indexed: usize,
    /// Number of interface (trait) records upserted.
    pub interfaces_indexed: usize,
    /// Number of edge records created.
    pub edges_created: usize,
    /// Number of embedding vectors generated.
    pub embeddings_generated: usize,
    /// Count of Tier 1 (`explicit_code`) symbols.
    pub tier1_count: usize,
    /// Count of Tier 2 (`summary_pointer`) symbols.
    pub tier2_count: usize,
    /// Number of cross-file import/call edges dropped (deferred to future phase).
    pub cross_file_edges_dropped: usize,
    /// Per-file errors encountered (non-fatal).
    pub errors: Vec<FileError>,
    /// Total indexing duration in milliseconds.
    pub duration_ms: u64,
}

/// A non-fatal error encountered while indexing a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileError {
    /// Workspace-relative file path.
    pub file: String,
    /// Error description.
    pub error: String,
}

/// Callback invoked with `(completed, total)` file progress during index or sync.
pub type ProgressCallback<'a> = dyn FnMut(u64, u64) + Send + 'a;

fn emit_progress(progress: &mut Option<&mut ProgressCallback<'_>>, completed: u64, total: u64) {
    if let Some(callback) = progress.as_deref_mut() {
        callback(completed, total);
    }
}

fn advance_progress(
    progress: &mut Option<&mut ProgressCallback<'_>>,
    completed: &mut u64,
    total: u64,
) {
    *completed += 1;
    emit_progress(progress, *completed, total);
}

/// Discover, parse, and index all supported source files in the workspace.
///
/// Uses the `ignore` crate for .gitignore-aware file traversal, filters by
/// supported languages and file size, parses via tree-sitter, assigns tiered
/// embeddings, and persists all nodes and edges to CozoDB.
///
/// `SQLITE_BUSY` retries are handled at the individual `run_script` level
/// inside `upsert_function`, `upsert_class`, and `upsert_interface`, so
/// per-symbol writes retry independently without skipping files whose
/// `content_hash` was already committed.
///
/// # Errors
///
/// Returns `EngramError` on database connection failure or fatal I/O errors.
/// Per-file parse errors are collected in `IndexResult::errors` (non-fatal).
pub async fn index_workspace(
    ws_path: &Path,
    data_dir: &Path,
    branch: &str,
    config: &CodeGraphConfig,
    force: bool,
) -> Result<IndexResult, EngramError> {
    index_workspace_with_progress(ws_path, data_dir, branch, config, force, None).await
}

/// Discover, parse, and index all supported source files while reporting
/// progress snapshots to interested callers.
pub async fn index_workspace_with_progress(
    ws_path: &Path,
    data_dir: &Path,
    branch: &str,
    config: &CodeGraphConfig,
    force: bool,
    progress: Option<&mut ProgressCallback<'_>>,
) -> Result<IndexResult, EngramError> {
    index_workspace_impl(ws_path, data_dir, branch, config, force, progress).await
}

async fn index_workspace_impl(
    ws_path: &Path,
    data_dir: &Path,
    branch: &str,
    config: &CodeGraphConfig,
    force: bool,
    mut progress: Option<&mut ProgressCallback<'_>>,
) -> Result<IndexResult, EngramError> {
    let start = std::time::Instant::now();

    let db = connect_db(data_dir, branch).await?;
    let queries = CodeGraphQueries::new(db);

    // Option C Unit A / A6: workspace crate set for canonical-identity derivation
    // (computed once per index run; Rust-only, precision-neutral).
    let crates = canonical::discover_workspace_crates(ws_path);

    // ── Step 1: Discover files ──────────────────────────────────────
    let files = discover_files(ws_path, config);
    info!(
        files_found = files.len(),
        "code graph: discovered source files"
    );
    let total_files = files.len() as u64;
    let mut completed_files = 0_u64;
    emit_progress(&mut progress, completed_files, total_files);

    let mut result = IndexResult {
        files_parsed: 0,
        files_skipped: 0,
        oversized_files_skipped: 0,
        functions_indexed: 0,
        classes_indexed: 0,
        interfaces_indexed: 0,
        edges_created: 0,
        embeddings_generated: 0,
        tier1_count: 0,
        tier2_count: 0,
        cross_file_edges_dropped: 0,
        errors: Vec::new(),
        duration_ms: 0,
    };

    // ── Step 2: Process each file ───────────────────────────────────
    for file_path in &files {
        'file: {
            let rel_path = if let Ok(p) = file_path.strip_prefix(ws_path) {
                p.to_string_lossy().replace('\\', "/")
            } else {
                warn!(path = %file_path.display(), "code graph: file outside workspace root, skipping");
                result.files_skipped += 1;
                break 'file;
            };

            // ── Early size check via filesystem metadata ────────────────
            // Avoids reading large files into memory only to discard them.
            // A metadata I/O failure is non-fatal: fall through to the
            // content-based check that follows the file read.
            if let Ok(meta) = tokio::fs::metadata(file_path).await {
                if meta.len() > config.max_file_size_bytes {
                    let stale_file_id = format!("code_file:{}", sha256_short(&rel_path));
                    let _orphaned =
                        handle_deleted_file(&queries, &rel_path, &stale_file_id).await?;
                    warn!(
                        path = %rel_path,
                        size_bytes = meta.len(),
                        limit_bytes = config.max_file_size_bytes,
                        "code graph: skipping oversized file"
                    );
                    result.oversized_files_skipped += 1;
                    result.files_skipped += 1;
                    break 'file;
                }
            }

            // Read file contents.
            let source = match tokio::fs::read_to_string(file_path).await {
                Ok(s) => s,
                Err(e) => {
                    result.errors.push(FileError {
                        file: rel_path.clone(),
                        error: format!("read error: {e}"),
                    });
                    result.files_skipped += 1;
                    break 'file;
                }
            };

            // Secondary size guard: protects against metadata races (TOCTOU).
            let size_bytes = source.len() as u64;
            if size_bytes > config.max_file_size_bytes {
                let stale_file_id = format!("code_file:{}", sha256_short(&rel_path));
                let _orphaned = handle_deleted_file(&queries, &rel_path, &stale_file_id).await?;
                warn!(
                    path = %rel_path,
                    size_bytes,
                    limit_bytes = config.max_file_size_bytes,
                    "code graph: skipping oversized file (content check)"
                );
                result.oversized_files_skipped += 1;
                result.files_skipped += 1;
                break 'file;
            }

            // Compute content hash.
            let content_hash = sha256_hex(&source);

            // Skip unchanged files (unless force).
            if !force {
                if let Ok(Some(existing)) = queries.get_code_file_by_path(&rel_path).await {
                    if existing.content_hash == content_hash {
                        debug!(path = %rel_path, "code graph: skipping unchanged file");
                        result.files_skipped += 1;
                        break 'file;
                    }
                }
            }

            // Detect language from extension.
            let lang = language_from_path(file_path);
            if !config.supported_languages.contains(&lang) {
                result.files_skipped += 1;
                break 'file;
            }

            // ── Parse via tree-sitter (CPU-bound, run in blocking task) ─
            let source_clone = source.clone();
            let lang_enum = match Language::try_from(lang.as_str()) {
                Ok(l) => l,
                Err(e) => {
                    result.errors.push(FileError {
                        file: rel_path.clone(),
                        error: e.to_string(),
                    });
                    result.files_skipped += 1;
                    break 'file;
                }
            };
            let parse_result =
                match tokio::task::spawn_blocking(move || parse_source(&source_clone, lang_enum))
                    .await
                {
                    Ok(Ok(pr)) => pr,
                    Ok(Err(e)) => {
                        result.errors.push(FileError {
                            file: rel_path.clone(),
                            error: e.to_string(),
                        });
                        result.files_skipped += 1;
                        break 'file;
                    }
                    Err(e) => {
                        result.errors.push(FileError {
                            file: rel_path.clone(),
                            error: format!("task join error: {e}"),
                        });
                        result.files_skipped += 1;
                        break 'file;
                    }
                };

            // ── Upsert code file node ───────────────────────────────────
            let file_id = format!("code_file:{}", sha256_short(&rel_path));
            let code_file = CodeFile {
                id: file_id.clone(),
                path: rel_path.clone(),
                language: lang.clone(),
                size_bytes,
                content_hash,
                last_indexed_at: chrono::Utc::now().to_rfc3339(),
            };
            queries.upsert_code_file(&code_file).await?;

            // Clear previous edges from this file.
            // 082.009-T: retract this file's prior calls_resolved_singleton
            // edges (caller or callee in this file) WHILE the old symbol IDs
            // still exist, and clear its staged calls, before deleting symbols
            // and re-staging.
            queries
                .retract_resolved_calls_edges_for_file(&rel_path)
                .await?;
            queries.clear_staged_calls_for_file(&rel_path).await?;
            queries.delete_functions_by_file(&rel_path).await?;
            queries.delete_classes_by_file(&rel_path).await?;
            queries.delete_interfaces_by_file(&rel_path).await?;
            queries.delete_edges_from_file("defines", &file_id).await?;
            queries
                .delete_edges_from_file("references", &file_id)
                .await?;

            // ── Collect symbols for embedding ───────────────────────────
            let token_limit = config.embedding.token_limit;
            let mut embed_texts: Vec<String> = Vec::new();
            let mut embed_ids: Vec<String> = Vec::new();

            // Track symbol IDs for edge creation.
            let mut function_ids: Vec<(String, String)> = Vec::new(); // (name, id)
            let mut class_ids: Vec<(String, String)> = Vec::new();
            let mut interface_ids: Vec<(String, String)> = Vec::new();

            // A6: per-file canonical context (Rust-only; None → empty canonical_path).
            let rust_ctx = rust_canonical_ctx(&crates, lang_enum, &rel_path, &source);

            for symbol in &parse_result.symbols {
                match symbol {
                    ExtractedSymbol::Function(f) => {
                        let sym_id = format!("function:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            f.token_count as usize,
                            token_limit,
                            &f.body,
                            &f.signature,
                            f.docstring.as_deref(),
                        );
                        embed_texts.push(summary.clone());
                        embed_ids.push(sym_id.clone());

                        let func = crate::models::function::Function {
                            id: sym_id.clone(),
                            name: f.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: f.line_start,
                            line_end: f.line_end,
                            signature: f.signature.clone(),
                            docstring: f.docstring.clone(),
                            body: f.body.clone(),
                            body_hash: f.body_hash.clone(),
                            token_count: f.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: vec![0.0_f32; embedding::EMBEDDING_DIM],
                            summary,
                        };
                        let canonical_path =
                            canonical_path_for_function(&crates, rust_ctx.as_ref(), &f.name);
                        queries
                            .upsert_function_with_canonical(&func, &canonical_path)
                            .await?;
                        function_ids.push((f.name.clone(), sym_id.clone()));

                        if embed_type == "explicit_code" {
                            result.tier1_count += 1;
                        } else {
                            result.tier2_count += 1;
                        }
                        result.functions_indexed += 1;

                        // Create defines edge.
                        queries
                            .create_defines_edge(&file_id, "function", &sym_id)
                            .await?;
                        result.edges_created += 1;
                    }
                    ExtractedSymbol::Class(c) => {
                        let sym_id = format!("class:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            c.token_count as usize,
                            token_limit,
                            &c.body,
                            "",
                            c.docstring.as_deref(),
                        );
                        embed_texts.push(summary.clone());
                        embed_ids.push(sym_id.clone());

                        let class = crate::models::class::Class {
                            id: sym_id.clone(),
                            name: c.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: c.line_start,
                            line_end: c.line_end,
                            docstring: c.docstring.clone(),
                            body: c.body.clone(),
                            body_hash: c.body_hash.clone(),
                            token_count: c.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: vec![0.0_f32; embedding::EMBEDDING_DIM],
                            summary,
                        };
                        queries.upsert_class(&class).await?;
                        class_ids.push((c.name.clone(), sym_id.clone()));

                        if embed_type == "explicit_code" {
                            result.tier1_count += 1;
                        } else {
                            result.tier2_count += 1;
                        }
                        result.classes_indexed += 1;

                        queries
                            .create_defines_edge(&file_id, "class", &sym_id)
                            .await?;
                        result.edges_created += 1;
                    }
                    ExtractedSymbol::Interface(i) => {
                        let sym_id = format!("interface:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            i.token_count as usize,
                            token_limit,
                            &i.body,
                            "",
                            i.docstring.as_deref(),
                        );
                        embed_texts.push(summary.clone());
                        embed_ids.push(sym_id.clone());

                        let iface = crate::models::interface::Interface {
                            id: sym_id.clone(),
                            name: i.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: i.line_start,
                            line_end: i.line_end,
                            docstring: i.docstring.clone(),
                            body: i.body.clone(),
                            body_hash: i.body_hash.clone(),
                            token_count: i.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: vec![0.0_f32; embedding::EMBEDDING_DIM],
                            summary,
                        };
                        queries.upsert_interface(&iface).await?;
                        interface_ids.push((i.name.clone(), sym_id.clone()));

                        if embed_type == "explicit_code" {
                            result.tier1_count += 1;
                        } else {
                            result.tier2_count += 1;
                        }
                        result.interfaces_indexed += 1;

                        queries
                            .create_defines_edge(&file_id, "interface", &sym_id)
                            .await?;
                        result.edges_created += 1;
                    }
                }
            }

            // ── Batch embed (non-fatal if model not loaded) ─────────────
            if !embed_texts.is_empty() {
                match embedding::embed_texts(&embed_texts) {
                    Ok(vectors) => {
                        result.embeddings_generated += vectors.len();
                        for (sym_id, vector) in embed_ids.iter().zip(vectors) {
                            if let Err(e) = queries.update_symbol_embedding(sym_id, vector).await {
                                debug!(error = %e, sym_id = %sym_id, "code graph: embedding write-back failed");
                            }
                        }
                        debug!(
                            count = result.embeddings_generated,
                            "code graph: generated and stored embeddings for file"
                        );
                    }
                    Err(e) => {
                        debug!(error = %e, "code graph: embedding unavailable, skipping");
                    }
                }
            }

            // ── Create edges from extracted relationships ───────────────
            for edge in &parse_result.edges {
                match edge {
                    ExtractedEdge::Calls {
                        caller,
                        callee,
                        is_method,
                        is_qualified,
                    } => {
                        // Method/receiver (`self.bar()`) and path-qualified
                        // (`Type::parse()`) calls are extracted for completeness
                        // (082.001-T) but NOT promoted to a calls_edge: their
                        // targets are indexed under qualified names, so name-only
                        // resolution cannot match them and would create a false
                        // singleton edge. Deferred pending qualification/method-
                        // aware resolution.
                        if *is_method || *is_qualified {
                            continue;
                        }
                        // Resolve names to IDs within this file's symbols. A
                        // callee resolved locally becomes a direct edge; a
                        // caller-resolved but callee-unresolved (cross-file)
                        // call is staged for the deferred post-pass (082.002-T)
                        // instead of being silently dropped.
                        match (
                            find_function_id(&function_ids, caller),
                            find_function_id(&function_ids, callee),
                        ) {
                            (Some(from_id), Some(to_id)) => {
                                queries.create_calls_edge(&from_id, &to_id).await?;
                                result.edges_created += 1;
                            }
                            (Some(from_id), None) => {
                                queries.put_staged_call(&from_id, callee, &rel_path).await?;
                            }
                            _ => {}
                        }
                    }
                    ExtractedEdge::InheritsFrom {
                        struct_name,
                        trait_name,
                    } => {
                        if let Some(child_id) = find_class_id(&class_ids, struct_name) {
                            if let Some(parent_id) = find_interface_id(&interface_ids, trait_name) {
                                queries
                                    .create_inherits_edge(
                                        "class",
                                        &child_id,
                                        "interface",
                                        &parent_id,
                                    )
                                    .await?;
                                result.edges_created += 1;
                            }
                        }
                    }
                    // Defines already created above; Imports are cross-file (deferred, counted).
                    ExtractedEdge::Imports { .. } => {
                        result.cross_file_edges_dropped += 1;
                    }
                    // Defines edge already handled during symbol upsert.
                    ExtractedEdge::Defines { .. } => {}
                    // SQL References: resolve target to a Class node or self-loop (033.001-T).
                    ExtractedEdge::References { target, .. } => {
                        let resolved_id = queries.resolve_reference_target(target).await?;
                        if let Some(class_id) = resolved_id {
                            queries
                                .create_references_edge(&file_id, &class_id, Some(target))
                                .await?;
                        } else {
                            queries
                                .create_references_edge(&file_id, &file_id, Some(target))
                                .await?;
                        }
                        result.edges_created += 1;
                    }
                }
            }

            result.files_parsed += 1;
            // Record file hash for offline change detection (TASK-009.09).
            // Non-fatal: a hash recording failure degrades offline detection but
            // does not invalidate the indexed code graph.
            if let Err(e) =
                crate::services::file_tracker::record_file_hash(&rel_path, file_path, &queries)
                    .await
            {
                debug!(
                    error = %e,
                    path = %rel_path,
                    "code graph: file hash recording failed — offline detection may report false changes"
                );
            }
            debug!(path = %rel_path, "code graph: indexed file");
        }
        // Progress counts completed file decisions, not only successfully indexed
        // files, so skips and non-fatal per-file errors still advance the caller's
        // view of total work completed.
        advance_progress(&mut progress, &mut completed_files, total_files);
    }

    // ── Post-pass: re-resolve unresolved references edges ───────────
    // Files are processed in filesystem order. A reference to `public.users`
    // may be processed before the `users` class is created, leaving a self-loop.
    // Now that all symbols exist, retry resolution for those edges.
    let reresolved = queries.reresolve_references_edges().await?;
    if reresolved.resolved > 0 {
        debug!(
            count = reresolved.resolved,
            "code graph: re-resolved deferred references edges"
        );
    }

    // ── Post-pass: resolve staged cross-file calls (082.008-T) ──────
    // Full / --force index only — NOT the incremental sync path (performance
    // gate). Staged calls (082.002-T) whose callee name is unambiguous
    // (exactly one workspace-global definition) become calls_resolved_singleton
    // edges; ambiguous / unmatched names are skipped to bound false edges.
    let resolved_calls = queries.reresolve_calls_edges().await?;
    // Post-pass singletons are real edge records: include them in the reported
    // edges_created so full-index CLI/API responses do not underreport (082.008-T).
    result.edges_created += resolved_calls.resolved;
    if resolved_calls.resolved > 0 {
        debug!(
            count = resolved_calls.resolved,
            lookups = resolved_calls.lookups,
            "code graph: resolved cross-file singleton calls edges"
        );
    }

    #[allow(clippy::cast_possible_truncation)]
    let elapsed = start.elapsed().as_millis() as u64;
    result.duration_ms = elapsed;
    info!(
        files_parsed = result.files_parsed,
        files_skipped = result.files_skipped,
        oversized_files_skipped = result.oversized_files_skipped,
        functions = result.functions_indexed,
        classes = result.classes_indexed,
        interfaces = result.interfaces_indexed,
        edges = result.edges_created,
        duration_ms = result.duration_ms,
        "code graph: indexing complete"
    );

    Ok(result)
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Summary returned by [`sync_workspace`] after incremental sync completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Number of files that were modified and re-indexed.
    pub files_modified: usize,
    /// Number of new files added and indexed.
    pub files_added: usize,
    /// Number of files deleted (nodes removed).
    pub files_deleted: usize,
    /// Number of files unchanged (skipped).
    pub files_unchanged: usize,
    /// Number of symbols that were re-embedded because their body changed.
    pub symbols_reembedded: usize,
    /// Number of symbols that kept existing embeddings (body unchanged).
    pub symbols_reused: usize,
    /// Number of `concerns` edges re-linked to new symbol nodes (FR-124).
    pub concerns_relinked: usize,
    /// Number of `concerns` edges orphaned and removed (FR-112).
    pub concerns_orphaned: usize,
    /// Number of edge records created (defines, references, concerns).
    pub edges_created: usize,
    /// Number of cross-file import/call edges dropped (deferred to future phase).
    pub cross_file_edges_dropped: usize,
    /// Number of files skipped specifically because they exceeded
    /// [`CodeGraphConfig::max_file_size_bytes`].
    ///
    /// Oversized files are not added to `errors` — they are a normal
    /// policy outcome, not a processing failure.
    pub oversized_files_skipped: usize,
    /// Per-file errors encountered (non-fatal).
    pub errors: Vec<FileError>,
    /// Total sync duration in milliseconds.
    pub duration_ms: u64,
}

/// Incrementally sync the code graph with changes on disk.
///
/// Detects changed, added, and deleted files since the last index
/// and updates only affected nodes. Uses two-level hashing:
///
/// 1. **File-level** – `content_hash` on `code_file` nodes identifies
///    which files changed on disk.
/// 2. **Symbol-level** – `body_hash` on function/class/interface nodes
///    identifies which symbols within a changed file actually need
///    re-embedding.
///
/// Preserves `concerns` edges across file moves via hash-resilient
/// identity matching on `(name, body_hash)` tuples (FR-124).
///
/// If no prior index exists, falls back to a full index (same outcome
/// as calling `index_workspace`).
///
/// # Errors
///
/// Returns `EngramError` on database connection failure or fatal I/O errors.
/// Per-file parse errors are collected in `SyncResult::errors` (non-fatal).
pub async fn sync_workspace(
    ws_path: &Path,
    data_dir: &Path,
    branch: &str,
    config: &CodeGraphConfig,
) -> Result<SyncResult, EngramError> {
    sync_workspace_with_progress(ws_path, data_dir, branch, config, None).await
}

/// Incrementally sync the code graph while reporting `(completed, total)` file
/// progress to callers that want streamed startup visibility.
pub async fn sync_workspace_with_progress(
    ws_path: &Path,
    data_dir: &Path,
    branch: &str,
    config: &CodeGraphConfig,
    mut progress: Option<&mut ProgressCallback<'_>>,
) -> Result<SyncResult, EngramError> {
    let start = std::time::Instant::now();

    let db = connect_db(data_dir, branch).await?;
    let queries = CodeGraphQueries::new(db);

    // Option C Unit A / A6: workspace crate set for canonical-identity derivation.
    let crates = canonical::discover_workspace_crates(ws_path);

    // Discover current files on disk.
    let current_files = discover_files(ws_path, config);

    // Load all indexed code files from DB.
    let indexed_files = queries.list_code_files().await?;
    let indexed_map: HashMap<String, CodeFile> = indexed_files
        .into_iter()
        .map(|f| (f.path.clone(), f))
        .collect();

    // Build a set of current relative paths for deletion detection.
    let current_rel_paths: std::collections::HashSet<String> = current_files
        .iter()
        .filter_map(|p| {
            p.strip_prefix(ws_path)
                .ok()
                .map(|r| r.to_string_lossy().replace('\\', "/"))
        })
        .collect();
    let deleted_paths: Vec<_> = indexed_map
        .iter()
        .filter(|(indexed_path, _)| !current_rel_paths.contains(*indexed_path))
        .collect();
    let total_files = (deleted_paths.len() + current_files.len()) as u64;
    let mut completed_files = 0_u64;
    emit_progress(&mut progress, completed_files, total_files);

    let mut result = SyncResult {
        files_modified: 0,
        files_added: 0,
        files_deleted: 0,
        files_unchanged: 0,
        symbols_reembedded: 0,
        symbols_reused: 0,
        concerns_relinked: 0,
        concerns_orphaned: 0,
        edges_created: 0,
        cross_file_edges_dropped: 0,
        oversized_files_skipped: 0,
        errors: Vec::new(),
        duration_ms: 0,
    };

    // ── Phase 1: Detect and remove deleted files ────────────────────
    for (indexed_path, indexed_file) in deleted_paths {
        // File deleted — collect concerns edges before removing symbols.
        let orphaned = handle_deleted_file(&queries, indexed_path, &indexed_file.id).await?;
        result.concerns_orphaned += orphaned;
        result.files_deleted += 1;
        // Progress counts completed file decisions, including unchanged/skipped
        // files, so callers see steady forward motion through the full sync set.
        advance_progress(&mut progress, &mut completed_files, total_files);
    }

    // ── Phase 2: Process current files (add / modify / skip) ────────
    for file_path in &current_files {
        'file: {
            let rel_path = if let Ok(p) = file_path.strip_prefix(ws_path) {
                p.to_string_lossy().replace('\\', "/")
            } else {
                warn!(path = %file_path.display(), "code graph sync: file outside workspace root, skipping");
                break 'file;
            };

            // ── Early size check via filesystem metadata ────────────────
            // Avoids reading large files into memory only to discard them.
            // A metadata I/O failure is non-fatal: fall through to the
            // content-based checks that follow the file read.
            if let Ok(meta) = tokio::fs::metadata(file_path).await {
                let meta_size = meta.len();
                if meta_size == 0 {
                    result.files_unchanged += 1;
                    break 'file;
                }
                if meta_size > config.max_file_size_bytes {
                    let stale_file_id = format!("code_file:{}", sha256_short(&rel_path));
                    let orphaned = handle_deleted_file(&queries, &rel_path, &stale_file_id).await?;
                    result.concerns_orphaned += orphaned;
                    warn!(
                        path = %rel_path,
                        size_bytes = meta_size,
                        limit_bytes = config.max_file_size_bytes,
                        "code graph sync: skipping oversized file"
                    );
                    result.oversized_files_skipped += 1;
                    break 'file;
                }
            }

            // Read file contents.
            let source = match tokio::fs::read_to_string(file_path).await {
                Ok(s) => s,
                Err(e) => {
                    result.errors.push(FileError {
                        file: rel_path.clone(),
                        error: format!("read error: {e}"),
                    });
                    break 'file;
                }
            };

            let size_bytes = source.len() as u64;
            if size_bytes == 0 {
                // Skip empty files (handles TOCTOU race between metadata and read).
                result.files_unchanged += 1;
                break 'file;
            }
            // Secondary size guard: protects against metadata races (TOCTOU).
            if size_bytes > config.max_file_size_bytes {
                let stale_file_id = format!("code_file:{}", sha256_short(&rel_path));
                let orphaned = handle_deleted_file(&queries, &rel_path, &stale_file_id).await?;
                result.concerns_orphaned += orphaned;
                warn!(
                    path = %rel_path,
                    size_bytes,
                    limit_bytes = config.max_file_size_bytes,
                    "code graph sync: skipping oversized file (content check)"
                );
                result.oversized_files_skipped += 1;
                break 'file;
            }

            // Language check.
            let lang = language_from_path(file_path);
            if !config.supported_languages.contains(&lang) {
                break 'file;
            }

            // File-level hash comparison (level 1).
            let content_hash = sha256_hex(&source);
            let is_new = !indexed_map.contains_key(&rel_path);

            if !is_new {
                let existing = &indexed_map[&rel_path];
                if existing.content_hash == content_hash {
                    // File unchanged — skip entirely.
                    result.files_unchanged += 1;
                    break 'file;
                }
            }

            // ── File changed or new: collect pre-sync concerns info ─────
            let pre_sync_identities = if is_new {
                Vec::new()
            } else {
                queries.get_symbol_identities_for_file(&rel_path).await?
            };
            let pre_sync_concerns = if is_new {
                Vec::new()
            } else {
                queries.get_concerns_edges_for_file(&rel_path).await?
            };

            // Enrich concerns edges with symbol name + body_hash.
            let enriched_concerns: Vec<_> = pre_sync_concerns
                .into_iter()
                .map(|mut c| {
                    if let Some(ident) = pre_sync_identities.iter().find(|i| i.id == c.symbol_id) {
                        c.symbol_name = ident.name.clone();
                        c.symbol_body_hash = ident.body_hash.clone();
                    }
                    c
                })
                .collect();

            // ── Parse file ──────────────────────────────────────────────
            let source_clone = source.clone();
            let lang_enum = match Language::try_from(lang.as_str()) {
                Ok(l) => l,
                Err(e) => {
                    result.errors.push(FileError {
                        file: rel_path.clone(),
                        error: e.to_string(),
                    });
                    break 'file;
                }
            };
            let parse_result =
                match tokio::task::spawn_blocking(move || parse_source(&source_clone, lang_enum))
                    .await
                {
                    Ok(Ok(pr)) => pr,
                    Ok(Err(e)) => {
                        result.errors.push(FileError {
                            file: rel_path.clone(),
                            error: e.to_string(),
                        });
                        break 'file;
                    }
                    Err(e) => {
                        result.errors.push(FileError {
                            file: rel_path.clone(),
                            error: format!("task join error: {e}"),
                        });
                        break 'file;
                    }
                };

            // ── Upsert code file node ───────────────────────────────────
            let file_id = format!("code_file:{}", sha256_short(&rel_path));
            let code_file = CodeFile {
                id: file_id.clone(),
                path: rel_path.clone(),
                language: lang.clone(),
                size_bytes,
                content_hash: content_hash.clone(),
                last_indexed_at: chrono::Utc::now().to_rfc3339(),
            };
            queries.upsert_code_file(&code_file).await?;

            // Record file hash for offline change detection using the already-computed
            // content_hash and size_bytes — avoids re-reading from disk.
            // Non-fatal: a hash recording failure degrades offline detection but
            // does not invalidate the synced code graph.
            if let Err(e) = crate::services::file_tracker::record_file_hash_precomputed(
                &rel_path,
                &content_hash,
                size_bytes,
                &queries,
            )
            .await
            {
                debug!(
                    error = %e,
                    path = %rel_path,
                    "code graph sync: file hash recording failed — offline detection may report false changes"
                );
            }

            // Clear previous symbols and defines edges for this file.
            // 082.009-T: retract this file's prior calls_resolved_singleton
            // edges and clear its staged calls WHILE the old symbol IDs still
            // exist, before deleting the function metadata — mirroring the
            // full-index and file-deletion paths so an incremental sync never
            // leaves a stale/dangling cross-file edge.
            queries
                .retract_resolved_calls_edges_for_file(&rel_path)
                .await?;
            queries.clear_staged_calls_for_file(&rel_path).await?;
            queries.delete_functions_by_file(&rel_path).await?;
            queries.delete_classes_by_file(&rel_path).await?;
            queries.delete_interfaces_by_file(&rel_path).await?;
            queries.delete_edges_from_file("defines", &file_id).await?;
            queries
                .delete_edges_from_file("references", &file_id)
                .await?;

            // ── Build map of old symbols by (name, body_hash) for reuse ─
            let old_sym_map: HashMap<(String, String), &crate::db::queries::SymbolIdentity> =
                pre_sync_identities
                    .iter()
                    .map(|s| ((s.name.clone(), s.body_hash.clone()), s))
                    .collect();

            // ── Insert new symbols, re-embed only if body changed ───────
            let token_limit = config.embedding.token_limit;
            let mut new_function_ids: Vec<(String, String)> = Vec::new();
            let mut new_class_ids: Vec<(String, String)> = Vec::new();
            let mut new_interface_ids: Vec<(String, String)> = Vec::new();
            let mut embed_texts: Vec<String> = Vec::new();
            let mut embed_ids: Vec<String> = Vec::new();

            // A6: per-file canonical context (Rust-only; None → empty canonical_path).
            let rust_ctx = rust_canonical_ctx(&crates, lang_enum, &rel_path, &source);

            for symbol in &parse_result.symbols {
                match symbol {
                    ExtractedSymbol::Function(f) => {
                        let sym_id = format!("function:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            f.token_count as usize,
                            token_limit,
                            &f.body,
                            &f.signature,
                            f.docstring.as_deref(),
                        );

                        // Check if body_hash matches an old symbol and carry its embedding forward.
                        // Without this, reused symbols would be written with a zero-vector, causing
                        // NaN cosine scores on the next KNN search.
                        let old_embedding = old_sym_map
                            .get(&(f.name.clone(), f.body_hash.clone()))
                            .filter(|s| embedding::has_meaningful_embedding(&s.embedding))
                            .map(|s| s.embedding.clone());

                        let reused = old_embedding.is_some();
                        if reused {
                            result.symbols_reused += 1;
                        } else {
                            embed_texts.push(summary.clone());
                            embed_ids.push(sym_id.clone());
                            result.symbols_reembedded += 1;
                        }

                        let func = crate::models::function::Function {
                            id: sym_id.clone(),
                            name: f.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: f.line_start,
                            line_end: f.line_end,
                            signature: f.signature.clone(),
                            docstring: f.docstring.clone(),
                            body: f.body.clone(),
                            body_hash: f.body_hash.clone(),
                            token_count: f.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: old_embedding
                                .unwrap_or_else(|| vec![0.0_f32; embedding::EMBEDDING_DIM]),
                            summary,
                        };
                        let canonical_path =
                            canonical_path_for_function(&crates, rust_ctx.as_ref(), &f.name);
                        queries
                            .upsert_function_with_canonical(&func, &canonical_path)
                            .await?;
                        new_function_ids.push((f.name.clone(), sym_id.clone()));
                        queries
                            .create_defines_edge(&file_id, "function", &sym_id)
                            .await?;
                    }
                    ExtractedSymbol::Class(c) => {
                        let sym_id = format!("class:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            c.token_count as usize,
                            token_limit,
                            &c.body,
                            "",
                            c.docstring.as_deref(),
                        );

                        let old_embedding = old_sym_map
                            .get(&(c.name.clone(), c.body_hash.clone()))
                            .filter(|s| embedding::has_meaningful_embedding(&s.embedding))
                            .map(|s| s.embedding.clone());

                        let reused = old_embedding.is_some();
                        if reused {
                            result.symbols_reused += 1;
                        } else {
                            embed_texts.push(summary.clone());
                            embed_ids.push(sym_id.clone());
                            result.symbols_reembedded += 1;
                        }

                        let class = crate::models::class::Class {
                            id: sym_id.clone(),
                            name: c.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: c.line_start,
                            line_end: c.line_end,
                            docstring: c.docstring.clone(),
                            body: c.body.clone(),
                            body_hash: c.body_hash.clone(),
                            token_count: c.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: old_embedding
                                .unwrap_or_else(|| vec![0.0_f32; embedding::EMBEDDING_DIM]),
                            summary,
                        };
                        queries.upsert_class(&class).await?;
                        new_class_ids.push((c.name.clone(), sym_id.clone()));
                        queries
                            .create_defines_edge(&file_id, "class", &sym_id)
                            .await?;
                    }
                    ExtractedSymbol::Interface(i) => {
                        let sym_id = format!("interface:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            i.token_count as usize,
                            token_limit,
                            &i.body,
                            "",
                            i.docstring.as_deref(),
                        );

                        let old_embedding = old_sym_map
                            .get(&(i.name.clone(), i.body_hash.clone()))
                            .filter(|s| embedding::has_meaningful_embedding(&s.embedding))
                            .map(|s| s.embedding.clone());

                        let reused = old_embedding.is_some();
                        if reused {
                            result.symbols_reused += 1;
                        } else {
                            embed_texts.push(summary.clone());
                            embed_ids.push(sym_id.clone());
                            result.symbols_reembedded += 1;
                        }

                        let iface = crate::models::interface::Interface {
                            id: sym_id.clone(),
                            name: i.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: i.line_start,
                            line_end: i.line_end,
                            docstring: i.docstring.clone(),
                            body: i.body.clone(),
                            body_hash: i.body_hash.clone(),
                            token_count: i.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: old_embedding
                                .unwrap_or_else(|| vec![0.0_f32; embedding::EMBEDDING_DIM]),
                            summary,
                        };
                        queries.upsert_interface(&iface).await?;
                        new_interface_ids.push((i.name.clone(), sym_id.clone()));
                        queries
                            .create_defines_edge(&file_id, "interface", &sym_id)
                            .await?;
                    }
                }
            }

            // ── Batch embed changed symbols ─────────────────────────────
            if !embed_texts.is_empty() {
                match embedding::embed_texts(&embed_texts) {
                    Ok(vectors) => {
                        for (sym_id, vector) in embed_ids.iter().zip(vectors) {
                            if let Err(e) = queries.update_symbol_embedding(sym_id, vector).await {
                                debug!(error = %e, sym_id = %sym_id, "code graph sync: embedding write-back failed");
                            }
                        }
                        debug!(
                            count = embed_ids.len(),
                            "code graph sync: generated and stored embeddings for changed symbols"
                        );
                    }
                    Err(e) => {
                        debug!(error = %e, "code graph sync: embedding unavailable, skipping");
                    }
                }
            }

            // ── Recreate edges from parse result ────────────────────────
            for edge in &parse_result.edges {
                match edge {
                    ExtractedEdge::Calls {
                        caller,
                        callee,
                        is_method,
                        is_qualified,
                    } => {
                        // Method/receiver and path-qualified calls are extracted
                        // but not promoted (see the index-path arm) to avoid
                        // false singleton edges.
                        if *is_method || *is_qualified {
                            continue;
                        }
                        // Mirror the index-path behavior: resolve locally for a
                        // direct edge, else stage the cross-file call for the
                        // deferred post-pass (082.002-T).
                        match (
                            find_function_id(&new_function_ids, caller),
                            find_function_id(&new_function_ids, callee),
                        ) {
                            (Some(from_id), Some(to_id)) => {
                                queries.create_calls_edge(&from_id, &to_id).await?;
                            }
                            (Some(from_id), None) => {
                                queries.put_staged_call(&from_id, callee, &rel_path).await?;
                            }
                            _ => {}
                        }
                    }
                    ExtractedEdge::InheritsFrom {
                        struct_name,
                        trait_name,
                    } => {
                        if let Some(child_id) = find_class_id(&new_class_ids, struct_name) {
                            if let Some(parent_id) =
                                find_interface_id(&new_interface_ids, trait_name)
                            {
                                queries
                                    .create_inherits_edge(
                                        "class",
                                        &child_id,
                                        "interface",
                                        &parent_id,
                                    )
                                    .await?;
                            }
                        }
                    }
                    // Defines already created above; Imports are cross-file (deferred, counted).
                    ExtractedEdge::Imports { .. } => {
                        result.cross_file_edges_dropped += 1;
                    }
                    // Defines edge already handled during symbol upsert.
                    ExtractedEdge::Defines { .. } => {}
                    // SQL References: resolve target to a Class node or self-loop (033.001-T).
                    ExtractedEdge::References { target, .. } => {
                        let resolved_id = queries.resolve_reference_target(target).await?;
                        if let Some(class_id) = resolved_id {
                            queries
                                .create_references_edge(&file_id, &class_id, Some(target))
                                .await?;
                        } else {
                            queries
                                .create_references_edge(&file_id, &file_id, Some(target))
                                .await?;
                        }
                        result.edges_created += 1;
                    }
                }
            }

            // ── Delete old concerns edges before relinking (prevent duplicates) ──
            for edge in &enriched_concerns {
                let _ = queries
                    .delete_concerns_edges_for_symbol(&edge.symbol_table, &edge.symbol_id)
                    .await;
            }

            // ── Relink concerns edges (FR-124) ──────────────────────────
            let (relinked, orphaned) = relink_concerns_edges(
                &queries,
                &enriched_concerns,
                &new_function_ids,
                &new_class_ids,
                &new_interface_ids,
            )
            .await?;
            result.concerns_relinked += relinked;
            result.concerns_orphaned += orphaned;

            if is_new {
                result.files_added += 1;
            } else {
                result.files_modified += 1;
            }
            debug!(path = %rel_path, "code graph sync: re-indexed file");
        }
        advance_progress(&mut progress, &mut completed_files, total_files);
    }

    // ── Post-pass: re-resolve unresolved references edges ───────────
    // Same ordering issue as index_workspace: a reference may be processed
    // before its target class exists. Re-resolve self-loops now all symbols exist.
    let reresolved = queries.reresolve_references_edges().await?;
    if reresolved.resolved > 0 {
        debug!(
            count = reresolved.resolved,
            "code graph sync: re-resolved deferred references edges"
        );
    }

    // ── Record sync summary ──────────────────────────────────────────
    let sync_summary = format!(
        "Code graph sync: {} modified, {} added, {} deleted, {} unchanged",
        result.files_modified, result.files_added, result.files_deleted, result.files_unchanged,
    );
    info!("{sync_summary}");

    #[allow(clippy::cast_possible_truncation)]
    let elapsed = start.elapsed().as_millis() as u64;
    result.duration_ms = elapsed;

    Ok(result)
}

/// Handle removal of an indexed file from the code graph.
///
/// Used when a file is deleted from disk or when we intentionally evict stale
/// indexed state, such as when a previously indexed file now exceeds the size
/// policy and should disappear from search and graph results.
///
/// Returns the number of concerns edges orphaned.
async fn handle_deleted_file(
    queries: &CodeGraphQueries,
    file_path: &str,
    file_id: &str,
) -> Result<usize, EngramError> {
    // Collect and delete concerns edges targeting symbols in this file.
    let concerns = queries.get_concerns_edges_for_file(file_path).await?;
    let mut orphaned = 0;
    for edge in &concerns {
        let deleted = queries
            .delete_concerns_edges_for_symbol(&edge.symbol_table, &edge.symbol_id)
            .await?;
        orphaned += deleted;
    }

    // Delete all symbol nodes, outbound file edges, and metadata for this file.
    // 082.009-T: retract this file's calls_resolved_singleton edges and clear
    // its staged calls WHILE the symbol IDs still exist, before deleting the
    // function metadata they are keyed against.
    queries
        .retract_resolved_calls_edges_for_file(file_path)
        .await?;
    queries.clear_staged_calls_for_file(file_path).await?;
    queries.delete_functions_by_file(file_path).await?;
    queries.delete_classes_by_file(file_path).await?;
    queries.delete_interfaces_by_file(file_path).await?;
    queries.delete_edges_from_file("defines", file_id).await?;
    queries
        .delete_edges_from_file("references", file_id)
        .await?;
    queries.delete_code_file(file_path).await?;
    queries.delete_file_hash_by_path(file_path).await?;

    if orphaned > 0 {
        warn!(
            file_path,
            orphaned, "code graph: orphaned concerns edges from removed file"
        );
    }

    Ok(orphaned)
}

/// Re-link `concerns` edges after re-indexing a modified file (FR-124).
///
/// For each concerns edge that existed before the re-index:
///   - If a new symbol with the same `(name, body_hash)` exists in ANY file,
///     re-create the concerns edge pointing to the new symbol ID.
///   - If no match is found, the edge is orphaned and removed.
///
/// Returns `(relinked, orphaned)` counts.
async fn relink_concerns_edges(
    queries: &CodeGraphQueries,
    pre_sync_concerns: &[crate::db::queries::ConcernsEdgeInfo],
    new_function_ids: &[(String, String)],
    new_class_ids: &[(String, String)],
    new_interface_ids: &[(String, String)],
) -> Result<(usize, usize), EngramError> {
    let mut relinked = 0;
    let mut orphaned = 0;

    for edge in pre_sync_concerns {
        if edge.symbol_name.is_empty() {
            // Cannot relink without a name — treat as orphan.
            orphaned += 1;
            continue;
        }

        // Try to find the new symbol by (name, body_hash) across all tables.
        let matches = queries
            .find_symbols_by_name_and_hash(&edge.symbol_name, &edge.symbol_body_hash)
            .await?;

        if matches.is_empty() {
            // Also try within the same file's new symbols by name only
            // (the symbol may have been modified, changing its body_hash).
            let in_file_match = match edge.symbol_table.as_str() {
                "function" => find_function_id(new_function_ids, &edge.symbol_name),
                "class" => find_class_id(new_class_ids, &edge.symbol_name),
                "interface" => find_interface_id(new_interface_ids, &edge.symbol_name),
                _ => None,
            };

            if let Some(new_id) = in_file_match {
                // Re-link to the new symbol (same name, different body).
                queries
                    .create_concerns_edge(
                        &edge.task_id,
                        &edge.symbol_table,
                        &new_id,
                        &edge.linked_by,
                    )
                    .await?;
                relinked += 1;
                debug!(
                    task = %edge.task_id,
                    symbol = %edge.symbol_name,
                    "concerns edge re-linked (name match, body changed)"
                );
            } else {
                orphaned += 1;
                warn!(
                    task = %edge.task_id,
                    symbol = %edge.symbol_name,
                    "concerns edge orphaned — symbol no longer exists"
                );
            }
        } else {
            // Re-link to the first matching new symbol.
            let target = &matches[0];
            queries
                .create_concerns_edge(&edge.task_id, &target.table, &target.id, &edge.linked_by)
                .await?;
            relinked += 1;
            debug!(
                task = %edge.task_id,
                symbol = %edge.symbol_name,
                new_path = %target.file_path,
                "concerns edge re-linked via hash-resilient identity"
            );
        }
    }

    Ok((relinked, orphaned))
}

/// Discover all source files in the workspace using `.gitignore`-aware traversal.
fn discover_files(ws_path: &Path, config: &CodeGraphConfig) -> Vec<std::path::PathBuf> {
    let mut builder = ignore::WalkBuilder::new(ws_path);
    builder
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .follow_links(false);

    // Add custom exclude patterns from config using a single OverrideBuilder.
    if !config.exclude_patterns.is_empty() {
        let mut ob = ignore::overrides::OverrideBuilder::new(ws_path);
        for pattern in &config.exclude_patterns {
            // Ignore patterns that fail to parse; log and continue.
            if ob.add(&format!("!{pattern}")).is_err() {
                warn!(pattern = %pattern, "code graph: invalid exclude pattern, skipping");
            }
        }
        match ob.build() {
            Ok(overrides) => {
                builder.overrides(overrides);
            }
            Err(e) => {
                warn!(error = %e, "code graph: failed to build exclude overrides, patterns ignored");
            }
        }
    }

    let supported: std::collections::HashSet<&str> = config
        .supported_languages
        .iter()
        .map(String::as_str)
        .collect();

    let mut files = Vec::new();
    for entry in builder.build().flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let lang = language_from_path(path);
        if supported.contains(lang.as_str()) {
            files.push(path.to_path_buf());
        }
    }

    files.sort();
    files
}

/// Map a file extension to a language identifier.
///
/// This is the **canonical** language vocabulary: the indexer stores the result
/// in `file_node.language`, and the retrieval-eval semantic gate
/// ([`crate::services::retrieval_eval::language_of`]) delegates here so the
/// semantic and graph gates never diverge (084.005-T and its generalization).
/// Unrecognized extensions fall through to the raw extension; a path with no
/// extension is `"unknown"`.
pub(crate) fn language_from_path(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| match ext {
            "rs" => "rust",
            "py" => "python",
            "js" | "jsx" => "javascript",
            "ts" => "typescript",
            "tsx" => "tsx",
            "go" => "go",
            "cs" => "csharp",
            "c" | "h" => "c",
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "h++" => "cpp",
            "swift" => "swift",
            "sql" => "sql",
            "kt" | "kts" => "kotlin",
            "md" => "markdown",
            _ => ext,
        })
        .unwrap_or("unknown")
        .to_owned()
}

/// Determine tier classification based on token count.
///
/// Returns `(embed_type, summary_text)`.
fn tier_classification(
    token_count: usize,
    token_limit: usize,
    body: &str,
    signature: &str,
    docstring: Option<&str>,
) -> (&'static str, String) {
    if token_count <= token_limit {
        ("explicit_code", body.to_owned())
    } else {
        let summary = match docstring {
            Some(doc) if !doc.is_empty() => format!("{signature}\n\n{doc}"),
            _ => {
                // No docstring: include first 5 lines / 256 chars of body as preview.
                let preview: String = body.lines().take(5).collect::<Vec<_>>().join("\n");
                let preview = if preview.len() > 256 {
                    // Safe truncation at char boundary.
                    let end = preview
                        .char_indices()
                        .nth(256)
                        .map_or(preview.len(), |(i, _)| i);
                    &preview[..end]
                } else {
                    &preview
                };
                if preview.is_empty() {
                    signature.to_owned()
                } else {
                    format!("{signature}\n\n{preview}")
                }
            }
        };
        ("summary_pointer", summary)
    }
}

/// SHA-256 hex digest of a string.
fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Short SHA-256 for IDs (first 16 hex chars).
fn sha256_short(input: &str) -> String {
    sha256_hex(input)[..16].to_owned()
}

/// Find a function ID by name.
fn find_function_id(ids: &[(String, String)], name: &str) -> Option<String> {
    ids.iter()
        .find(|(n, _)| n == name)
        .map(|(_, id)| id.clone())
}

/// Find a class ID by name.
fn find_class_id(ids: &[(String, String)], name: &str) -> Option<String> {
    ids.iter()
        .find(|(n, _)| n == name)
        .map(|(_, id)| id.clone())
}

/// Find an interface ID by name.
fn find_interface_id(ids: &[(String, String)], name: &str) -> Option<String> {
    ids.iter()
        .find(|(n, _)| n == name)
        .map(|(_, id)| id.clone())
}
