//! Typed error hierarchy for Engram domain operations.
//!
//! Errors are organized by domain: workspace (1xxx), hydration (2xxx),
//! query (4xxx), system (5xxx), config (6xxx), code graph (7xxx),
//! IPC/daemon (8xxx), installer (9xxx), registry (10xxx),
//! ingestion (11xxx), git graph (12xxx), and metrics (13xxx).
//! Each variant maps to a numeric error code defined in [codes].

use std::path::PathBuf;

use serde::Serialize;
use serde_json::{Value, json};
use thiserror::Error;
use uuid::Uuid;

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
    #[error(
        "Path '{attempted}' escapes the workspace root '{root}'; only in-workspace paths are allowed"
    )]
    PathEscape { attempted: PathBuf, root: PathBuf },
    #[error(
        "Workspace bind is ambiguous for '{path}': expected workspace-id {expected_id}, found {found_id}. Remove stale runtime state and retry."
    )]
    AmbiguousBind {
        expected_id: Uuid,
        found_id: Uuid,
        path: PathBuf,
    },
}

#[derive(Debug, Error)]
pub enum HydrationError {
    #[error("Failed to parse workspace files: {reason}")]
    Failed { reason: String },
    #[error(
        "Workspace schema version mismatch: found '{found}', expected '{expected}'. Migrate by deleting `.engram/` and running `engram install` again."
    )]
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
    /// Grammar-engine parse failure with no source-location information.
    #[error("Source parse failed: {reason}")]
    ParseFailed { reason: String },
    /// A code-graph source could not be safely accessed beneath the workspace.
    #[error("Failed to access source file '{file_path}': {reason}")]
    SourceAccess { file_path: String, reason: String },
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
    #[error(
        "Daemon protocol version mismatch: expected {expected}, found {actual}. Restart the daemon or rerun the shim to respawn the current binary."
    )]
    VersionMismatch { expected: u32, actual: u32 },
}

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("Failed to spawn daemon process: {reason}")]
    SpawnFailed { reason: String },
    #[error(
        "Daemon failed to reach Ready state within {timeout_ms}ms. If the engram daemon process has exited, run 'engram index --direct' (or set ENGRAM_DIRECT=1) to index without the daemon. If a daemon process is still running, --direct will fail while it holds the workspace lock — wait and retry if it is still starting up, or stop that engram process if it appears stuck."
    )]
    NotReady { timeout_ms: u64 },
    /// The previous daemon failed to exit within the shutdown-wait deadline
    /// during a respawn. Unlike [`DaemonError::NotReady`] (a genuine startup
    /// timeout), the stuck daemon still holds the workspace lock, so
    /// `--direct` / `ENGRAM_DIRECT=1` is *not* a valid escape hatch here — the
    /// operator must stop the running daemon process first.
    #[error(
        "Daemon failed to shut down within {timeout_ms}ms during respawn; the previous daemon is still running and holds the workspace lock — stop the running engram daemon process, then retry"
    )]
    ShutdownTimeout { timeout_ms: u64 },
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

/// Errors for the metrics subsystem (13xxx).
#[derive(Debug, Error)]
pub enum MetricsError {
    /// Failed to write metrics data to disk.
    #[error("failed to write metrics: {reason}")]
    WriteFailed { reason: String },
    /// No metrics data found for the requested branch.
    #[error("no metrics data found for branch '{branch}'")]
    NotFound { branch: String },
    /// Failed to parse persisted metrics data.
    #[error("failed to parse metrics data: {reason}")]
    ParseError { reason: String },
}

/// Errors for the MCP sandbox policy engine (14xxx).
#[derive(Debug, Error)]
pub enum PolicyError {
    /// Agent role is denied from calling the requested tool.
    #[error("agent '{agent_role}' is denied access to tool '{tool_name}'")]
    Denied {
        agent_role: String,
        tool_name: String,
    },
    /// Policy configuration is invalid (logged as warning, fallback to disabled).
    #[error("invalid policy configuration: {reason}")]
    ConfigInvalid { reason: String },
}

/// Classifies why the shim's deferred startup preconditions (workspace
/// admission, daemon readiness, IPC endpoint derivation) or the MCP stdio
/// transport itself failed (124-F, stash 870B1AFF).
///
/// Under the serve-first, degrade-in-session contract, a shim session always
/// answers `initialize` and serves the static `tools/list` catalog regardless
/// of these outcomes. This classification names the cause surfaced to
/// `tools/call` callers, recorded in the durable startup-failure record, and
/// reflected in the shim process's documented exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShimFailureClass {
    /// Workspace admission failed (e.g. path not found, not a Git root).
    AdmissionFailure,
    /// The daemon failed to reach a ready state within its startup budget.
    ReadinessTimeout,
    /// The daemon IPC endpoint could not be derived for the workspace.
    EndpointDerivationFailure,
    /// The MCP stdio transport itself failed to bind or ended abnormally.
    TransportFailure,
    /// The daemon's protocol or `_health` contract is incompatible with this shim.
    ProtocolIncompatible,
}

