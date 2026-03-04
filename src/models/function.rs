//! Function model — a callable code unit extracted via AST parsing.

use serde::{Deserialize, Serialize};

/// A callable code unit extracted from a source file via AST parsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Function {
    /// SurrealDB record ID (format: `function:<ulid>`).
    pub id: String,
    /// Function name.
    pub name: String,
    /// Workspace-relative path of containing file.
    pub file_path: String,
    /// 1-based start line.
    pub line_start: u32,
    /// 1-based end line (inclusive).
    pub line_end: u32,
    /// Full function signature.
    pub signature: String,
    /// Doc comment text if present.
    pub docstring: Option<String>,
    /// Full source body (populated at runtime, not persisted).
    pub body: String,
    /// SHA-256 hex digest of source body.
    pub body_hash: String,
    /// Estimated token count (character-based: body length / 4).
    pub token_count: u32,
    /// `"explicit_code"` (Tier 1) or `"summary_pointer"` (Tier 2).
    pub embed_type: String,
    /// 384-dimensional embedding vector.
    pub embedding: Vec<f32>,
    /// Summary text (= body for Tier 1, = signature+docstring for Tier 2).
    pub summary: String,
}
