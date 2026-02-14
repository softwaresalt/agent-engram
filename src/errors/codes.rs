//! Numeric error code constants for structured MCP error responses.
//!
//! Ranges: 1xxx workspace, 2xxx hydration, 3xxx task, 4xxx query, 5xxx system.

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

/// Task error codes
pub const TASK_NOT_FOUND: u16 = 3001;
pub const INVALID_STATUS: u16 = 3002;
pub const CYCLIC_DEPENDENCY: u16 = 3003;
pub const BLOCKER_EXISTS: u16 = 3004;
pub const TASK_TITLE_EMPTY: u16 = 3005;

/// Query error codes
pub const QUERY_TOO_LONG: u16 = 4001;
pub const MODEL_NOT_LOADED: u16 = 4002;
pub const SEARCH_FAILED: u16 = 4003;

/// System error codes
pub const DATABASE_ERROR: u16 = 5001;
pub const FLUSH_FAILED: u16 = 5002;
pub const RATE_LIMITED: u16 = 5003;
pub const SHUTTING_DOWN: u16 = 5004;
