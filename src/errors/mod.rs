//! Typed error hierarchy for Engram domain operations.
//!
//! Errors are organized by domain: workspace (1xxx), hydration (2xxx),
//! task (3xxx), query (4xxx), system (5xxx), config (6xxx), and
//! code graph (7xxx). Each variant maps to a numeric error code
//! defined in [codes].

use serde::Serialize;
use serde_json::{Value, json};
use thiserror::Error;

pub mod codes;
use codes::*;

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
    #[error("Workspace limit reached (limit {limit})")]
    LimitReached { limit: usize },
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
    #[error("Task '{id}' is already claimed by '{assignee}'")]
    AlreadyClaimed { id: String, assignee: String },
    #[error("Label validation failed: {reason}")]
    LabelValidation { reason: String },
    #[error("Batch operation partially failed: {succeeded} succeeded, {failed} failed")]
    BatchPartialFailure {
        succeeded: u64,
        failed: u64,
        results: serde_json::Value,
    },
    #[error("Compaction failed for task '{id}': {reason}")]
    CompactionFailed { id: String, reason: String },
    #[error("Invalid priority '{priority}'")]
    InvalidPriority { priority: String },
    #[error("Invalid issue type '{issue_type}'")]
    InvalidIssueType { issue_type: String },
    #[error("Duplicate label '{label}' on task '{task_id}'")]
    DuplicateLabel { task_id: String, label: String },
    #[error("Task '{id}' is not claimable in status '{status}'")]
    NotClaimable { id: String, status: String },
    #[error("Task title is empty")]
    TitleEmpty,
    #[error("Task title exceeds maximum length of 200 characters")]
    TitleTooLong,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to parse config: {reason}")]
    ParseError { reason: String },
    #[error("Invalid config value for '{key}': {reason}")]
    InvalidValue { key: String, reason: String },
    #[error("Unknown config key '{key}'")]
    UnknownKey { key: String },
}

#[derive(Debug, Error)]
pub enum CodeGraphError {
    /// A source file could not be parsed by tree-sitter.
    #[error("Failed to parse source file '{file_path}': line {line}, column {column}")]
    ParseError {
        file_path: String,
        line: u32,
        column: u32,
    },
    /// A file's language is not in the configured supported_languages list.
    #[error("Language '{language}' is not supported for file '{file_path}'")]
    UnsupportedLanguage { file_path: String, language: String },
    /// An indexing or sync operation is already running for this workspace.
    #[error("Indexing is already in progress for this workspace")]
    IndexInProgress,
    /// The requested symbol name does not exist in the code graph.
    #[error("Symbol '{name}' not found in code graph")]
    SymbolNotFound { name: String },
    /// A source file exceeds the configured maximum file size.
    #[error("File '{file_path}' exceeds maximum size ({size_bytes} > {max_bytes} bytes)")]
    FileTooLarge {
        file_path: String,
        size_bytes: u64,
        max_bytes: u64,
    },
    /// A sync operation detected conflicting state.
    #[error("File '{file_path}' changed during sync operation")]
    SyncConflict { file_path: String },
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
    #[error("Invalid request parameters: {reason}")]
    InvalidParams { reason: String },
}

