//! IPC client: connects to the daemon via `interprocess` `LocalSocketStream`.
//!
//! Sends a newline-delimited JSON-RPC request and reads the response with
//! a configurable timeout. Used exclusively by the shim transport.

// TODO(T026): implement IPC client — LocalSocketStream, JSON-RPC framing, timeout
