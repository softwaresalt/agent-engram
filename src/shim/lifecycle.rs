//! Shim lifecycle: daemon health-check, spawn, and wait-for-ready logic.
//!
//! Before forwarding the first request the shim checks whether a daemon is
//! already running by sending an `_health` IPC message. If the check fails
//! (no daemon running), the shim spawns a new daemon process via
//! `std::process::Command` and waits with exponential backoff until the daemon
//! reports `Ready`.

// TODO(T027): implement daemon health-check + spawn-with-backoff
// TODO(T028): implement spawn guard — acquire lock before spawn, detect existing daemon
