//! Plugin installer: workspace setup and management commands.
//!
//! Implements the `install`, `update`, `reinstall`, and `uninstall` subcommands.
//! The installer creates the `.engram/` directory structure, generates MCP
//! configuration files, and manages the plugin lifecycle for each workspace.

pub mod templates;

use crate::errors::EngramError;

/// Install the engram plugin into the current workspace.
///
/// Creates `.engram/` directory structure, generates MCP configuration,
/// and performs a health check to verify the installation is functional.
///
/// # Errors
///
/// Returns [`EngramError::InstallError`] if the workspace directory cannot be
/// created, the configuration template cannot be written, or the health check fails.
#[allow(clippy::unused_async)]
pub async fn install() -> Result<(), EngramError> {
    // TODO(T059): implement install command
    todo!("Phase 6: install command")
}

/// Update the engram plugin runtime artifacts in the current workspace.
///
/// Replaces binary references and configuration templates while preserving
/// existing workspace data files (tasks.md, config.toml, graph state).
///
/// # Errors
///
/// Returns [`EngramError::InstallError`] if the update operation fails.
#[allow(clippy::unused_async)]
pub async fn update() -> Result<(), EngramError> {
    // TODO(T061): implement update command
    todo!("Phase 6: update command")
}

/// Reinstall the engram plugin, cleaning runtime artifacts while preserving data.
///
/// Cleans the runtime directory, re-creates the `.engram/` structure, and
/// rehydrates workspace data from existing files.
///
/// # Errors
///
/// Returns [`EngramError::InstallError`] if the reinstall operation fails.
#[allow(clippy::unused_async)]
pub async fn reinstall() -> Result<(), EngramError> {
    // TODO(T062): implement reinstall command
    todo!("Phase 6: reinstall command")
}

/// Uninstall the engram plugin from the current workspace.
///
/// Stops any running daemon (via `_shutdown` IPC message), then removes plugin
/// artifacts. When `keep_data` is `true`, workspace data files are preserved.
///
/// # Errors
///
/// Returns [`EngramError::InstallError`] if the uninstall operation fails.
#[allow(clippy::unused_async)]
pub async fn uninstall(_keep_data: bool) -> Result<(), EngramError> {
    // TODO(T063): implement uninstall command
    todo!("Phase 6: uninstall command")
}
