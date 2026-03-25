//! Workspace file hash tracking for offline change detection.
//!
//! Provides SHA-256 hash recording for tracked workspace files and compares
//! stored hashes against current on-disk state to surface changes that
//! occurred while the daemon was not running.
//!
//! # How it fits in the lifecycle
//!
//! 1. After indexing or ingesting a file, call [`record_file_hash`] to persist
//!    the hash to the `file_hash` table.
//! 2. On daemon startup (after DB hydration), call [`detect_offline_changes`]
//!    to discover files that were added, modified, or deleted while the daemon
//!    was offline.  Pass the results to the sync pipeline for re-indexing.
//! 3. The file watcher calls [`record_file_hash`] after processing each
//!    [`WatcherEvent`](crate::models::WatcherEvent) to keep hashes current.

use std::path::Path;

use sha2::{Digest, Sha256};
use tracing::{debug, warn};

use crate::db::queries::CodeGraphQueries;
use crate::errors::{EngramError, SystemError};

// ── Default exclusion patterns (mirrors watcher defaults) ─────────────────────

/// Directory prefixes excluded from file tracking (forward-slash normalised).
const EXCLUDED_PREFIXES: &[&str] = &[".engram/", ".git/", "node_modules/", "target/", ".env"];

// ── Public types ──────────────────────────────────────────────────────────────

/// The kind of change detected for a workspace file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChangeKind {
    /// File exists on disk but has no stored hash (new since last daemon run).
    Added,
    /// File exists on disk and has a stored hash, but the hash differs.
    Modified,
    /// File has a stored hash but no longer exists on disk.
    Deleted,
}

/// A single file change detected by [`detect_offline_changes`].
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Workspace-relative file path (forward-slash separated).
    pub path: String,
    /// The kind of change.
    pub kind: FileChangeKind,
    /// The previously stored hash, if any (`None` for `Added`).
    pub previous_hash: Option<String>,
    /// The current on-disk hash, if any (`None` for `Deleted`).
    pub current_hash: Option<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Compute the SHA-256 hex digest of a file's contents.
///
/// # Errors
///
/// Returns `EngramError` if the file cannot be read.
pub fn compute_file_hash(path: &Path) -> Result<String, EngramError> {
    let content = std::fs::read(path).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("cannot read file for hashing ({}): {e}", path.display()),
        })
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(hex::encode(hasher.finalize()))
}

/// Compute the SHA-256 hex digest of a file and store it in the database.
///
/// `rel_path` must be the workspace-relative, forward-slash-separated path
/// (e.g., `"src/main.rs"`).  `abs_path` is the absolute path used for
/// reading.
///
/// Idempotent: calling this multiple times for the same path replaces the
/// stored record with the current state.
///
/// # Errors
///
/// Returns `EngramError` if the file cannot be read or the database write
/// fails.
pub async fn record_file_hash(
    rel_path: &str,
    abs_path: &Path,
    queries: &CodeGraphQueries,
) -> Result<(), EngramError> {
    let metadata = std::fs::metadata(abs_path).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("cannot stat file ({}): {e}", abs_path.display()),
        })
    })?;

    let hash = compute_file_hash(abs_path)?;
    let normalized = rel_path.replace('\\', "/");

    debug!(
        path = %normalized,
        hash = %&hash[..8],
        size = metadata.len(),
        "file_tracker: recording hash"
    );

    queries
        .upsert_file_hash(&normalized, &hash, metadata.len())
        .await
}

