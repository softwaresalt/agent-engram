//! Typed error hierarchy for Engram domain operations.
//!
//! Errors are organized by domain: workspace (1xxx), hydration (2xxx),
//! query (4xxx), system (5xxx), config (6xxx), code graph (7xxx),
//! IPC/daemon (8xxx), and installer (9xxx). Each variant maps to a
//! numeric error code defined in [codes].

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
    #[error("Workspace schema version mismatch: found '{found}', expected '{expected}'. Migrate by deleting `.engram/` and running `engram install` again.")]
    SchemaMismatch { expected: String, found: String },
    #[error("Workspace state corrupted: {reason}")]
    CorruptedState { reason: String },
    #[error("Workspace files changed externally")]
    StaleWorkspace,
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

/// Errors for sandboxed graph queries (4010–4012).
#[derive(Debug, Error)]
pub enum GraphQueryError {
    #[error("Query rejected: write operations are not permitted (keyword: {keyword})")]
    Rejected { keyword: String },
    #[error("Query timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("Query syntax is invalid: {reason}")]
    Invalid { reason: String },
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
    #[error("Query must not be empty")]
    QueryEmpty,
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
    #[error("Embedding model failed to load: {reason}")]
    ModelLoadFailed { reason: String },
}

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("Failed to connect to daemon IPC endpoint '{address}': {reason}")]
    ConnectionFailed { address: String, reason: String },
    #[error("Failed to send IPC request: {reason}")]
    SendFailed { reason: String },
    #[error("Failed to receive IPC response: {reason}")]
    ReceiveFailed { reason: String },
    #[error("IPC request timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
}

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("Failed to spawn daemon process: {reason}")]
    SpawnFailed { reason: String },
    #[error("Daemon failed to reach Ready state within {timeout_ms}ms")]
    NotReady { timeout_ms: u64 },
}

#[derive(Debug, Error)]
pub enum LockError {
    #[error("Failed to acquire daemon lockfile '{path}': {reason}")]
    AcquisitionFailed { path: String, reason: String },
    #[error("Daemon lock already held by PID {pid}")]
    AlreadyHeld { pid: u32 },
}

#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("Failed to initialize file watcher for '{path}': {reason}")]
    InitFailed { path: String, reason: String },
}

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("Plugin installation failed: {reason}")]
    Failed { reason: String },
    #[error("Plugin update failed: {reason}")]
    UpdateFailed { reason: String },
    #[error("Plugin uninstall failed: {reason}")]
    UninstallFailed { reason: String },
    #[error("Engram plugin is already installed in this workspace")]
    AlreadyInstalled,
    #[error("Engram plugin is not installed in this workspace")]
    NotInstalled,
}

/// Errors for content registry operations (10xxx).
#[derive(Debug, Error)]
pub enum RegistryError {
    /// Failed to parse `.engram/registry.yaml`.
    #[error("Failed to parse registry YAML: {reason}")]
    ParseFailed { reason: String },
    /// A registry entry failed validation.
    #[error("Registry validation failed: {reason}")]
    ValidationFailed { reason: String },
}

/// Errors for content ingestion operations (11xxx).
#[derive(Debug, Error)]
pub enum IngestionError {
    /// Content ingestion failed for a source path.
    #[error("Ingestion failed for '{path}': {reason}")]
    Failed { path: String, reason: String },
}

/// Errors for git commit graph operations (12xxx).
#[derive(Debug, Error)]
pub enum GitGraphError {
    /// Git repository not found at workspace root.
    #[error("Git repository not found at '{path}'")]
    NotFound { path: String },
    /// Git access error during commit graph operations.
    #[error("Git access error: {reason}")]
    AccessError { reason: String },
}

