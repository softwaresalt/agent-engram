//! Read-server layout, identity, and generation-ID minting for preflight.
//!
//! This is the single derivation of the read-server identity, generations
//! root, runtime root, and activation deadline.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::{ExpectedIdentity, GenerationId, GenerationIdError};
use crate::errors::WorkspaceError;

/// Deadline for read-server generation activation.
pub const READ_SERVER_ACTIVATION_DEADLINE: Duration = Duration::from_secs(60);

/// Derived read-server layout; only [`read_server_layout`] constructs it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReadServerLayout {
    /// Canonical workspace root, exactly as `canonicalize_workspace` returned.
    pub workspace: PathBuf,
    /// Resolved branch, or `default` when it cannot be resolved.
    pub branch: String,
    /// Workspace hash for `workspace` and `branch`.
    pub workspace_id: String,
    /// `<workspace>/.engram/generations`.
    pub generations_root: PathBuf,
    /// `<generations_root>/runtime`.
    pub runtime_root: PathBuf,
    /// Activation deadline, always [`READ_SERVER_ACTIVATION_DEADLINE`].
    pub activation_deadline: Duration,
}

impl ReadServerLayout {
    /// Identity activation must match, built from the layout's own fields.
    #[must_use]
    pub fn expected_identity(&self) -> ExpectedIdentity {
        unimplemented!("Worker: expected_identity")
    }
}

/// Derive the read-server layout for `workspace` without any writes.
///
/// # Errors
///
/// Returns [`WorkspaceError`] from workspace canonicalization.
pub fn read_server_layout(workspace: &str) -> Result<ReadServerLayout, WorkspaceError> {
    let _ = workspace;
    unimplemented!("Worker: read_server_layout")
}

/// Mint a pure `preflight-<unix_ms>-<pid>` generation ID.
///
/// # Errors
///
/// Returns [`GenerationIdError`] if the minted value fails validation.
pub fn mint_generation_id(now: SystemTime) -> Result<GenerationId, GenerationIdError> {
    let _ = now;
    unimplemented!("Worker: mint_generation_id")
}
