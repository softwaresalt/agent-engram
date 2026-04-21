//! Tree-sitter C grammar parser.
//!
//! Extracts top-level functions, named struct definitions, and `#include`
//! edges from C source files.
//!
//! # Verified node kinds (from tree-sitter-c/src/node-types.json)
//!
//! - `function_definition` → [`super::ExtractedSymbol::Function`]
//! - `struct_specifier` (named) → [`super::ExtractedSymbol::Class`]
//! - `preproc_include` → [`super::ExtractedEdge::Imports`]
//! - `call_expression` → [`super::ExtractedEdge::Calls`]
//!
//! # Out of scope
//!
//! Function-pointer calls — `(*fn_ptr)(args)` or indirect `fn_ptr(args)` where
//! `fn_ptr` is a variable — are excluded by design to avoid false-positive
//! `Calls` edges from indirect dispatch. Only direct call-identifier expressions
//! are extracted.

use tree_sitter::Parser;

use super::ParseResult;

/// Parse a C source file and extract symbols and edges.
///
/// This is a stub that verifies grammar ABI compatibility and returns an empty
/// [`ParseResult`]. Full extraction is implemented in task C-1 (027.008-T).
///
/// # Errors
///
/// Returns [`crate::errors::EngramError`] if the C grammar cannot be loaded
/// (ABI mismatch or internal tree-sitter error).
pub(super) fn parse_c_source(_source: &str) -> Result<ParseResult, crate::errors::EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::LANGUAGE.into())
        .map_err(|e| {
            crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
                reason: format!("Failed to set C grammar: {e}"),
            })
        })?;
    // Symbol extraction not yet implemented — see task C-1 (027.008-T).
    Ok(ParseResult {
        symbols: vec![],
        edges: vec![],
    })
}
