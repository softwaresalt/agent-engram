//! Kotlin grammar parser stub.
//!
//! Full extraction is deferred: `tree-sitter-kotlin 0.3.x` depends on
//! `tree-sitter 0.20–0.22`, which conflicts with the `tree-sitter 0.24`
//! runtime used by the rest of this codebase.  The `Language::Kotlin` variant
//! and file-extension routing (`.kt`, `.kts`) are wired up so this language
//! slot is ready for activation once a `tree-sitter 0.24`-compatible Kotlin
//! grammar crate is published.
//!
//! Until then this function returns an empty [`ParseResult`] rather than an
//! error, so Kotlin files are silently skipped during indexing rather than
//! causing noise.

use super::ParseResult;

/// Parse a Kotlin source file.
///
/// Currently a no-op stub — returns an empty result.  See module-level docs
/// for the reason this feature is deferred.
///
/// # Errors
///
/// This function never errors in its current stub form.
// The Result return type is required to match the dispatcher's call convention.
// This stub intentionally never errors; the allow suppresses the lint.
#[allow(clippy::unnecessary_wraps)]
pub(super) fn parse_kotlin_source(
    _source: &str,
) -> Result<ParseResult, crate::errors::EngramError> {
    Ok(ParseResult {
        symbols: vec![],
        edges: vec![],
    })
}
