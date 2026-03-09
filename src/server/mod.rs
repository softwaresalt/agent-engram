//! Server application state and (optionally) HTTP/SSE transport layer.
//!
//! The [`state`] sub-module defines [`state::AppState`] and [`state::SharedState`],
//! which are used by the IPC daemon as the runtime context for tool dispatch,
//! workspace management, and connection tracking.
//!
//! The HTTP/SSE transport sub-modules (`router`, `mcp`, `sse`) are preserved for
//! compatibility but are only compiled when the **`legacy-sse`** Cargo feature is
//! enabled. Default builds use the IPC transport exclusively. See ADR-0016.

#[cfg(feature = "legacy-sse")]
pub mod mcp;
pub mod observability;
#[cfg(feature = "legacy-sse")]
pub mod router;
#[cfg(feature = "legacy-sse")]
pub mod sse;
pub mod state;