impl ShimFailureClass {
    /// Documented process exit code for this failure class (124-F U5).
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            ShimFailureClass::AdmissionFailure => 10,
            ShimFailureClass::ReadinessTimeout => 11,
            ShimFailureClass::EndpointDerivationFailure => 12,
            ShimFailureClass::TransportFailure => 13,
            ShimFailureClass::ProtocolIncompatible => 14,
        }
    }

    /// Stable machine-readable name used in the startup-failure record and
    /// structured `tools/call` error data.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ShimFailureClass::AdmissionFailure => "admission_failure",
            ShimFailureClass::ReadinessTimeout => "readiness_timeout",
            ShimFailureClass::EndpointDerivationFailure => "endpoint_derivation_failure",
            ShimFailureClass::TransportFailure => "transport_failure",
            ShimFailureClass::ProtocolIncompatible => "protocol_incompatible",
        }
    }

    /// Numeric wire error code for this failure class (15xxx range).
    #[must_use]
    pub const fn wire_code(self) -> u16 {
        match self {
            ShimFailureClass::AdmissionFailure => SHIM_ADMISSION_FAILURE,
            ShimFailureClass::ReadinessTimeout => SHIM_READINESS_TIMEOUT,
            ShimFailureClass::EndpointDerivationFailure => SHIM_ENDPOINT_DERIVATION_FAILED,
            ShimFailureClass::TransportFailure => SHIM_TRANSPORT_FAILURE,
            ShimFailureClass::ProtocolIncompatible => SHIM_PROTOCOL_INCOMPATIBLE,
        }
    }

    /// Fixed, class-specific, variable-free description for the durable
    /// startup-failure record (124-F U5). The live `message` carried in
    /// [`ShimStartupError`] may embed step-specific detail (e.g. the
    /// caller-supplied workspace path for [`ShimFailureClass::AdmissionFailure`])
    /// that is appropriate to surface live (`tools/call` response, stderr)
    /// but not to persist into an on-disk record that could later be
    /// aggregated across many workspaces.
    #[must_use]
    pub const fn record_message(self) -> &'static str {
        match self {
            ShimFailureClass::AdmissionFailure => {
                "workspace path does not exist or is not a Git repository root"
            }
            ShimFailureClass::ReadinessTimeout => {
                "daemon did not reach a ready state within the configured budget"
            }
            ShimFailureClass::EndpointDerivationFailure => {
                "failed to derive the daemon IPC endpoint for this workspace"
            }
            ShimFailureClass::TransportFailure => {
                "MCP stdio transport failed to bind or the session ended abnormally"
            }
            ShimFailureClass::ProtocolIncompatible => {
                "daemon protocol or _health contract is incompatible with this shim"
            }
        }
    }
}

