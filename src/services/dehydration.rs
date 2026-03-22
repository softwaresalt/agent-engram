//! Dehydration: serialize workspace state from SurrealDB to `.engram/` files.
//!
//! Produces JSONL code-graph files and backlog JSON that can be committed to
//! Git. Task-specific serialization (tasks.md, graph.surql, comments.md) has
//! been removed; this module now serves pure code-intelligence dehydration.

use std::path::Path;

use tokio::io::AsyncWriteExt as _;
use tokio::sync::{Mutex, MutexGuard};

use crate::db::{self};
use crate::errors::{EngramError, SystemError};
use crate::server::state::SharedState;

/// Global flush lock for serializing concurrent dehydration operations (ADR-0002).
static FLUSH_LOCK: Mutex<()> = Mutex::const_new(());

/// Acquire the per-process flush lock for concurrent dehydration serialization.
///
/// Returns a guard that releases the lock when dropped.
pub async fn acquire_flush_lock() -> MutexGuard<'static, ()> {
    FLUSH_LOCK.lock().await
}

/// Flush all active workspaces to `.engram/` files (FR-006 graceful shutdown).
///
/// Acquires the flush lock and dehydrates the active workspace, if any.
pub async fn flush_all_workspaces(state: &SharedState) -> Result<(), EngramError> {
    let _guard = acquire_flush_lock().await;
    let Some(snapshot) = state.snapshot_workspace().await else {
        return Ok(());
    };

    let workspace_path = Path::new(&snapshot.path);
    let db = db::connect_db(&snapshot.data_dir, &snapshot.branch).await?;
    let cg_queries = crate::db::queries::CodeGraphQueries::new(db);

    dehydrate_code_graph(&cg_queries, workspace_path).await?;
    Ok(())
}

/// Schema version written to `.engram/.version`.
///
/// This is a **semantic schema version**, not the crate version. It MUST only
/// be incremented when the on-disk `.engram/` file format changes in a way
/// that is incompatible with previous readers. Tying it to `CARGO_PKG_VERSION`
/// would invalidate every existing workspace on every release.
pub const SCHEMA_VERSION: &str = "3.0.0";

/// Result of a code graph dehydration operation.
#[derive(Debug, Clone)]
pub struct CodeGraphDehydrationResult {
    /// Files written during code graph serialization.
    pub files_written: Vec<String>,
    /// Total nodes written (code_files + functions + classes + interfaces).
    pub nodes_written: usize,
    /// Total edges written.
    pub edges_written: usize,
}

/// Dehydrate code graph state to `.engram/code-graph/` JSONL files (FR-132, FR-133, FR-134).
///
/// Writes `nodes.jsonl` and `edges.jsonl` using atomic temp-file-then-rename.
/// Only writes files when there is data; removes stale files when the graph is empty.
pub async fn dehydrate_code_graph(
    cg_queries: &crate::db::queries::CodeGraphQueries,
    workspace_path: &Path,
) -> Result<CodeGraphDehydrationResult, EngramError> {
    let code_graph_dir = workspace_path.join(".engram").join("code-graph");
    tokio::fs::create_dir_all(&code_graph_dir)
        .await
        .map_err(|_| flush_err(&code_graph_dir))?;

    let code_files = cg_queries.list_code_files().await?;
    let functions = cg_queries.all_functions().await?;
    let classes = cg_queries.all_classes().await?;
    let interfaces = cg_queries.all_interfaces().await?;
    let edges = cg_queries.all_code_edges().await?;

    let total_nodes = code_files.len() + functions.len() + classes.len() + interfaces.len();
    let total_edges = edges.len();
    let mut files_written = Vec::new();

    // Serialize nodes.jsonl
    let nodes_path = code_graph_dir.join("nodes.jsonl");
    if total_nodes > 0 {
        let nodes_content = serialize_nodes_jsonl(&code_files, &functions, &classes, &interfaces);
        atomic_write(&nodes_path, &nodes_content).await?;
        files_written.push(".engram/code-graph/nodes.jsonl".to_string());
    } else if tokio::fs::try_exists(&nodes_path).await.unwrap_or(false) {
        let _ = tokio::fs::remove_file(&nodes_path).await;
    }

    // Serialize edges.jsonl
    let edges_path = code_graph_dir.join("edges.jsonl");
    if total_edges > 0 {
        let edges_content = serialize_edges_jsonl(&edges);
        atomic_write(&edges_path, &edges_content).await?;
        files_written.push(".engram/code-graph/edges.jsonl".to_string());
    } else if tokio::fs::try_exists(&edges_path).await.unwrap_or(false) {
        let _ = tokio::fs::remove_file(&edges_path).await;
    }

    Ok(CodeGraphDehydrationResult {
        files_written,
        nodes_written: total_nodes,
        edges_written: total_edges,
    })
}

