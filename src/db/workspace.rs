use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::errors::WorkspaceError;

/// Strip the Windows extended-length path prefix (`\\?\`) from a canonicalized path.
///
/// `std::fs::canonicalize` on Windows returns paths prefixed with `\\?\` for
/// extended-length path support. This prefix causes hash instability (the same
/// workspace produces a different SHA-256 depending on how the path was derived)
/// and can cause compatibility issues with crates that do not handle UNC paths.
/// Stripping it gives a regular absolute path while preserving full path fidelity
/// for paths under 260 characters, which all workspace roots in practice are.
fn normalize_canonical(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};

        // Inspect the leading path component to detect any verbatim prefix
        // (`\\?\C:\...` or `\\?\UNC\server\share\...`).  String round-trips
        // miss the UNC variant and produce non-canonical output on non-UTF-8
        // paths; component inspection is exact.
        if let Some(Component::Prefix(prefix_component)) = path.components().next() {
            let rebuilt: Option<PathBuf> = match prefix_component.kind() {
                Prefix::VerbatimDisk(drive) => {
                    // \\?\C:\rest  →  C:\rest
                    let suffix: PathBuf = path.components().skip(1).collect();
                    Some(PathBuf::from(format!("{}:\\", drive as char)).join(suffix))
                }
                Prefix::VerbatimUNC(server, share) => {
                    // \\?\UNC\server\share\rest  →  \\server\share\rest
                    let server = server.to_string_lossy();
                    let share = share.to_string_lossy();
                    let suffix: PathBuf = path.components().skip(1).collect();
                    Some(PathBuf::from(format!(r"\\{server}\{share}")).join(suffix))
                }
                Prefix::Verbatim(inner) => {
                    // \\?\other  →  other (best-effort)
                    let inner = inner.to_string_lossy();
                    let suffix: PathBuf = path.components().skip(1).collect();
                    Some(PathBuf::from(inner.as_ref()).join(suffix))
                }
                _ => None,
            };
            if let Some(p) = rebuilt {
                return p;
            }
        }
    }
    path
}

/// Canonicalize and validate a workspace path; ensures .git exists at root.
pub fn canonicalize_workspace(path: &str) -> Result<PathBuf, WorkspaceError> {
    let candidate = Path::new(path);
    if !candidate.exists() {
        return Err(WorkspaceError::NotFound {
            path: path.to_string(),
        });
    }

    let canonical =
        normalize_canonical(
            candidate
                .canonicalize()
                .map_err(|_| WorkspaceError::NotFound {
                    path: path.to_string(),
                })?,
        );

    if !canonical.join(".git").is_dir() {
        return Err(WorkspaceError::NotGitRoot {
            path: canonical.display().to_string(),
        });
    }

    Ok(canonical)
}

/// Compute a stable SHA256 hash for the workspace path.
///
/// `branch` is accepted so callers pass the active branch when constructing
/// `workspace_id`. The branch is not yet included in the digest —
/// embedding it is the implementation task for TASK-009.04.
///
/// # Worker instruction
///
/// Include `branch` in the SHA-256 digest so `workspace_id` uniquely
/// identifies `(path, branch)` pairs:
/// ```text
/// hasher.update(path.to_string_lossy().as_bytes());
/// hasher.update(b":");
/// hasher.update(branch.as_bytes());
/// ```
pub fn workspace_hash(path: &Path, branch: &str) -> String {
    let _ = branch; // TODO(009.04): include branch in digest
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

/// Resolve the current git branch name for the workspace.
///
/// Reads `.git/HEAD` directly (no subprocess) and extracts the branch name.
/// Returns a truncated commit SHA when HEAD is detached.
pub fn resolve_git_branch(workspace: &Path) -> Result<String, WorkspaceError> {
    let head_path = workspace.join(".git").join("HEAD");
    let head_content =
        std::fs::read_to_string(&head_path).map_err(|_| WorkspaceError::NotGitRoot {
            path: workspace.display().to_string(),
        })?;

    let head = head_content.trim();
    if let Some(branch) = head.strip_prefix("ref: refs/heads/") {
        Ok(sanitize_branch_for_path(branch))
    } else {
        // Detached HEAD: use first 12 chars of the commit SHA
        Ok(head.chars().take(12).collect())
    }
}

/// Sanitize a git branch name for use as a filesystem directory name.
///
/// Replaces `/` with `__` so branches like `feature/foo` become `feature__foo`.
pub(crate) fn sanitize_branch_for_path(branch: &str) -> String {
    branch.replace('/', "__")
}

/// Resolve the data directory for database storage.
///
/// Priority:
/// 1. `ENGRAM_DATA_DIR` env var (resolved relative to workspace if not absolute)
/// 2. Default: `{workspace}/.engram`
pub fn resolve_data_dir(workspace: &Path) -> PathBuf {
    if let Ok(env_dir) = std::env::var("ENGRAM_DATA_DIR") {
        let p = PathBuf::from(&env_dir);
        if p.is_absolute() {
            p
        } else {
            workspace.join(p)
        }
    } else {
        workspace.join(".engram")
    }
}
