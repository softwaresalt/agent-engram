//! Daemon IPC server: `interprocess` `LocalSocketListener` accept loop.
//!
//! Listens for shim connections on the workspace-scoped IPC endpoint
//! (Unix domain socket or Windows named pipe), reads newline-delimited
//! JSON-RPC requests, dispatches to `tools::dispatch`, and writes responses.

// TODO(T016): implement IPC accept loop — LocalSocketListener, JSON-RPC framing
// TODO(T017): implement IPC endpoint naming (Unix socket / Windows named pipe)
// TODO(T033): implement _health IPC handler
// TODO(T052): implement _shutdown IPC handler