/// Write content atomically using temp file + rename pattern.
///
/// Creates a temporary `.tmp` file alongside the target, writes content,
/// then renames atomically to prevent partial writes on crash.
///
/// # Atomic write pattern (T043)
///
/// Writes to `.<filename>.tmp` (same directory, so same filesystem) then
/// calls `rename` to replace the target in a single atomic kernel operation.
/// This prevents partial writes from corrupting workspace state during
/// crashes or concurrent access.
pub async fn atomic_write(path: &Path, content: &str) -> Result<(), EngramError> {
    let tmp_path = path.with_extension("tmp");
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|_| flush_err(path))?;
    file.write_all(content.as_bytes())
        .await
        .map_err(|_| flush_err(path))?;
    file.sync_all().await.map_err(|_| flush_err(path))?;
    drop(file);

    tokio::fs::rename(&tmp_path, path)
        .await
        .map_err(|_| flush_err(path))?;

    Ok(())
}

fn flush_err(path: &Path) -> EngramError {
    EngramError::System(SystemError::FlushFailed {
        path: path.display().to_string(),
    })
}

// ── Code graph JSONL serialization (FR-132, FR-133, FR-134) ───────────────

use crate::models::{Class, CodeEdge, CodeFile, Function, Interface};

/// Intermediate struct for serializing a code graph node to one JSONL line.
///
/// Bodies are stripped (FR-133, FR-134) — only `body_hash` is persisted.
#[derive(Debug, serde::Serialize)]
struct NodeLine {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    name: String,
    file_path: String,
    line_start: u32,
    line_end: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    docstring: Option<String>,
    body_hash: String,
    token_count: u32,
    embed_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    embedding: Option<Vec<f32>>,
    summary: String,
}

/// Intermediate struct for serializing a code edge to one JSONL line.
#[derive(Debug, serde::Serialize)]
struct EdgeLine {
    #[serde(rename = "type")]
    edge_type: String,
    from: String,
    to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    import_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    linked_by: Option<String>,
    created_at: String,
}

/// Intermediate struct for serializing a `CodeFile` to one JSONL line.
#[derive(Debug, serde::Serialize)]
struct FileLine {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    path: String,
    language: String,
    size_bytes: u64,
    content_hash: String,
    last_indexed_at: String,
}

/// Returns `true` if `e` contains at least one non-zero component.
///
/// Used to distinguish a real embedding from the zero-vector placeholder that is
/// emitted when the `embeddings` feature is disabled or the model is unavailable.
fn is_meaningful_embedding(e: &[f32]) -> bool {
    !e.is_empty() && e.iter().any(|&v| v != 0.0)
}