/// A classified shim startup failure carrying an attributable, live-facing
/// message (surfaced in the degraded `tools/call` response and the shim's
/// stderr line).
///
/// This `message` MUST NOT contain credentials, tokens, or environment
/// variable values. It MAY legitimately contain the workspace's own path
/// (e.g. [`ShimFailureClass::AdmissionFailure`]'s underlying error names the
/// caller-supplied workspace path) — that is expected, live, operator-facing
/// detail, not a leak. The stronger guarantee — no variable data at all, not
/// even the workspace's own path — applies only to the *persisted* durable
/// startup-failure record, which never stores this `message`; it stores
/// [`ShimFailureClass::record_message`]'s fixed, class-specific description
/// instead (see `shim::spawn_record_startup_failure`).
///
/// Despite its name, this type also carries [`ShimFailureClass::TransportFailure`]
/// (the MCP stdio transport itself failed to bind, or the session ended with
/// a protocol error) — a failure discovered at transport level, not a
/// deferred startup precondition. The `Display` wording below is
/// deliberately class-neutral ("shim failure") rather than
/// "startup precondition failed" so it does not overclaim for that case.
#[derive(Debug, Error, Clone)]
#[error("shim failure ({}): {message}", class.as_str())]
pub struct ShimStartupError {
    pub class: ShimFailureClass,
    pub message: String,
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
    #[error(transparent)]
    Metrics(#[from] MetricsError),
    #[error(transparent)]
    Policy(#[from] PolicyError),
    #[error(transparent)]
    ShimStartup(#[from] ShimStartupError),
}

impl EngramError {
    /// Returns the documented shim process exit code when this error is a
    /// classified [`ShimStartupError`] (124-F U5), otherwise `None`.
    #[must_use]
    pub fn shim_exit_code(&self) -> Option<i32> {
        match self {
            EngramError::ShimStartup(inner) => Some(inner.class.exit_code()),
            _ => None,
        }
    }
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
                WorkspaceError::PathEscape { attempted, root } => (
                    INVALID_PARAMS,
                    "WorkspacePathEscape",
                    inner.to_string(),
                    Some(json!({
                        "attempted": attempted.display().to_string(),
                        "root": root.display().to_string(),
                    })),
                ),
                WorkspaceError::AmbiguousBind {
                    expected_id,
                    found_id,
                    path,
                } => (
                    INVALID_PARAMS,
                    "AmbiguousBind",
                    inner.to_string(),
                    Some(json!({
                        "expected_id": expected_id.to_string(),
                        "found_id": found_id.to_string(),
                        "path": path.display().to_string(),
                    })),
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
                IpcError::VersionMismatch { expected, actual } => (
                    IPC_CONNECTION_FAILED,
                    "IpcVersionMismatch",
                    inner.to_string(),
                    Some(json!({ "expected": expected, "actual": actual })),
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
                DaemonError::ShutdownTimeout { timeout_ms } => (
                    DAEMON_SHUTDOWN_TIMEOUT,
                    "DaemonShutdownTimeout",
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
                CodeGraphError::ParseFailed { reason } => (
                    PARSE_FAILED,
                    "ParseFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                CodeGraphError::SourceAccess { file_path, reason } => (
                    SOURCE_ACCESS_FAILED,
                    "SourceAccessFailed",
                    inner.to_string(),
                    Some(json!({ "file_path": file_path, "reason": reason })),
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
            EngramError::Metrics(inner) => match inner {
                MetricsError::WriteFailed { reason } => (
                    METRICS_WRITE_FAILED,
                    "MetricsWriteFailed",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
                MetricsError::NotFound { branch } => (
                    METRICS_NOT_FOUND,
                    "MetricsNotFound",
                    inner.to_string(),
                    Some(json!({ "branch": branch })),
                ),
                MetricsError::ParseError { reason } => (
                    METRICS_PARSE_ERROR,
                    "MetricsParseError",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
            },
            EngramError::Policy(inner) => match inner {
                PolicyError::Denied {
                    agent_role,
                    tool_name,
                } => (
                    POLICY_DENIED,
                    "PolicyDenied",
                    inner.to_string(),
                    Some(json!({ "agent_role": agent_role, "tool_name": tool_name })),
                ),
                PolicyError::ConfigInvalid { reason } => (
                    POLICY_CONFIG_INVALID,
                    "PolicyConfigInvalid",
                    inner.to_string(),
                    Some(json!({ "reason": reason })),
                ),
            },
            EngramError::ShimStartup(inner) => (
                inner.class.wire_code(),
                "ShimStartupFailed",
                inner.to_string(),
                Some(json!({ "failure_class": inner.class.as_str() })),
            ),
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

    #[test]
    fn not_ready_message_points_at_direct() {
        let msg = DaemonError::NotReady { timeout_ms: 5000 }.to_string();
        // The runtime timeout interpolation is preserved.
        assert!(
            msg.contains("5000ms"),
            "NotReady message should retain the timeout value: {msg}"
        );
        // The message signposts the daemonless escape hatch.
        assert!(
            msg.contains("--direct"),
            "NotReady message should mention `--direct`: {msg}"
        );
        assert!(
            msg.contains("ENGRAM_DIRECT=1"),
            "NotReady message should mention `ENGRAM_DIRECT=1`: {msg}"
        );
        // F1 (E0659C5C): the hint must also cover the startup-hydration-hang
        // sub-case where a daemon is still running and holds the workspace lock,
        // so `--direct` would fail with AlreadyHeld. The message must both name
        // the exited-daemon branch (where `--direct` is valid) and tell the user
        // to stop a stuck daemon, otherwise the `--direct` hint is misleading.
        assert!(
            msg.contains("exited"),
            "NotReady message should name the exited-daemon branch: {msg}"
        );
        assert!(
            msg.contains("stop"),
            "NotReady message should tell the user to stop a stuck daemon: {msg}"
        );
        assert!(
            msg.contains("lock"),
            "NotReady message should mention the held workspace lock: {msg}"
        );
        // No stray thiserror braces leaked into the rendered string.
        assert!(
            !msg.contains('{') && !msg.contains('}'),
            "NotReady message must not contain literal braces: {msg}"
        );
    }

    #[test]
    fn not_ready_wire_contract_unchanged() {
        let payload = EngramError::from(DaemonError::NotReady { timeout_ms: 5000 }).to_response();
        // The machine-readable contract is string-independent and must not move
        // when the human-facing message text changes.
        assert_eq!(payload.error.code, DAEMON_NOT_READY);
        // Pin the literal external number so a future renumber is a visible break.
        assert_eq!(payload.error.code, 8006);
        assert_eq!(payload.error.name, "DaemonNotReady");
    }

    #[test]
    fn shutdown_timeout_message_omits_direct() {
        let msg = DaemonError::ShutdownTimeout { timeout_ms: 2000 }.to_string();
        // The runtime timeout interpolation is preserved.
        assert!(
            msg.contains("2000ms"),
            "ShutdownTimeout message should retain the timeout value: {msg}"
        );
        // The shutdown-wait path must NOT steer users to `--direct`: the stuck
        // daemon still holds the workspace lock, so daemonless indexing fails.
        assert!(
            !msg.contains("--direct"),
            "ShutdownTimeout message must not mention `--direct`: {msg}"
        );
        assert!(
            !msg.contains("ENGRAM_DIRECT"),
            "ShutdownTimeout message must not mention `ENGRAM_DIRECT`: {msg}"
        );
        // It is a shutdown-oriented message that points at the held lock.
        assert!(
            msg.to_lowercase().contains("shut down"),
            "ShutdownTimeout message should describe a shutdown timeout: {msg}"
        );
        assert!(
            msg.to_lowercase().contains("lock"),
            "ShutdownTimeout message should mention the held workspace lock: {msg}"
        );
        // No stray thiserror braces leaked into the rendered string.
        assert!(
            !msg.contains('{') && !msg.contains('}'),
            "ShutdownTimeout message must not contain literal braces: {msg}"
        );
    }

    #[test]
    fn shutdown_timeout_wire_contract() {
        let payload =
            EngramError::from(DaemonError::ShutdownTimeout { timeout_ms: 2000 }).to_response();
        assert_eq!(payload.error.code, DAEMON_SHUTDOWN_TIMEOUT);
        // Pin the literal external number so a future renumber is a visible break.
        assert_eq!(payload.error.code, 8010);
        assert_eq!(payload.error.name, "DaemonShutdownTimeout");
        assert_eq!(payload.error.details, Some(json!({ "timeout_ms": 2000 })));
    }

    /// Golden-record additivity check: pre-existing `ShimFailureClass` values
    /// are byte-identical and the new `ProtocolIncompatible` is strictly additive.
    #[test]
    fn shim_failure_class_golden_record() {
        // Pre-existing variants — values MUST NOT shift.
        assert_eq!(ShimFailureClass::AdmissionFailure.exit_code(), 10);
        assert_eq!(
            ShimFailureClass::AdmissionFailure.as_str(),
            "admission_failure"
        );
        assert_eq!(ShimFailureClass::AdmissionFailure.wire_code(), 15_001);

        assert_eq!(ShimFailureClass::ReadinessTimeout.exit_code(), 11);
        assert_eq!(
            ShimFailureClass::ReadinessTimeout.as_str(),
            "readiness_timeout"
        );
        assert_eq!(ShimFailureClass::ReadinessTimeout.wire_code(), 15_002);

        assert_eq!(ShimFailureClass::EndpointDerivationFailure.exit_code(), 12);
        assert_eq!(
            ShimFailureClass::EndpointDerivationFailure.as_str(),
            "endpoint_derivation_failure"
        );
        assert_eq!(
            ShimFailureClass::EndpointDerivationFailure.wire_code(),
            15_003
        );

        assert_eq!(ShimFailureClass::TransportFailure.exit_code(), 13);
        assert_eq!(
            ShimFailureClass::TransportFailure.as_str(),
            "transport_failure"
        );
        assert_eq!(ShimFailureClass::TransportFailure.wire_code(), 15_004);

        // New additive variant.
        assert_eq!(ShimFailureClass::ProtocolIncompatible.exit_code(), 14);
        assert_eq!(
            ShimFailureClass::ProtocolIncompatible.as_str(),
            "protocol_incompatible"
        );
        assert_eq!(ShimFailureClass::ProtocolIncompatible.wire_code(), 15_005);
        assert_eq!(
            ShimFailureClass::ProtocolIncompatible.record_message(),
            "daemon protocol or _health contract is incompatible with this shim"
        );
    }
}
