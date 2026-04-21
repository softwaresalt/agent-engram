//! Tree-sitter Swift grammar parser.
//!
//! Extracts top-level and member functions, class/struct/actor declarations,
//! protocol declarations, and `import` edges from Swift source files.
//!
//! # Status
//!
//! This is a no-op stub pending ABI 14 verification (task A-1, 027.002-T).
//! Once ABI is confirmed, grammar loading and extraction are implemented in
//! task A-2 (027.003-T).
//!
//! # Planned node kinds (pending node-types.json validation in A-2)
//!
//! - `function_declaration` / `init_declaration` → [`super::ExtractedSymbol::Function`]
//! - `class_declaration` / `struct_declaration` / `actor_declaration` → [`super::ExtractedSymbol::Class`]
//! - `protocol_declaration` → [`super::ExtractedSymbol::Interface`]
//! - `import_declaration` → [`super::ExtractedEdge::Imports`]
//! - `call_expression` → [`super::ExtractedEdge::Calls`]

use super::ParseResult;

/// Parse a Swift source file and extract symbols and edges.
///
/// This is a no-op stub returning an empty [`ParseResult`]. ABI compatibility
/// with tree-sitter 0.24 is verified in task A-1 (027.002-T); full extraction
/// is implemented in task A-2 (027.003-T).
///
/// # Errors
///
/// Currently infallible. Will return errors once grammar loading is added in A-1.
pub(super) fn parse_swift_source(
    _source: &str,
) -> Result<ParseResult, crate::errors::EngramError> {
    // No-op stub — grammar loading pending ABI verification in task A-1 (027.002-T).
    Ok(ParseResult {
        symbols: vec![],
        edges: vec![],
    })
}
