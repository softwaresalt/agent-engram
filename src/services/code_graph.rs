//! Code graph indexing orchestration service.
//!
//! Coordinates file discovery, parallel parsing, tiered embedding,
//! incremental sync, and concerns edge management.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info};
use uuid::Uuid;

use crate::db::connect_db;
use crate::db::queries::CodeGraphQueries;
use crate::errors::EngramError;
use crate::models::code_file::CodeFile;
use crate::models::config::CodeGraphConfig;
use crate::services::embedding;
use crate::services::parsing::{ExtractedEdge, ExtractedSymbol, parse_rust_source};

/// Summary returned by [`index_workspace`] after indexing completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    /// Number of source files successfully parsed.
    pub files_parsed: usize,
    /// Number of files skipped (unsupported, too large, or unchanged).
    pub files_skipped: usize,
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

/// Discover, parse, and index all supported source files in the workspace.
///
/// Uses the `ignore` crate for .gitignore-aware file traversal, filters by
/// supported languages and file size, parses via tree-sitter, assigns tiered
/// embeddings, and persists all nodes and edges to SurrealDB.
///
/// # Errors
///
/// Returns `EngramError` on database connection failure or fatal I/O errors.
/// Per-file parse errors are collected in `IndexResult::errors` (non-fatal).
pub async fn index_workspace(
    ws_path: &Path,
    ws_id: &str,
    config: &CodeGraphConfig,
    force: bool,
) -> Result<IndexResult, EngramError> {
    let start = std::time::Instant::now();

    let db = connect_db(ws_id).await?;
    let queries = CodeGraphQueries::new(db);

    // ── Step 1: Discover files ──────────────────────────────────────
    let files = discover_files(ws_path, config);
    info!(
        files_found = files.len(),
        "code graph: discovered source files"
    );

    let mut result = IndexResult {
        files_parsed: 0,
        files_skipped: 0,
        functions_indexed: 0,
        classes_indexed: 0,
        interfaces_indexed: 0,
        edges_created: 0,
        embeddings_generated: 0,
        tier1_count: 0,
        tier2_count: 0,
        errors: Vec::new(),
        duration_ms: 0,
    };

    // ── Step 2: Process each file ───────────────────────────────────
    for file_path in &files {
        let rel_path = file_path
            .strip_prefix(ws_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");

        // Read file contents.
        let source = match tokio::fs::read_to_string(file_path).await {
            Ok(s) => s,
            Err(e) => {
                result.errors.push(FileError {
                    file: rel_path.clone(),
                    error: format!("read error: {e}"),
                });
                result.files_skipped += 1;
                continue;
            }
        };

        // Check file size.
        let size_bytes = source.len() as u64;
        if size_bytes > config.max_file_size_bytes {
            result.errors.push(FileError {
                file: rel_path.clone(),
                error: format!(
                    "file too large ({size_bytes} > {} bytes)",
                    config.max_file_size_bytes
                ),
            });
            result.files_skipped += 1;
            continue;
        }

        // Compute content hash.
        let content_hash = sha256_hex(&source);

        // Skip unchanged files (unless force).
        if !force {
            if let Ok(Some(existing)) = queries.get_code_file_by_path(&rel_path).await {
                if existing.content_hash == content_hash {
                    debug!(path = %rel_path, "code graph: skipping unchanged file");
                    result.files_skipped += 1;
                    continue;
                }
            }
        }

        // Detect language from extension.
        let lang = language_from_path(file_path);
        if !config.supported_languages.contains(&lang) {
            result.files_skipped += 1;
            continue;
        }

        // ── Parse via tree-sitter (CPU-bound, run in blocking task) ─
        let source_clone = source.clone();
        let parse_result =
            match tokio::task::spawn_blocking(move || parse_rust_source(&source_clone)).await {
                Ok(Ok(pr)) => pr,
                Ok(Err(e)) => {
                    result.errors.push(FileError {
                        file: rel_path.clone(),
                        error: e,
                    });
                    result.files_skipped += 1;
                    continue;
                }
                Err(e) => {
                    result.errors.push(FileError {
                        file: rel_path.clone(),
                        error: format!("task join error: {e}"),
                    });
                    result.files_skipped += 1;
                    continue;
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
        queries.delete_functions_by_file(&rel_path).await?;
        queries.delete_classes_by_file(&rel_path).await?;
        queries.delete_interfaces_by_file(&rel_path).await?;
        queries.delete_edges_from_file("defines", &file_id).await?;

        // ── Collect symbols for embedding ───────────────────────────
        let token_limit = config.embedding.token_limit;
        let mut embed_texts: Vec<String> = Vec::new();
        let mut embed_indices: Vec<usize> = Vec::new();

        // Track symbol IDs for edge creation.
        let mut function_ids: Vec<(String, String)> = Vec::new(); // (name, id)
        let mut class_ids: Vec<(String, String)> = Vec::new();
        let mut interface_ids: Vec<(String, String)> = Vec::new();

        for (idx, symbol) in parse_result.symbols.iter().enumerate() {
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
                    embed_indices.push(idx);

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
                        embedding: Vec::new(), // placeholder until batch embed
                        summary,
                    };
                    queries.upsert_function(&func).await?;
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
                    embed_indices.push(idx);

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
                        embedding: Vec::new(),
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
                    embed_indices.push(idx);

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
                        embedding: Vec::new(),
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
                    // TODO: Update symbol records with embedding vectors.
                    // This requires a set_embedding query per symbol type,
                    // which will be added when embeddings feature is active.
                    debug!(
                        count = vectors.len(),
                        "code graph: generated embeddings for file"
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
                ExtractedEdge::Calls { caller, callee } => {
                    // Resolve names to IDs within this file's symbols.
                    if let (Some(from_id), Some(to_id)) = (
                        find_function_id(&function_ids, caller),
                        find_function_id(&function_ids, callee),
                    ) {
                        queries.create_calls_edge(&from_id, &to_id).await?;
                        result.edges_created += 1;
                    }
                }
                ExtractedEdge::InheritsFrom {
                    struct_name,
                    trait_name,
                } => {
                    if let Some(child_id) = find_class_id(&class_ids, struct_name) {
                        if let Some(parent_id) = find_interface_id(&interface_ids, trait_name) {
                            queries
                                .create_inherits_edge("class", &child_id, "interface", &parent_id)
                                .await?;
                            result.edges_created += 1;
                        }
                    }
                }
                // Imports are cross-file (deferred); Defines already created above.
                ExtractedEdge::Imports { .. } | ExtractedEdge::Defines { .. } => {}
            }
        }

        result.files_parsed += 1;
        debug!(path = %rel_path, "code graph: indexed file");
    }

    #[allow(clippy::cast_possible_truncation)]
    let elapsed = start.elapsed().as_millis() as u64;
    result.duration_ms = elapsed;
    info!(
        files_parsed = result.files_parsed,
        files_skipped = result.files_skipped,
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

/// Discover all source files in the workspace using `.gitignore`-aware traversal.
fn discover_files(ws_path: &Path, config: &CodeGraphConfig) -> Vec<std::path::PathBuf> {
    let mut builder = ignore::WalkBuilder::new(ws_path);
    builder
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);

    // Add custom exclude patterns from config.
    for pattern in &config.exclude_patterns {
        let glob = ignore::overrides::OverrideBuilder::new(ws_path)
            .add(&format!("!{pattern}"))
            .and_then(|b| b.build());
        if let Ok(overrides) = glob {
            builder.overrides(overrides);
            break; // WalkBuilder only supports one override set
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
fn language_from_path(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| match ext {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
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
            _ => signature.to_owned(),
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
