//! Tree-sitter C++ grammar parser.
//!
//! Extracts free functions, class/struct member functions, class and struct
//! declarations, and `#include` edges from C++ source files.
//!
//! # Verified node kinds (from tree-sitter-cpp/src/node-types.json)
//!
//! - `function_definition` (free + out-of-line member) → [`super::ExtractedSymbol::Function`]
//! - `class_specifier` → [`super::ExtractedSymbol::Class`]
//! - `struct_specifier` (named) → [`super::ExtractedSymbol::Class`]
//! - `preproc_include` → [`super::ExtractedEdge::Imports`]
//! - `call_expression` → [`super::ExtractedEdge::Calls`]
//!
//! # Out of scope
//!
//! Template instantiations and overload-set ranking are excluded by design.
//! Namespace declarations are treated as scope context only; no symbol is
//! emitted for them.

use tree_sitter::Parser;

use super::ParseResult;

/// Parse a C++ source file and extract symbols and edges.
///
/// This is a stub that verifies grammar ABI compatibility and returns an empty
/// [`ParseResult`]. Full extraction is implemented in task D-1 (027.010-T).
///
/// # Errors
///
/// Returns [`crate::errors::EngramError`] if the C++ grammar cannot be loaded.
pub(super) fn parse_cpp_source(_source: &str) -> Result<ParseResult, crate::errors::EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_cpp::LANGUAGE.into())
        .map_err(|e| {
            crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
                reason: format!("Failed to set C++ grammar: {e}"),
            })
        })?;
    // Symbol extraction not yet implemented — see task D-1 (027.010-T).
    Ok(ParseResult {
        symbols: vec![],
        edges: vec![],
    })
}