/// Serialize code graph nodes (code_files + functions + classes + interfaces) to JSONL.
///
/// Each line is a self-contained JSON object sorted by `id`.
/// Bodies are excluded; only `body_hash` is persisted (FR-133).
pub fn serialize_nodes_jsonl(
    code_files: &[CodeFile],
    functions: &[Function],
    classes: &[Class],
    interfaces: &[Interface],
) -> String {
    let mut lines: Vec<String> = Vec::new();

    // Code files
    for cf in code_files {
        let fl = FileLine {
            id: cf.id.clone(),
            node_type: "code_file".to_string(),
            path: cf.path.clone(),
            language: cf.language.clone(),
            size_bytes: cf.size_bytes,
            content_hash: cf.content_hash.clone(),
            last_indexed_at: cf.last_indexed_at.clone(),
        };
        if let Ok(json) = serde_json::to_string(&fl) {
            lines.push(json);
        }
    }

    // Functions
    for f in functions {
        let nl = NodeLine {
            id: f.id.clone(),
            node_type: "function".to_string(),
            name: f.name.clone(),
            file_path: f.file_path.clone(),
            line_start: f.line_start,
            line_end: f.line_end,
            signature: Some(f.signature.clone()),
            docstring: f.docstring.clone(),
            body_hash: f.body_hash.clone(),
            token_count: f.token_count,
            embed_type: f.embed_type.clone(),
            embedding: if is_meaningful_embedding(&f.embedding) {
                Some(f.embedding.clone())
            } else {
                None
            },
            summary: f.summary.clone(),
        };
        if let Ok(json) = serde_json::to_string(&nl) {
            lines.push(json);
        }
    }

    // Classes
    for c in classes {
        let nl = NodeLine {
            id: c.id.clone(),
            node_type: "class".to_string(),
            name: c.name.clone(),
            file_path: c.file_path.clone(),
            line_start: c.line_start,
            line_end: c.line_end,
            signature: None,
            docstring: c.docstring.clone(),
            body_hash: c.body_hash.clone(),
            token_count: c.token_count,
            embed_type: c.embed_type.clone(),
            embedding: if is_meaningful_embedding(&c.embedding) {
                Some(c.embedding.clone())
            } else {
                None
            },
            summary: c.summary.clone(),
        };
        if let Ok(json) = serde_json::to_string(&nl) {
            lines.push(json);
        }
    }

    // Interfaces
    for i in interfaces {
        let nl = NodeLine {
            id: i.id.clone(),
            node_type: "interface".to_string(),
            name: i.name.clone(),
            file_path: i.file_path.clone(),
            line_start: i.line_start,
            line_end: i.line_end,
            signature: None,
            docstring: i.docstring.clone(),
            body_hash: i.body_hash.clone(),
            token_count: i.token_count,
            embed_type: i.embed_type.clone(),
            embedding: if is_meaningful_embedding(&i.embedding) {
                Some(i.embedding.clone())
            } else {
                None
            },
            summary: i.summary.clone(),
        };
        if let Ok(json) = serde_json::to_string(&nl) {
            lines.push(json);
        }
    }

    // Sort all lines by id for deterministic output
    lines.sort();

    if lines.is_empty() {
        String::new()
    } else {
        let mut out = lines.join("\n");
        out.push('\n');
        out
    }
}

