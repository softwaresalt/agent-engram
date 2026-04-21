//! Tree-sitter Kotlin grammar parser.
//!
//! Extracts top-level and member functions, class/data-class/sealed-class
//! declarations, interface declarations, and `import` edges from Kotlin
//! source files.
//!
//! # Status
//!
//! This is a no-op stub pending ABI 14 verification (task B-1, 027.005-T).
//! Once ABI is confirmed, grammar loading and extraction are implemented in
//! task B-2 (027.006-T).
//!
//! # Planned node kinds (pending node-types.json validation in B-2)
//!
//! - `function_declaration` → [`super::ExtractedSymbol::Function`]
//! - `class_declaration` (incl. data class, sealed class) → [`super::ExtractedSymbol::Class`]
//! - `interface_declaration` → [`super::ExtractedSymbol::Interface`]
//! - `import_header` → [`super::ExtractedEdge::Imports`]
//! - `call_expression` → [`super::ExtractedEdge::Calls`]

use super::ParseResult;

/// Parse a Kotlin source file and extract symbols and edges.
///
/// This is a no-op stub returning an empty [`ParseResult`]. ABI compatibility
/// with tree-sitter 0.24 is verified in task B-1 (027.005-T); full extraction
/// is implemented in task B-2 (027.006-T).
///
/// # Errors
///
/// Currently infallible. Will return errors once grammar loading is added in B-1.
pub(super) fn parse_kotlin_source(
    _source: &str,
) -> Result<ParseResult, crate::errors::EngramError> {
    // No-op stub — grammar loading pending ABI verification in task B-1 (027.005-T).
    Ok(ParseResult {
        symbols: vec![],
        edges: vec![],
    })
}
