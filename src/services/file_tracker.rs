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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tracing::{debug, warn};

use crate::daemon::watcher::DEFAULT_EXCLUDE_PREFIXES;
use crate::db::queries::CodeGraphQueries;
use crate::errors::{EngramError, IngestionError, SystemError};

/// Files larger than this threshold are skipped during offline change detection
/// to prevent the daemon from reading gigabyte-scale log or binary files into
/// memory.  These files are not source code and do not benefit from hash-based
/// change tracking.
const MAX_HASH_FILE_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB

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
        EngramError::Ingestion(IngestionError::Failed {
            path: path.display().to_string(),
            reason: format!("cannot read file for hashing: {e}"),
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
        EngramError::Ingestion(IngestionError::Failed {
            path: abs_path.display().to_string(),
            reason: format!("cannot stat file for hash recording: {e}"),
        })
    })?;

    let hash = compute_file_hash(abs_path)?;
    let normalized = rel_path.replace('\\', "/");

    debug!(
        path = %normalized,
        hash = %utf8_prefix(&hash, 8),
        size = metadata.len(),
        "file_tracker: recording hash"
    );

    queries
        .upsert_file_hash(&normalized, &hash, metadata.len())
        .await
}

/// Store a file hash that was already computed by the caller.
///
/// Avoids the extra I/O of [`record_file_hash`] when the caller already holds
/// `content_hash` and `size_bytes` in memory (e.g., after `upsert_code_file`
/// in the sync loop which computed the hash for change-detection).
///
/// `rel_path` must be the workspace-relative, forward-slash-separated path.
///
/// # Errors
///
/// Returns `EngramError` if the database write fails.
pub async fn record_file_hash_precomputed(
    rel_path: &str,
    hash: &str,
    size_bytes: u64,
    queries: &CodeGraphQueries,
) -> Result<(), EngramError> {
    let normalized = rel_path.replace('\\', "/");

    debug!(
        path = %normalized,
        hash = %utf8_prefix(hash, 8),
        size = size_bytes,
        "file_tracker: recording precomputed hash"
    );

    queries
        .upsert_file_hash(&normalized, hash, size_bytes)
        .await
}

fn utf8_prefix(value: &str, max_chars: usize) -> &str {
    value
        .char_indices()
        .nth(max_chars)
        .map_or(value, |(byte_index, _)| &value[..byte_index])
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
/// A two-level fast-path avoids redundant SHA-256 computation: when the
/// stored `size_bytes` differs from the current file size, the file is
/// immediately classified as Modified without re-hashing.
///
/// All file system I/O runs inside [`tokio::task::spawn_blocking`] so the
/// Tokio event loop is never blocked by directory walks or file reads.
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
    // Load all stored hashes into a map keyed by relative path.
    let stored = queries.get_all_file_hashes().await?;
    let stored_map: HashMap<String, (String, u64)> = stored
        .into_iter()
        .map(|r| (r.file_path, (r.content_hash, r.size_bytes)))
        .collect();

    let root = workspace_root.to_path_buf();

    // All blocking I/O runs in a dedicated thread so the event loop is free.
    let changes = tokio::task::spawn_blocking(move || compare_disk_to_stored(&root, stored_map))
        .await
        .map_err(|e| {
            EngramError::System(SystemError::DatabaseError {
                reason: format!("file tracker scan task panicked: {e}"),
            })
        })?;

    let (added, modified, deleted) =
        changes
            .iter()
            .fold((0usize, 0, 0), |(a, m, d), c| match c.kind {
                FileChangeKind::Added => (a + 1, m, d),
                FileChangeKind::Modified => (a, m + 1, d),
                FileChangeKind::Deleted => (a, m, d + 1),
            });

    tracing::info!(
        added,
        modified,
        deleted,
        "file_tracker: offline change detection complete"
    );

    Ok(changes)
}

// ── Internals ─────────────────────────────────────────────────────────────────

