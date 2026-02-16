//! Class model — a type definition (struct in Rust) extracted via AST parsing.

use serde::{Deserialize, Serialize};

/// A type definition (struct in Rust) extracted from a source file.
///
/// In Rust, `struct_item` nodes map to `class` entities. Unlike `Function`,
/// `Class` does not have a `signature` field because struct definitions
/// do not have callable signatures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Class {
    /// SurrealDB record ID (format: `class:<ulid>`).
    pub id: String,
    /// Class/struct name.
    pub name: String,
    /// Workspace-relative path of containing file.
    pub file_path: String,
    /// 1-based start line.
    pub line_start: u32,
    /// 1-based end line (inclusive).
    pub line_end: u32,
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
    /// Summary text.
    pub summary: String,
}
