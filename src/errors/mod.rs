use serde::Serialize;
use serde_json::{Value, json};
use thiserror::Error;

use crate::errors::codes::*;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("Path '{path}' does not exist")]
    NotFound { path: String },
    #[error("Path '{path}' is not a Git repository root")]
    NotGitRoot { path: String },
    #[error("No workspace bound to this connection")]
    NotSet,
    #[error("Workspace '{path}' already active")]
    AlreadyActive { path: String },
}

#[derive(Debug, Error)]
pub enum HydrationError {
    #[error("Failed to parse workspace files: {reason}")]
    Failed { reason: String },
    #[error("Workspace schema mismatch: {expected} vs {found}")]
    SchemaMismatch { expected: String, found: String },
    #[error("Workspace state corrupted: {reason}")]
    CorruptedState { reason: String },
    #[error("Workspace files changed externally")]
    StaleWorkspace,
}

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("Task '{id}' not found")]
    NotFound { id: String },
    #[error("Invalid status '{status}'")]
    InvalidStatus { status: String },
    #[error("Cyclic dependency detected")]
    CyclicDependency,
    #[error("Blocker already exists for task '{id}'")]
    BlockerExists { id: String },
}

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("Query too long")]
    QueryTooLong,
    #[error("Model not loaded")]
    ModelNotLoaded,
    #[error("Search failed: {reason}")]
    SearchFailed { reason: String },
}

#[derive(Debug, Error)]
pub enum SystemError {
    #[error("Database operation failed: {reason}")]
    DatabaseError { reason: String },
    #[error("Failed to write workspace state: {path}")]
    FlushFailed { path: String },
    #[error("Rate limited")]
    RateLimited,
    #[error("Daemon is shutting down")]
    ShuttingDown,
}

#[derive(Debug, Error)]
pub enum TMemError {
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    #[error(transparent)]
    Hydration(#[from] HydrationError),
    #[error(transparent)]
    Task(#[from] TaskError),
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error(transparent)]
    System(#[from] SystemError),
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: u16,
    pub name: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

impl TMemError {
    pub fn to_response(&self) -> ErrorResponse {
        let (code, name, message, details) = match self {
            TMemError::Workspace(inner) => match inner {
                WorkspaceError::NotFound { path } => (
                    WORKSPACE_NOT_FOUND,
                    "WorkspaceNotFound",
                    inner.to_string(),
                    Some(json!({ "path": path })),
                ),
                WorkspaceError::NotGitRoot { path } => (
                    NOT_A_GIT_ROOT,
                    "NotAGitRoot",
                    inner.to_string(),
                    Some(json!({ "path": path })),
                ),
                WorkspaceError::NotSet => (
                    WORKSPACE_NOT_SET,
                    "WorkspaceNotSet",
                    inner.to_string(),
                    None,
                ),
                WorkspaceError::AlreadyActive { path } => (
                    WORKSPACE_ALREADY_ACTIVE,
                    "WorkspaceAlreadyActive",
                    inner.to_string(),
                    Some(json!({ "path": path })),
                ),
            },
            TMemError::Hydration(inner) => match inner {
                HydrationError::Failed { reason } => (
                    HYDRATION_FAILED,
                    "HydrationFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                HydrationError::SchemaMismatch { expected, found } => (
                    SCHEMA_MISMATCH,
                    "SchemaMismatch",
                    inner.to_string(),
                    Some(json!({ "expected": expected, "found": found })),
                ),
                HydrationError::CorruptedState { reason } => (
                    CORRUPTED_STATE,
                    "CorruptedState",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                HydrationError::StaleWorkspace => {
                    (STALE_WORKSPACE, "StaleWorkspace", inner.to_string(), None)
                }
            },
            TMemError::Task(inner) => match inner {
                TaskError::NotFound { id } => (
                    TASK_NOT_FOUND,
                    "TaskNotFound",
                    inner.to_string(),
                    Some(json!({ "task_id": id })),
                ),
                TaskError::InvalidStatus { status } => (
                    INVALID_STATUS,
                    "InvalidStatus",
                    inner.to_string(),
                    Some(json!({ "status": status })),
                ),
                TaskError::CyclicDependency => (
                    CYCLIC_DEPENDENCY,
                    "CyclicDependency",
                    inner.to_string(),
                    None,
                ),
                TaskError::BlockerExists { id } => (
                    BLOCKER_EXISTS,
                    "BlockerExists",
                    inner.to_string(),
                    Some(json!({ "task_id": id })),
                ),
            },
            TMemError::Query(inner) => match inner {
                QueryError::QueryTooLong => {
                    (QUERY_TOO_LONG, "QueryTooLong", inner.to_string(), None)
                }
                QueryError::ModelNotLoaded => {
                    (MODEL_NOT_LOADED, "ModelNotLoaded", inner.to_string(), None)
                }
                QueryError::SearchFailed { reason } => (
                    SEARCH_FAILED,
                    "SearchFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
            },
            TMemError::System(inner) => match inner {
                SystemError::DatabaseError { reason } => (
                    DATABASE_ERROR,
                    "DatabaseError",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                SystemError::FlushFailed { path } => (
                    FLUSH_FAILED,
                    "FlushFailed",
                    inner.to_string(),
                    Some(json!({ "path": path })),
                ),
                SystemError::RateLimited => (RATE_LIMITED, "RateLimited", inner.to_string(), None),
                SystemError::ShuttingDown => {
                    (SHUTTING_DOWN, "ShuttingDown", inner.to_string(), None)
                }
            },
        };

        ErrorResponse {
            error: ErrorBody {
                code,
                name: name.to_string(),
                message,
                details,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_workspace_not_found() {
        let err = TMemError::from(WorkspaceError::NotFound {
            path: "./missing".into(),
        });
        let payload = err.to_response();
        assert_eq!(payload.error.code, WORKSPACE_NOT_FOUND);
        assert_eq!(payload.error.name, "WorkspaceNotFound");
    }
}
