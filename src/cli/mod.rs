//! CLI module: subcommand dispatch and infrastructure.
//!
//! Provides `GlobalFlags`, `OutputFormatter`, the IPC runner, and all
//! subcommand implementations. Called from `src/bin/engram.rs` when a
//! CLI subcommand variant is matched.

pub mod commands;
pub mod direct;
pub mod flags;
pub mod output;
pub mod runner;