#[derive(Debug, Error)]
pub enum EngramError {
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    #[error(transparent)]
    Hydration(#[from] HydrationError),
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error(transparent)]
    System(#[from] SystemError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    CodeGraph(#[from] CodeGraphError),
    #[error(transparent)]
    Ipc(#[from] IpcError),
    #[error(transparent)]
    Daemon(#[from] DaemonError),
    #[error(transparent)]
    Lock(#[from] LockError),
    #[error(transparent)]
    Watcher(#[from] WatcherError),
    #[error(transparent)]
    Install(#[from] InstallError),
    #[error(transparent)]
    GraphQuery(#[from] GraphQueryError),
    #[error(transparent)]
    Registry(#[from] RegistryError),
    #[error(transparent)]
    Ingestion(#[from] IngestionError),
    #[error(transparent)]
    GitGraph(#[from] GitGraphError),
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
            EngramError::Query(inner) => match inner {
                QueryError::QueryEmpty => (QUERY_EMPTY, "QueryEmpty", inner.to_string(), None),
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
                SystemError::ModelLoadFailed { reason } => (
                    MODEL_LOAD_FAILED,
                    "ModelLoadFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason, "suggestion": "try restarting" })),
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
            EngramError::Ipc(inner) => match inner {
                IpcError::ConnectionFailed { address, .. } => (
                    IPC_CONNECTION_FAILED,
                    "IpcConnectionFailed",
                    inner.to_string(),
                    Some(json!({ "address": address })),
                ),
                IpcError::SendFailed { reason } => (
                    IPC_SEND_FAILED,
                    "IpcSendFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                IpcError::ReceiveFailed { reason } => (
                    IPC_RECEIVE_FAILED,
                    "IpcReceiveFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                IpcError::Timeout { timeout_ms } => (
                    IPC_TIMEOUT,
                    "IpcTimeout",
                    inner.to_string(),
                    Some(json!({ "timeout_ms": timeout_ms })),
                ),
            },
            EngramError::Daemon(inner) => match inner {
                DaemonError::SpawnFailed { reason } => (
                    DAEMON_SPAWN_FAILED,
                    "DaemonSpawnFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                DaemonError::NotReady { timeout_ms } => (
                    DAEMON_NOT_READY,
                    "DaemonNotReady",
                    inner.to_string(),
                    Some(json!({ "timeout_ms": timeout_ms })),
                ),
            },
            EngramError::Lock(inner) => match inner {
                LockError::AcquisitionFailed { path, .. } => (
                    LOCK_ACQUISITION_FAILED,
                    "LockAcquisitionFailed",
                    inner.to_string(),
                    Some(json!({ "path": path })),
                ),
                LockError::AlreadyHeld { pid } => (
                    LOCK_ALREADY_HELD,
                    "LockAlreadyHeld",
                    inner.to_string(),
                    Some(json!({ "pid": pid })),
                ),
            },
            EngramError::Watcher(inner) => match inner {
                WatcherError::InitFailed { path, .. } => (
                    WATCHER_INIT_FAILED,
                    "WatcherInitFailed",
                    inner.to_string(),
                    Some(json!({ "path": path })),
                ),
            },
            EngramError::Install(inner) => match inner {
                InstallError::Failed { reason } => (
                    INSTALL_FAILED,
                    "InstallFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                InstallError::UpdateFailed { reason } => (
                    UPDATE_FAILED,
                    "UpdateFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                InstallError::UninstallFailed { reason } => (
                    UNINSTALL_FAILED,
                    "UninstallFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                InstallError::AlreadyInstalled => (
                    ALREADY_INSTALLED,
                    "AlreadyInstalled",
                    inner.to_string(),
                    None,
                ),
                InstallError::NotInstalled => {
                    (NOT_INSTALLED, "NotInstalled", inner.to_string(), None)
                }
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
            EngramError::GraphQuery(inner) => match inner {
                GraphQueryError::Rejected { keyword } => (
                    QUERY_REJECTED,
                    "QueryRejected",
                    inner.to_string(),
                    Some(json!({ "keyword": keyword })),
                ),
                GraphQueryError::Timeout { timeout_ms } => (
                    QUERY_TIMEOUT,
                    "QueryTimeout",
                    inner.to_string(),
                    Some(json!({ "timeout_ms": timeout_ms })),
                ),
                GraphQueryError::Invalid { reason } => (
                    QUERY_INVALID,
                    "QueryInvalid",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
            },
            EngramError::Registry(inner) => match inner {
                RegistryError::ParseFailed { reason } => (
                    REGISTRY_PARSE_FAILED,
                    "RegistryParseFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                RegistryError::ValidationFailed { reason } => (
                    REGISTRY_VALIDATION_FAILED,
                    "RegistryValidationFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
            },
            EngramError::Ingestion(inner) => match inner {
                IngestionError::Failed { path, reason } => (
                    INGESTION_FAILED,
                    "IngestionFailed",
                    inner.to_string(),
                    Some(json!({ "path": path, "reason": reason })),
                ),
            },
            EngramError::GitGraph(inner) => match inner {
                GitGraphError::NotFound { path } => (
                    GIT_NOT_FOUND,
                    "GitNotFound",
                    inner.to_string(),
                    Some(json!({ "path": path })),
                ),
                GitGraphError::AccessError { reason } => (
                    GIT_ACCESS_ERROR,
                    "GitAccessError",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
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
