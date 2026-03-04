//! rmcp `StdioTransport` setup and `ServerHandler` implementation for the shim.
//!
//! The shim's `ServerHandler` does not execute tools locally; it forwards every
//! `call_tool` request to the workspace daemon via the IPC client and returns
//! the daemon's response verbatim.

// TODO(T029): implement rmcp ServerHandler — forwards to ipc_client