#[derive(Debug, Error)]
pub enum EngramError {
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
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    CodeGraph(#[from] CodeGraphError),
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

impl EngramError {
    pub fn to_response(&self) -> ErrorResponse {
        let (code, name, message, details) = match self {
            EngramError::Workspace(inner) => match inner {
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
                WorkspaceError::LimitReached { limit } => (
                    WORKSPACE_LIMIT_REACHED,
                    "WorkspaceLimitReached",
                    inner.to_string(),
                    Some(json!({ "limit": limit })),
                ),
            },
            EngramError::Hydration(inner) => match inner {
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
            EngramError::Task(inner) => match inner {
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
                TaskError::AlreadyClaimed { id, assignee } => (
                    TASK_ALREADY_CLAIMED,
                    "TaskAlreadyClaimed",
                    inner.to_string(),
                    Some(json!({ "task_id": id, "assignee": assignee })),
                ),
                TaskError::LabelValidation { reason } => (
                    LABEL_VALIDATION,
                    "LabelValidation",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                TaskError::BatchPartialFailure {
                    succeeded,
                    failed,
                    results,
                } => (
                    BATCH_PARTIAL_FAILURE,
                    "BatchPartialFailure",
                    inner.to_string(),
                    Some(json!({ "succeeded": succeeded, "failed": failed, "results": results })),
                ),
                TaskError::CompactionFailed { id, reason } => (
                    COMPACTION_FAILED,
                    "CompactionFailed",
                    inner.to_string(),
                    Some(json!({ "task_id": id, "reason": reason })),
                ),
                TaskError::InvalidPriority { priority } => (
                    INVALID_PRIORITY,
                    "InvalidPriority",
                    inner.to_string(),
                    Some(json!({ "priority": priority })),
                ),
                TaskError::InvalidIssueType { issue_type } => (
                    INVALID_ISSUE_TYPE,
                    "InvalidIssueType",
                    inner.to_string(),
                    Some(json!({ "issue_type": issue_type })),
                ),
                TaskError::DuplicateLabel { task_id, label } => (
                    DUPLICATE_LABEL,
                    "DuplicateLabel",
                    inner.to_string(),
                    Some(json!({ "task_id": task_id, "label": label })),
                ),
                TaskError::NotClaimable { id, status } => (
                    TASK_NOT_CLAIMABLE,
                    "TaskNotClaimable",
                    inner.to_string(),
                    Some(json!({ "task_id": id, "status": status })),
                ),
                TaskError::TitleEmpty => {
                    (TASK_TITLE_EMPTY, "TaskTitleEmpty", inner.to_string(), None)
                }
                TaskError::TitleTooLong => (
                    TASK_TITLE_TOO_LONG,
                    "TaskTitleTooLong",
                    inner.to_string(),
                    None,
                ),
            },
            EngramError::Query(inner) => match inner {
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
            EngramError::System(inner) => match inner {
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
                SystemError::InvalidParams { reason } => (
                    INVALID_PARAMS,
                    "InvalidParams",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
            },
            EngramError::Config(inner) => match inner {
                ConfigError::ParseError { reason } => (
                    CONFIG_PARSE_ERROR,
                    "ConfigParseError",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                ConfigError::InvalidValue { key, reason } => (
                    CONFIG_INVALID_VALUE,
                    "ConfigInvalidValue",
                    inner.to_string(),
                    Some(json!({ "key": key, "reason": reason })),
                ),
                ConfigError::UnknownKey { key } => (
                    UNKNOWN_CONFIG_KEY,
                    "UnknownConfigKey",
                    inner.to_string(),
                    Some(json!({ "key": key })),
                ),
            },
            EngramError::CodeGraph(inner) => match inner {
                CodeGraphError::ParseError {
                    file_path,
                    line,
                    column,
                } => (
                    PARSE_ERROR,
                    "ParseError",
                    inner.to_string(),
                    Some(
                        json!({ "file_path": file_path, "line": line, "column": column, "suggestion": "Fix the syntax error and re-run sync_workspace" }),
                    ),
                ),
                CodeGraphError::UnsupportedLanguage {
                    file_path,
                    language,
                } => (
                    UNSUPPORTED_LANGUAGE,
                    "UnsupportedLanguage",
                    inner.to_string(),
                    Some(
                        json!({ "file_path": file_path, "language": language, "supported": ["rust"], "suggestion": "Add language support or exclude the file via code_graph.exclude_patterns" }),
                    ),
                ),
                CodeGraphError::IndexInProgress => (
                    INDEX_IN_PROGRESS,
                    "IndexInProgress",
                    inner.to_string(),
                    Some(
                        json!({ "suggestion": "Wait for the current indexing operation to complete" }),
                    ),
                ),
                CodeGraphError::SymbolNotFound { name } => (
                    SYMBOL_NOT_FOUND,
                    "SymbolNotFound",
                    inner.to_string(),
                    Some(
                        json!({ "symbol_name": name, "suggestion": "Run index_workspace or check the symbol name spelling" }),
                    ),
                ),
                CodeGraphError::FileTooLarge {
                    file_path,
                    size_bytes,
                    max_bytes,
                } => (
                    FILE_TOO_LARGE,
                    "FileTooLarge",
                    inner.to_string(),
                    Some(
                        json!({ "file_path": file_path, "size_bytes": size_bytes, "max_bytes": max_bytes, "suggestion": "Exclude the file via code_graph.exclude_patterns or increase code_graph.max_file_size_bytes" }),
                    ),
                ),
                CodeGraphError::SyncConflict { file_path } => (
                    SYNC_CONFLICT,
                    "SyncConflict",
                    inner.to_string(),
                    Some(
                        json!({ "file_path": file_path, "suggestion": "Re-run sync_workspace to resolve the conflict" }),
                    ),
                ),
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
        let err = EngramError::from(WorkspaceError::NotFound {
            path: "./missing".into(),
        });
        let payload = err.to_response();
        assert_eq!(payload.error.code, WORKSPACE_NOT_FOUND);
        assert_eq!(payload.error.name, "WorkspaceNotFound");
    }
}