/// Serialize code edges to JSONL sorted by (type, from, to).
pub fn serialize_edges_jsonl(edges: &[CodeEdge]) -> String {
    let mut lines: Vec<(String, String, String, String)> = Vec::new();

    for edge in edges {
        let el = EdgeLine {
            edge_type: edge.edge_type.as_str().to_string(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            import_path: edge.import_path.clone(),
            linked_by: edge.linked_by.clone(),
            created_at: edge.created_at.clone(),
        };
        if let Ok(json) = serde_json::to_string(&el) {
            lines.push((
                edge.edge_type.as_str().to_string(),
                edge.from.clone(),
                edge.to.clone(),
                json,
            ));
        }
    }

    // Sort by (type, from, to)
    lines.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

    if lines.is_empty() {
        String::new()
    } else {
        let sorted: Vec<String> = lines.into_iter().map(|(_, _, _, json)| json).collect();
        let mut out = sorted.join("\n");
        out.push('\n');
        out
    }
}

// ── SpecKit backlog dehydration (006-workspace-content-intelligence) ──────────

use crate::models::backlog::{BacklogFile, ProjectManifest};

/// Write all backlog JSON files and the project manifest to `.engram/`.
///
/// Each [`BacklogFile`] is serialized to `.engram/backlog-NNN.json`.
/// The [`ProjectManifest`] is serialized to `.engram/project.json`.
/// Uses atomic temp-file-then-rename writes per Constitution VI.
pub async fn dehydrate_backlogs(
    workspace_path: &Path,
    backlogs: &[BacklogFile],
    manifest: &ProjectManifest,
) -> Result<usize, EngramError> {
    let engram_dir = workspace_path.join(".engram");
    tokio::fs::create_dir_all(&engram_dir)
        .await
        .map_err(|_| flush_err(&engram_dir))?;

    let mut written = 0;

    for backlog in backlogs {
        let filename = format!("backlog-{}.json", backlog.id);
        let path = engram_dir.join(&filename);
        let json = serde_json::to_string_pretty(backlog).map_err(|e| {
            EngramError::System(SystemError::FlushFailed {
                path: format!("serialize {filename}: {e}"),
            })
        })?;
        atomic_write(&path, &json).await?;
        written += 1;
    }

    // Write project manifest.
    let manifest_path = engram_dir.join("project.json");
    let manifest_json = serde_json::to_string_pretty(manifest).map_err(|e| {
        EngramError::System(SystemError::FlushFailed {
            path: format!("serialize project.json: {e}"),
        })
    })?;
    atomic_write(&manifest_path, &manifest_json).await?;

    Ok(written)
}

/// Update a single backlog JSON file after a task change.
///
/// Reads the existing backlog, replaces the artifacts section with
/// fresh content from the spec directory, and writes it back atomically.
/// If the spec directory no longer exists (S041), the existing JSON
/// is preserved unchanged.
pub async fn update_backlog_for_feature(
    workspace_path: &Path,
    feature_id: &str,
) -> Result<bool, EngramError> {
    let engram_dir = workspace_path.join(".engram");
    let backlog_path = engram_dir.join(format!("backlog-{feature_id}.json"));

    if !backlog_path.exists() {
        return Ok(false);
    }

    // Read existing backlog.
    let content = tokio::fs::read_to_string(&backlog_path)
        .await
        .map_err(|_| flush_err(&backlog_path))?;
    let mut backlog: BacklogFile = serde_json::from_str(&content).map_err(|e| {
        EngramError::System(SystemError::FlushFailed {
            path: format!("parse {}: {e}", backlog_path.display()),
        })
    })?;

    // Check if the spec directory still exists.
    let spec_dir = workspace_path.join(&backlog.spec_path);
    if !spec_dir.is_dir() {
        tracing::warn!(
            feature_id,
            "Spec directory no longer exists; preserving existing backlog JSON"
        );
        return Ok(false);
    }

    // Refresh artifacts from disk.
    backlog.artifacts = crate::services::hydration::read_speckit_artifacts_pub(&spec_dir);

    // Write back atomically.
    let json = serde_json::to_string_pretty(&backlog).map_err(|e| {
        EngramError::System(SystemError::FlushFailed {
            path: format!("serialize backlog-{feature_id}.json: {e}"),
        })
    })?;
    atomic_write(&backlog_path, &json).await?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn atomic_write_creates_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.txt");
        atomic_write(&path, "hello").await.expect("write");
        let content = tokio::fs::read_to_string(&path).await.expect("read");
        assert_eq!(content, "hello");
    }

    #[test]
    fn serialize_nodes_jsonl_empty() {
        let out = serialize_nodes_jsonl(&[], &[], &[], &[]);
        assert_eq!(out, "");
    }

    #[test]
    fn serialize_nodes_jsonl_function_excludes_body() {
        let f = Function {
            id: "function:abc".to_string(),
            name: "my_fn".to_string(),
            file_path: "src/lib.rs".to_string(),
            line_start: 1,
            line_end: 10,
            signature: "fn my_fn()".to_string(),
            docstring: None,
            body: "fn my_fn() { /* secret */ }".to_string(),
            body_hash: "abcdef".to_string(),
            token_count: 6,
            embed_type: "explicit_code".to_string(),
            embedding: vec![0.1, 0.2],
            summary: "A test function".to_string(),
        };
        let out = serialize_nodes_jsonl(&[], &[f], &[], &[]);
        // Must contain body_hash but NOT the raw body
        assert!(out.contains("\"body_hash\":\"abcdef\""));
        assert!(!out.contains("secret"));
        assert!(out.contains("\"type\":\"function\""));
        assert!(out.contains("\"signature\":\"fn my_fn()\""));
    }

    #[test]
    fn serialize_nodes_jsonl_sorted_by_id() {
        let f1 = Function {
            id: "function:zzz".to_string(),
            name: "z".to_string(),
            file_path: "a.rs".to_string(),
            line_start: 1,
            line_end: 1,
            signature: String::new(),
            docstring: None,
            body: String::new(),
            body_hash: String::new(),
            token_count: 0,
            embed_type: String::new(),
            embedding: vec![],
            summary: String::new(),
        };
        let f2 = Function {
            id: "function:aaa".to_string(),
            ..f1.clone()
        };
        let out = serialize_nodes_jsonl(&[], &[f1, f2], &[], &[]);
        let lines: Vec<&str> = out.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"id\":\"function:aaa\""));
        assert!(lines[1].contains("\"id\":\"function:zzz\""));
    }

    #[test]
    fn serialize_edges_jsonl_empty() {
        let out = serialize_edges_jsonl(&[]);
        assert_eq!(out, "");
    }

    #[test]
    fn serialize_edges_jsonl_sorted_by_type_from_to() {
        use crate::models::code_edge::{CodeEdge, CodeEdgeType};
        let edges = vec![
            CodeEdge {
                edge_type: CodeEdgeType::Imports,
                from: "code_file:b".to_string(),
                to: "code_file:a".to_string(),
                import_path: Some("crate::a".to_string()),
                linked_by: None,
                created_at: String::new(),
            },
            CodeEdge {
                edge_type: CodeEdgeType::Calls,
                from: "function:x".to_string(),
                to: "function:y".to_string(),
                import_path: None,
                linked_by: None,
                created_at: String::new(),
            },
        ];
        let out = serialize_edges_jsonl(&edges);
        let lines: Vec<&str> = out.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        // calls < imports alphabetically
        assert!(lines[0].contains("\"type\":\"calls\""));
        assert!(lines[1].contains("\"type\":\"imports\""));
        assert!(lines[1].contains("\"import_path\":\"crate::a\""));
    }

    #[test]
    fn serialize_nodes_jsonl_code_file() {
        let cf = CodeFile {
            id: "code_file:f1".to_string(),
            path: "src/main.rs".to_string(),
            language: "rust".to_string(),
            size_bytes: 1024,
            content_hash: "hash123".to_string(),
            last_indexed_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let out = serialize_nodes_jsonl(&[cf], &[], &[], &[]);
        assert!(out.contains("\"type\":\"code_file\""));
        assert!(out.contains("\"path\":\"src/main.rs\""));
        assert!(out.contains("\"size_bytes\":1024"));
    }

    // ── GAP-002: zero-vector null serialization tests ───────────────

    #[test]
    fn is_meaningful_embedding_rejects_empty() {
        assert!(!is_meaningful_embedding(&[]));
    }

    #[test]
    fn is_meaningful_embedding_rejects_all_zeros() {
        assert!(!is_meaningful_embedding(&vec![0.0_f32; 384]));
    }

    #[test]
    fn is_meaningful_embedding_accepts_nonzero() {
        let mut e = vec![0.0_f32; 384];
        e[100] = 0.01;
        assert!(is_meaningful_embedding(&e));
    }

    #[test]
    fn zero_embedding_serializes_as_null() {
        let f = Function {
            id: "function:test".to_string(),
            name: "f".to_string(),
            file_path: "src/lib.rs".to_string(),
            line_start: 1,
            line_end: 3,
            signature: "fn f()".to_string(),
            docstring: None,
            body: String::new(),
            body_hash: "abc".to_string(),
            token_count: 0,
            embed_type: "summary_pointer".to_string(),
            embedding: vec![0.0_f32; 384], // all zeros — placeholder
            summary: "fn f()".to_string(),
        };
        let out = serialize_nodes_jsonl(&[], &[f], &[], &[]);
        // Zero embedding must be omitted (skip_serializing_if = "Option::is_none")
        assert!(
            !out.contains("\"embedding\""),
            "zero embedding must be omitted from JSONL, got: {out}"
        );
    }

    #[test]
    fn non_zero_embedding_serializes_as_array() {
        let mut emb = vec![0.0_f32; 384];
        emb[0] = 0.5;
        let f = Function {
            id: "function:test2".to_string(),
            name: "g".to_string(),
            file_path: "src/lib.rs".to_string(),
            line_start: 5,
            line_end: 7,
            signature: "fn g()".to_string(),
            docstring: None,
            body: String::new(),
            body_hash: "def".to_string(),
            token_count: 0,
            embed_type: "explicit_code".to_string(),
            embedding: emb,
            summary: "fn g()".to_string(),
        };
        let out = serialize_nodes_jsonl(&[], &[f], &[], &[]);
        assert!(
            out.contains("\"embedding\""),
            "non-zero embedding must be present in JSONL, got: {out}"
        );
    }

    #[test]
    fn empty_vec_embedding_serializes_as_null() {
        let f = Function {
            id: "function:empty".to_string(),
            name: "h".to_string(),
            file_path: "src/lib.rs".to_string(),
            line_start: 1,
            line_end: 1,
            signature: String::new(),
            docstring: None,
            body: String::new(),
            body_hash: String::new(),
            token_count: 0,
            embed_type: String::new(),
            embedding: vec![], // empty
            summary: String::new(),
        };
        let out = serialize_nodes_jsonl(&[], &[f], &[], &[]);
        assert!(
            !out.contains("\"embedding\""),
            "empty embedding must be omitted from JSONL"
        );
    }
}
