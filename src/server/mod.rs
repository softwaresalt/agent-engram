//! Server application state.
//!
//! The [`state`] sub-module defines [`state::AppState`] and [`state::SharedState`],
//! which are used by the IPC daemon as the runtime context for tool dispatch,
//! workspace management, and connection tracking.
//!
//! The legacy HTTP/SSE transport sub-modules (`router`, `mcp`, `sse`) and the
//! `legacy-sse` Cargo feature that gated them have been retired. The daemon
//! supports exactly three transport surfaces: direct IPC, the `engram` CLI,
//! and stdio MCP via `engram shim`. See ADR-0016 (superseded).

pub mod observability;
pub mod state;