/// Compare all non-excluded workspace files against `stored_map`, returning
/// the full set of `FileChange` entries.  Intended for use inside
/// `spawn_blocking`.
fn compare_disk_to_stored(
    root: &Path,
    mut stored_map: HashMap<String, (String, u64)>,
) -> Vec<FileChange> {
    let mut changes = Vec::new();
    let disk_files = collect_workspace_files(root);

    for abs_path in disk_files {
        let rel = match abs_path.strip_prefix(root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        // Read size first; if it differs from stored we know the file changed
        // without paying for a full SHA-256 pass.
        let current_size = match std::fs::metadata(&abs_path) {
            Ok(m) => m.len(),
            Err(e) => {
                warn!(path = %rel, error = %e, "file_tracker: cannot stat file — skipping");
                continue;
            }
        };

        // Skip files above the hash threshold: reading multi-GB log or binary
        // files into memory would stall the daemon for minutes on cold start.
        if current_size > MAX_HASH_FILE_BYTES {
            debug!(
                path = %rel,
                size_bytes = current_size,
                "file_tracker: skipping large file (above hash threshold)"
            );
            stored_map.remove(&rel);
            continue;
        }

        match stored_map.remove(&rel) {
            None => {
                // No stored hash → file was Added since last run.
                let current_hash = hash_or_skip(&abs_path, &rel);
                changes.push(FileChange {
                    path: rel,
                    kind: FileChangeKind::Added,
                    previous_hash: None,
                    current_hash,
                });
            }
            Some((stored_hash, stored_size)) => {
                if current_size == stored_size {
                    // Same size — verify via hash.
                    match compute_file_hash(&abs_path) {
                        Ok(current_hash) if current_hash != stored_hash => {
                            changes.push(FileChange {
                                path: rel,
                                kind: FileChangeKind::Modified,
                                previous_hash: Some(stored_hash),
                                current_hash: Some(current_hash),
                            });
                        }
                        Ok(_) => {
                            debug!(path = %rel, "file_tracker: unchanged");
                        }
                        Err(e) => {
                            warn!(path = %rel, error = %e, "file_tracker: cannot hash file — skipping");
                        }
                    }
                } else {
                    // Size differs — skip hashing, immediately Modified.
                    let current_hash = hash_or_skip(&abs_path, &rel);
                    changes.push(FileChange {
                        path: rel,
                        kind: FileChangeKind::Modified,
                        previous_hash: Some(stored_hash),
                        current_hash,
                    });
                }
            }
        }
    }

    // Remaining entries in stored_map have no disk counterpart → Deleted.
    for (path, (stored_hash, _)) in stored_map {
        changes.push(FileChange {
            path,
            kind: FileChangeKind::Deleted,
            previous_hash: Some(stored_hash),
            current_hash: None,
        });
    }

    changes
}

/// Hash a file, returning `Some(hex)` on success or `None` (with a warning)
/// on I/O error.
fn hash_or_skip(path: &Path, rel: &str) -> Option<String> {
    match compute_file_hash(path) {
        Ok(h) => Some(h),
        Err(e) => {
            warn!(path = %rel, error = %e, "file_tracker: cannot hash file — hash unavailable");
            None
        }
    }
}

/// Recursively collect all non-excluded files under `workspace_root`.
fn collect_workspace_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_recursive(root, root, &mut files);
    files.sort();
    files
}

fn collect_recursive(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
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
pub(crate) fn is_excluded(rel: &str) -> bool {
    DEFAULT_EXCLUDE_PREFIXES.iter().any(|prefix| {
        let stem = prefix.trim_end_matches('/');
        excluded_prefix_matches(rel, stem)
    })
}

#[cfg(not(windows))]
fn excluded_prefix_matches(rel: &str, stem: &str) -> bool {
    rel == stem
        || rel
            .strip_prefix(stem)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(windows)]
fn excluded_prefix_matches(rel: &str, stem: &str) -> bool {
    rel.eq_ignore_ascii_case(stem)
        || rel.get(..stem.len()).is_some_and(|prefix| {
            prefix.eq_ignore_ascii_case(stem)
                && rel
                    .as_bytes()
                    .get(stem.len())
                    .is_some_and(|byte| *byte == b'/')
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

    #[cfg(windows)]
    #[test]
    fn excluded_prefixes_are_ascii_case_insensitive_on_windows() {
        assert!(is_excluded(".GIT/objects/pack"));
        assert!(is_excluded("TARGET/debug/engram.exe"));
    }

    #[test]
    fn not_excluded_dotenv_like_dir_named_targeting() {
        // Only ".env" prefix matches, not arbitrary names starting with 't'.
        assert!(!is_excluded("targeting/foo.rs"));
    }
}
