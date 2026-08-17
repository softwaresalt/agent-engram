//! Tree-sitter HCL grammar service.
//!
//! Symbol and reference extraction are intentionally deferred to later units.

use tree_sitter::Parser;

use super::ParseResult;
use crate::errors::{CodeGraphError, EngramError};

/// Parse HCL source through the shared grammar.
///
/// # Errors
///
/// Returns an [`EngramError`] if the grammar cannot be loaded or tree-sitter
/// fails to produce a parse tree.
pub(super) fn parse_hcl_source(source: &str) -> Result<ParseResult, EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_hcl::LANGUAGE.into())
        .map_err(|error| {
            EngramError::CodeGraph(CodeGraphError::ParseFailed {
                reason: format!("failed to set HCL grammar: {error}"),
            })
        })?;

    let _tree = parser.parse(source, None).ok_or_else(|| {
        EngramError::CodeGraph(CodeGraphError::ParseFailed {
            reason: "tree-sitter returned no parse tree for HCL source".to_owned(),
        })
    })?;

    Ok(ParseResult {
        symbols: Vec::new(),
        edges: Vec::new(),
    })
}
