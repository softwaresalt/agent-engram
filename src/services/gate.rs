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
