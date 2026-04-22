//! Shared shim/daemon IPC protocol version helpers.

use crate::errors::{EngramError, IpcError};

/// Current IPC protocol version understood by both the shim and daemon.
pub const ENGRAM_PROTOCOL_VERSION: u32 = 1;

/// Build identifier exposed in daemon handshake responses.
pub const ENGRAM_BUILD_HASH: &str = match option_env!("ENGRAM_BUILD_HASH") {
    Some(build_hash) => build_hash,
    None => env!("CARGO_PKG_VERSION"),
};

/// Validate that the daemon protocol version matches the shim expectation.
///
/// # Errors
///
/// Returns [`EngramError::Ipc`] with [`IpcError::VersionMismatch`] when the
/// daemon reports a different protocol version.
pub fn ensure_protocol_compatible(actual: u32) -> Result<(), EngramError> {
    if actual == ENGRAM_PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(EngramError::Ipc(IpcError::VersionMismatch {
            expected: ENGRAM_PROTOCOL_VERSION,
            actual,
        }))
    }
}
