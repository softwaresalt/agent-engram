//! Numeric error code constants for structured MCP error responses.
//!
//! Ranges: 1xxx workspace, 2xxx hydration, 4xxx query,
//! 5xxx system, 6xxx config, 7xxx code graph, 8xxx IPC/daemon,
//! 9xxx installer, 10xxx registry, 11xxx ingestion, 12xxx git graph.

/// Workspace error codes
pub const WORKSPACE_NOT_FOUND: u16 = 1001;
pub const NOT_A_GIT_ROOT: u16 = 1002;
pub const WORKSPACE_NOT_SET: u16 = 1003;
pub const WORKSPACE_ALREADY_ACTIVE: u16 = 1004;
pub const WORKSPACE_LIMIT_REACHED: u16 = 1005;

/// Hydration error codes
pub const HYDRATION_FAILED: u16 = 2001;
pub const SCHEMA_MISMATCH: u16 = 2002;
pub const CORRUPTED_STATE: u16 = 2003;
pub const STALE_WORKSPACE: u16 = 2004;

/// Query error codes
pub const QUERY_TOO_LONG: u16 = 4001;
pub const MODEL_NOT_LOADED: u16 = 4002;
pub const SEARCH_FAILED: u16 = 4003;
pub const QUERY_EMPTY: u16 = 4004;
/// Sandboxed graph query error codes (4010–4019)
pub const QUERY_REJECTED: u16 = 4010;
pub const QUERY_TIMEOUT: u16 = 4011;
pub const QUERY_INVALID: u16 = 4012;

/// System error codes
pub const DATABASE_ERROR: u16 = 5001;
pub const FLUSH_FAILED: u16 = 5002;
pub const RATE_LIMITED: u16 = 5003;
pub const SHUTTING_DOWN: u16 = 5004;
pub const INVALID_PARAMS: u16 = 5005;
pub const MODEL_LOAD_FAILED: u16 = 5006;

/// Config error codes
pub const CONFIG_PARSE_ERROR: u16 = 6001;
pub const CONFIG_INVALID_VALUE: u16 = 6002;
pub const UNKNOWN_CONFIG_KEY: u16 = 6003;

/// Code graph error codes
pub const PARSE_ERROR: u16 = 7001;
pub const UNSUPPORTED_LANGUAGE: u16 = 7002;
pub const INDEX_IN_PROGRESS: u16 = 7003;
pub const SYMBOL_NOT_FOUND: u16 = 7004;
/// 7005 is reserved for future use.
pub const FILE_TOO_LARGE: u16 = 7006;
pub const SYNC_CONFLICT: u16 = 7007;

/// IPC and daemon error codes (8xxx)
pub const IPC_CONNECTION_FAILED: u16 = 8001;
pub const IPC_SEND_FAILED: u16 = 8002;
pub const IPC_RECEIVE_FAILED: u16 = 8003;
pub const IPC_TIMEOUT: u16 = 8004;
pub const DAEMON_SPAWN_FAILED: u16 = 8005;
pub const DAEMON_NOT_READY: u16 = 8006;
pub const LOCK_ACQUISITION_FAILED: u16 = 8007;
pub const LOCK_ALREADY_HELD: u16 = 8008;
pub const WATCHER_INIT_FAILED: u16 = 8009;

/// Installer error codes (9xxx)
pub const INSTALL_FAILED: u16 = 9001;
pub const UPDATE_FAILED: u16 = 9002;
pub const UNINSTALL_FAILED: u16 = 9003;
pub const ALREADY_INSTALLED: u16 = 9004;
pub const NOT_INSTALLED: u16 = 9005;

/// Content registry error codes (10xxx)
pub const REGISTRY_PARSE_FAILED: u16 = 10_001;
pub const REGISTRY_VALIDATION_FAILED: u16 = 10_002;

/// Content ingestion error codes (11xxx)
pub const INGESTION_FAILED: u16 = 11_001;

/// Git commit graph error codes (12xxx)
pub const GIT_NOT_FOUND: u16 = 12_001;
pub const GIT_ACCESS_ERROR: u16 = 12_002;
