use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::errors::WorkspaceError;

/// Canonicalize and validate a workspace path; ensures .git exists at root.
pub fn canonicalize_workspace(path: &str) -> Result<PathBuf, WorkspaceError> {
    let candidate = Path::new(path);
    if !candidate.exists() {
        return Err(WorkspaceError::NotFound {
            path: path.to_string(),
        });
    }

    let canonical = candidate
        .canonicalize()
        .map_err(|_| WorkspaceError::NotFound {
            path: path.to_string(),
        })?;

    if !canonical.join(".git").is_dir() {
        return Err(WorkspaceError::NotGitRoot {
            path: canonical.display().to_string(),
        });
    }

    Ok(canonical)
}

/// Compute a stable SHA256 hash for the workspace path.
pub fn workspace_hash(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}
