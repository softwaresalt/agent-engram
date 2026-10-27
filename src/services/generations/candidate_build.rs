//! Read-server layout, identity, and generation-ID minting for preflight.
//!
//! This is the single derivation of the read-server identity, generations
//! root, runtime root, and activation deadline.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::{ExpectedIdentity, GenerationId, GenerationIdError};
use crate::db::workspace::{canonicalize_workspace, resolve_git_branch, workspace_hash};
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
        ExpectedIdentity::new(self.branch.clone(), self.workspace_id.clone())
    }
}

/// Derive the read-server layout for `workspace` without any writes.
///
/// # Errors
///
/// Returns [`WorkspaceError`] from workspace canonicalization.
pub fn read_server_layout(workspace: &str) -> Result<ReadServerLayout, WorkspaceError> {
    // `canonicalize_workspace` reports a missing path as `NotGitRoot` (the
    // capability anchor fails first), so the missing case is classified here.
    if !Path::new(workspace).exists() {
        return Err(WorkspaceError::NotFound {
            path: workspace.to_owned(),
        });
    }
    let workspace = canonicalize_workspace(workspace)?;
    let branch = resolve_git_branch(&workspace).unwrap_or_else(|_| "default".to_owned());
    let workspace_id = workspace_hash(&workspace, &branch);
    let generations_root = workspace.join(".engram").join("generations");
    let runtime_root = generations_root.join("runtime");
    Ok(ReadServerLayout {
        workspace,
        branch,
        workspace_id,
        generations_root,
        runtime_root,
        activation_deadline: READ_SERVER_ACTIVATION_DEADLINE,
    })
}

/// Mint a pure `preflight-<unix_ms>-<pid>` generation ID.
///
/// # Errors
///
/// Returns [`GenerationIdError`] if the minted value fails validation.
pub fn mint_generation_id(now: SystemTime) -> Result<GenerationId, GenerationIdError> {
    let unix_ms = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis());
    GenerationId::new(format!("preflight-{unix_ms}-{}", std::process::id()))
}
