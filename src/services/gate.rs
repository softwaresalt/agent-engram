//! Sandboxed query sanitizer for MCP graph queries.
//!
//! Validates that a SurrealQL query string does not contain write operations.
//! Used by the `query_graph` tool to enforce read-only query access.

// ── Sandboxed query sanitizer ─────────────────────────────────────────────────

/// Validates that a SurrealQL query string does not contain write operations.
///
/// Strips quoted string literals first, then checks for write keywords on word
/// boundaries. Returns `Ok(())` if the query is safe for read-only execution.
///
/// # Errors
///
/// Returns `Err(EngramError::GraphQuery(GraphQueryError::Rejected { keyword }))` when a
/// write keyword is detected outside of a quoted string literal.
///
/// # Examples
///
/// ```
/// use engram::services::gate::sanitize_query;
///
/// assert!(sanitize_query("SELECT * FROM task").is_ok());
/// assert!(sanitize_query("DELETE task:A").is_err());
/// ```
pub fn sanitize_query(query: &str) -> Result<(), crate::errors::EngramError> {
    use crate::errors::{EngramError, GraphQueryError};

    const WRITE_KEYWORDS: &[&str] = &[
        "INSERT", "UPDATE", "DELETE", "CREATE", "DEFINE", "REMOVE", "RELATE", "KILL", "SLEEP",
        "THROW", "UPSERT", "ALTER", "REBUILD",
    ];

    // Strip string literals to avoid false positives on keywords inside quotes.
    let stripped = strip_string_literals(query)?;
    let upper = stripped.to_uppercase();

    for keyword in WRITE_KEYWORDS {
        if contains_whole_word(&upper, keyword) {
            return Err(EngramError::GraphQuery(GraphQueryError::Rejected {
                keyword: (*keyword).to_string(),
            }));
        }
    }
    Ok(())
}

/// Replaces the content of single- and double-quoted string literals with spaces.
///
/// This prevents keyword detection from matching tokens that appear inside string
/// values while preserving byte length (and therefore byte offsets).
///
/// Returns `Err` if the input contains an unterminated string literal, which could
/// be used to hide write keywords from the sanitizer.
fn strip_string_literals(input: &str) -> Result<String, crate::errors::EngramError> {
    use crate::errors::{EngramError, GraphQueryError};

    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\'' || c == '"' {
            result.push(c);
            let mut closed = false;
            // Consume until the matching closing quote, honouring backslash escapes.
            while let Some(inner) = chars.next() {
                if inner == '\\' {
                    // Replace both the backslash and the escaped character with spaces.
                    result.push(' ');
                    if chars.next().is_some() {
                        result.push(' ');
                    }
                } else if inner == c {
                    result.push(inner);
                    closed = true;
                    break;
                } else {
                    result.push(' ');
                }
            }
            if !closed {
                return Err(EngramError::GraphQuery(GraphQueryError::Invalid {
                    reason: format!(
                        "unterminated string literal (opening {c} has no closing match)"
                    ),
                }));
            }
        } else {
            result.push(c);
        }
    }
    Ok(result)
}

/// Returns `true` if `keyword` appears as a whole word in `text`.
///
/// A whole word is bounded by non-alphanumeric/underscore characters or by the
/// start/end of the string. Both `text` and `keyword` must be uppercase for
/// case-insensitive matching.
fn contains_whole_word(text: &str, keyword: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = text[start..].find(keyword) {
        let abs = start + pos;
        let before_ok = abs == 0 || !text[..abs].chars().next_back().is_some_and(is_word_char);
        let end = abs + keyword.len();
        let after_ok = end >= text.len() || !text[end..].chars().next().is_some_and(is_word_char);
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

/// Returns `true` for characters that may appear inside an identifier or keyword.
const fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
