//! Daemon error transport seam: the single domain-error to wire-error
//! conversion used by the IPC surface.
//!
//! The domain error type is [`EngramError`]; the wire shape is the existing
//! [`ErrorResponse`] envelope produced by [`EngramError::to_response`]. This
//! module does not define a parallel error shape — it is the named seam through
//! which the daemon converts domain failures into transport payloads.

use serde_json::json;

use crate::daemon::protocol::IpcError;
use crate::errors::{EngramError, ErrorResponse};

/// Domain error carried by daemon operations.
pub type DomainError = EngramError;

/// Wire representation of a [`DomainError`].
pub type WireError = ErrorResponse;

/// JSON-RPC internal-error code used for every domain failure surfaced over IPC.
const JSONRPC_INTERNAL_ERROR: i32 = -32_603;

/// Convert a domain error into its wire envelope.
// The seam contract transfers ownership of the domain error to the transport
// layer, so the by-value parameter is deliberate.
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn to_wire(error: DomainError) -> WireError {
    error.to_response()
}

/// Convert a domain error into the JSON-RPC error object sent on the IPC wire.
///
/// The stable Engram error code is preserved in `data.engram_code` so clients
/// can distinguish domain failures behind the single JSON-RPC internal-error code.
#[must_use]
pub fn to_ipc_error(error: DomainError) -> IpcError {
    let wire = to_wire(error);
    IpcError {
        code: JSONRPC_INTERNAL_ERROR,
        message: wire.error.message,
        data: Some(json!({ "engram_code": wire.error.code })),
    }
}
