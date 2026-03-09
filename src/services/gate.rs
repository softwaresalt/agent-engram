//! Dependency gate evaluation for task status transitions.
//!
//! Enforces that tasks with `hard_blocker` upstream prerequisites cannot
//! transition to `in_progress` until all blockers are `done`. Also collects
//! `soft_dependency` edges and emits warnings instead of rejections.
//!
//! See `specs/005-lifecycle-observability/spec.md` User Story 1 for requirements.

use crate::db::queries::Queries;
use crate::errors::{EngramError, TaskError};

/// Outcome of evaluating the dependency gate for an `in_progress` transition.
#[derive(Debug)]
pub struct GateResult {
    /// Incomplete hard_blocker prerequisites (each a JSON object with
    /// `id`/`status`/`dependency_type`/`transitively_blocks`).
    pub blockers: Vec<serde_json::Value>,
    /// Incomplete soft_dependency prerequisites (each a JSON object with
    /// `type`/`id`/`status`).
    pub warnings: Vec<serde_json::Value>,
}

impl GateResult {
    /// Returns `true` if the gate blocks the transition (one or more hard blockers remain).
    pub fn is_blocked(&self) -> bool {
        !self.blockers.is_empty()
    }
}

/// Evaluates the dependency gate for a task about to transition to `in_progress`.
///
/// Returns `Ok(GateResult)` where:
/// - `blockers` is empty → gate passes; `warnings` may be non-empty.
/// - `blockers` is non-empty → gate fails; caller should convert via [`blocked_error`].
///
/// Callers MUST only invoke this function when `new_status == TaskStatus::InProgress`.
///
/// # Errors
///
/// Returns `Err` if a database error occurs while traversing dependencies.
pub async fn evaluate(task_id: &str, queries: &Queries) -> Result<GateResult, EngramError> {
    let blockers = queries.check_blockers(task_id).await?;
    let warnings = if blockers.is_empty() {
        // Only collect soft dep warnings when hard blockers don't fail the gate
        // (S011: mixed hard/soft — hard failure takes precedence).
        queries.check_soft_deps(task_id).await?
    } else {
        Vec::new()
    };
    Ok(GateResult { blockers, warnings })
}

/// Converts a blocked [`GateResult`] into an [`EngramError`].
///
/// # Panics
///
/// Panics (in debug builds) if called on a `GateResult` that is not blocked
/// (i.e., `blockers` is empty). This represents a programmer error.
pub fn blocked_error(task_id: &str, result: GateResult) -> EngramError {
    debug_assert!(
        !result.blockers.is_empty(),
        "blocked_error called on a non-blocked GateResult"
    );
    let count = result.blockers.len();
    EngramError::Task(TaskError::Blocked {
        id: task_id.to_string(),
        blocker_count: count,
        blockers: result.blockers,
    })
}

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
    let stripped = strip_string_literals(query);
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
fn strip_string_literals(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\'' || c == '"' {
            result.push(c);
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
                    break;
                } else {
                    result.push(' ');
                }
            }
        } else {
            result.push(c);
        }
    }
    result
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