/// Walk `workspace_root`, compare every non-excluded file against stored
/// hashes, and return the set of files that changed while the daemon was
/// offline.
///
/// Three change kinds are detected:
/// - **Added**: file present on disk, no stored hash.
/// - **Modified**: file present on disk, stored hash differs from current.
/// - **Deleted**: stored hash exists, file no longer present on disk.
///
/// Paths inside `.engram/`, `.git/`, `target/`, `node_modules/`, and `.env*`
/// are excluded (mirroring [`crate::daemon::watcher::WatcherConfig`] defaults).
///
/// # Errors
///
/// Returns `EngramError` if the database query fails.  File I/O errors for
/// individual files are logged as warnings and skipped rather than aborting
/// the scan.
#[tracing::instrument(skip(queries), fields(workspace = %workspace_root.display()))]
pub async fn detect_offline_changes(
    workspace_root: &Path,
    queries: &CodeGraphQueries,
) -> Result<Vec<FileChange>, EngramError> {
    // Load all stored hashes into a map for O(1) lookup.
    let stored = queries.get_all_file_hashes().await?;
    let mut stored_map: std::collections::HashMap<String, String> = stored
        .into_iter()
        .map(|r| (r.file_path, r.content_hash))
        .collect();

    let mut changes = Vec::new();

    // Walk the workspace, skipping excluded paths.
    let disk_files = collect_workspace_files(workspace_root);

    for abs_path in disk_files {
        let rel = match abs_path.strip_prefix(workspace_root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        let current_hash = match compute_file_hash(&abs_path) {
            Ok(h) => h,
            Err(e) => {
                warn!(path = %rel, error = %e, "file_tracker: cannot hash file — skipping");
                continue;
            }
        };

        match stored_map.remove(&rel) {
            None => {
                // File on disk but no stored hash → Added.
                changes.push(FileChange {
                    path: rel,
                    kind: FileChangeKind::Added,
                    previous_hash: None,
                    current_hash: Some(current_hash),
                });
            }
            Some(stored_hash) if stored_hash != current_hash => {
                // Hash mismatch → Modified.
                changes.push(FileChange {
                    path: rel,
                    kind: FileChangeKind::Modified,
                    previous_hash: Some(stored_hash),
                    current_hash: Some(current_hash),
                });
            }
            Some(_) => {
                // Hashes match → no change.
                debug!(path = %rel, "file_tracker: unchanged");
            }
        }
    }

    // Any entries remaining in stored_map have no matching disk file → Deleted.
    for (path, stored_hash) in stored_map {
        changes.push(FileChange {
            path,
            kind: FileChangeKind::Deleted,
            previous_hash: Some(stored_hash),
            current_hash: None,
        });
    }

    tracing::info!(
        added = changes
            .iter()
            .filter(|c| c.kind == FileChangeKind::Added)
            .count(),
        modified = changes
            .iter()
            .filter(|c| c.kind == FileChangeKind::Modified)
            .count(),
        deleted = changes
            .iter()
            .filter(|c| c.kind == FileChangeKind::Deleted)
            .count(),
        "file_tracker: offline change detection complete"
    );

    Ok(changes)
}

// ── Internals ─────────────────────────────────────────────────────────────────

/// Recursively collect all non-excluded files under `workspace_root`.
fn collect_workspace_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    collect_recursive(root, root, &mut files);
    files.sort();
    files
}

fn collect_recursive(root: &Path, dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            warn!(dir = %dir.display(), error = %e, "file_tracker: cannot read directory");
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Compute workspace-relative forward-slash path for exclusion check.
        let rel = match path.strip_prefix(root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        if is_excluded(&rel) {
            debug!(path = %rel, "file_tracker: excluded");
            continue;
        }

        if path.is_dir() {
            collect_recursive(root, &path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

/// Return `true` when a workspace-relative path falls under an excluded prefix.
fn is_excluded(rel: &str) -> bool {
    EXCLUDED_PREFIXES.iter().any(|prefix| {
        let stem = prefix.trim_end_matches('/');
        rel == stem || rel.starts_with(&format!("{stem}/"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excluded_git_dir() {
        assert!(is_excluded(".git/HEAD"));
        assert!(is_excluded(".git"));
    }

    #[test]
    fn excluded_engram_dir() {
        assert!(is_excluded(".engram/db/main"));
        assert!(is_excluded(".engram"));
    }

    #[test]
    fn excluded_target_dir() {
        assert!(is_excluded("target/debug/engram"));
    }

    #[test]
    fn excluded_node_modules() {
        assert!(is_excluded("node_modules/lodash/index.js"));
    }

    #[test]
    fn not_excluded_src() {
        assert!(!is_excluded("src/main.rs"));
        assert!(!is_excluded("README.md"));
    }

    #[test]
    fn not_excluded_dotenv_like_dir_named_targeting() {
        // Only ".env" prefix matches, not arbitrary names starting with 't'.
        assert!(!is_excluded("targeting/foo.rs"));
    }
}
