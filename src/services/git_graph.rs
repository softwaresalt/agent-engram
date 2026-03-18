//! Git commit graph indexing and retrieval service.
//!
//! This entire module is gated on the `git-graph` feature flag
//! via the module declaration in `services/mod.rs`.
//! All `git2` operations execute inside `tokio::task::spawn_blocking`
//! to avoid blocking the async runtime.

use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};
use git2::{Delta, DiffOptions, Repository, Sort};

use crate::{
    db::queries::Queries,
    errors::{EngramError, GitGraphError},
    models::{ChangeRecord, ChangeType, CommitNode},
};

/// Default number of commits to walk when no depth is specified.
const DEFAULT_DEPTH: u32 = 500;

/// Number of context lines to include around each diff hunk.
const CONTEXT_LINES: u32 = 20;

/// Maximum diff lines per file before truncation.
const MAX_DIFF_LINES: usize = 500;

/// Summary returned by [`index_git_history`].
#[derive(Debug, serde::Serialize)]
pub struct IndexSummary {
    /// Total commits walked and stored.
    pub commits_indexed: u32,
    /// Commits that were newly stored (vs already present).
    pub new_commits: u32,
    /// Total `ChangeRecord` entries across all commits.
    pub total_changes: u32,
    /// Wall-clock milliseconds for the full indexing run.
    pub elapsed_ms: u64,
}

/// Index the git history of the workspace into `commit_node` records.
///
/// # Errors
///
/// Returns [`EngramError::GitGraph`] with [`GitGraphError::NotFound`] when
/// `workspace_path` is not inside a git repository, and
/// [`GitGraphError::AccessError`] for any other `git2` failure.
pub async fn index_git_history(
    db: &Queries,
    workspace_path: &Path,
    depth: u32,
    force: bool,
) -> Result<IndexSummary, EngramError> {
    let ws = workspace_path.to_path_buf();
    let effective_depth = if depth == 0 { DEFAULT_DEPTH } else { depth };

    let start = std::time::Instant::now();

    // Resolve the last indexed commit to enable incremental sync.
    let last_hash: Option<String> = if force {
        None
    } else {
        db.latest_indexed_commit_hash().await.unwrap_or(None)
    };

    // git2 is not Send, so all repo operations go into spawn_blocking.
    let nodes = tokio::task::spawn_blocking(move || walk_commits(&ws, effective_depth, last_hash))
        .await
        .map_err(|e| {
            EngramError::GitGraph(GitGraphError::AccessError {
                reason: format!("spawn_blocking join error: {e}"),
            })
        })??;

    let new_commits = u32::try_from(nodes.len()).unwrap_or(u32::MAX);
    let total_changes: u32 = nodes
        .iter()
        .map(|n| u32::try_from(n.changes.len()).unwrap_or(u32::MAX))
        .fold(0_u32, u32::saturating_add);

    for node in &nodes {
        db.upsert_commit_node(node).await?;
    }

    Ok(IndexSummary {
        commits_indexed: new_commits,
        new_commits,
        total_changes,
        elapsed_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

/// Walk commits from HEAD up to `depth`, stopping at `stop_hash` when set.
fn walk_commits(
    workspace_path: &PathBuf,
    depth: u32,
    stop_hash: Option<String>,
) -> Result<Vec<CommitNode>, EngramError> {
    let repo = Repository::discover(workspace_path).map_err(|_| GitGraphError::NotFound {
        path: workspace_path.display().to_string(),
    })?;

    let mut revwalk = repo.revwalk().map_err(|e| GitGraphError::AccessError {
        reason: format!("revwalk creation failed: {e}"),
    })?;

    revwalk
        .push_head()
        .map_err(|e| GitGraphError::AccessError {
            reason: format!("push_head failed: {e}"),
        })?;

    revwalk
        .set_sorting(Sort::TIME)
        .map_err(|e| GitGraphError::AccessError {
            reason: format!("set_sorting failed: {e}"),
        })?;

    let mut nodes = Vec::new();

    for (i, oid_result) in revwalk.enumerate() {
        if i >= depth as usize {
            break;
        }

        let oid = oid_result.map_err(|e| GitGraphError::AccessError {
            reason: format!("revwalk oid error at index {i}: {e}"),
        })?;

        let hash = oid.to_string();

        // Stop at the previously indexed commit for incremental sync.
        if stop_hash.as_deref() == Some(hash.as_str()) {
            break;
        }

        let commit = repo
            .find_commit(oid)
            .map_err(|e| GitGraphError::AccessError {
                reason: format!("find_commit failed for {hash}: {e}"),
            })?;

        let parent_hashes: Vec<String> = commit.parent_ids().map(|p| p.to_string()).collect();
        let changes = extract_changes(&repo, &commit)?;
        let short_hash = hash[..7.min(hash.len())].to_string();

        let timestamp_secs = commit.author().when().seconds();
        let timestamp = Utc
            .timestamp_opt(timestamp_secs, 0)
            .single()
            .unwrap_or_else(Utc::now);

        nodes.push(CommitNode {
            id: format!("commit_node:{hash}"),
            hash,
            short_hash,
            author_name: commit.author().name().unwrap_or("Unknown").to_string(),
            author_email: commit.author().email().unwrap_or("").to_string(),
            timestamp,
            message: commit.message().unwrap_or("").trim().to_string(),
            parent_hashes,
            changes,
        });
    }

    Ok(nodes)
}

/// Extract per-file [`ChangeRecord`]s from a single commit.
///
/// Diffs against the first parent (or an empty tree for root commits).
/// Merge commits are handled by diffing only against the first parent.
fn extract_changes(
    repo: &Repository,
    commit: &git2::Commit<'_>,
) -> Result<Vec<ChangeRecord>, EngramError> {
    let commit_tree = commit.tree().map_err(|e| GitGraphError::AccessError {
        reason: format!("commit tree error: {e}"),
    })?;

    let parent_tree = if commit.parent_count() > 0 {
        let parent = commit.parent(0).map_err(|e| GitGraphError::AccessError {
            reason: format!("parent commit error: {e}"),
        })?;
        Some(parent.tree().map_err(|e| GitGraphError::AccessError {
            reason: format!("parent tree error: {e}"),
        })?)
    } else {
        None
    };

    let mut diff_opts = DiffOptions::new();
    diff_opts.context_lines(CONTEXT_LINES);

    let diff = repo
        .diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit_tree),
            Some(&mut diff_opts),
        )
        .map_err(|e| GitGraphError::AccessError {
            reason: format!("diff_tree_to_tree failed: {e}"),
        })?;

    let num_deltas = diff.deltas().count();
    let mut changes = Vec::with_capacity(num_deltas);

    for delta_idx in 0..num_deltas {
        let Some(delta) = diff.get_delta(delta_idx) else {
            continue;
        };

        let file_path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let change_type = match delta.status() {
            Delta::Added => ChangeType::Add,
            Delta::Deleted => ChangeType::Delete,
            Delta::Renamed => ChangeType::Rename,
            _ => ChangeType::Modify,
        };

        let (diff_snippet, lines_added, lines_removed) = extract_patch_text(&diff, delta_idx);

        changes.push(ChangeRecord {
            file_path,
            change_type,
            diff_snippet,
            old_line_start: None,
            new_line_start: None,
            lines_added,
            lines_removed,
        });
    }

    Ok(changes)
}

/// Extract diff text for a single delta, truncating at [`MAX_DIFF_LINES`].
fn extract_patch_text(diff: &git2::Diff<'_>, delta_idx: usize) -> (String, u32, u32) {
    let Ok(Some(patch)) = git2::Patch::from_diff(diff, delta_idx) else {
        return (String::new(), 0, 0);
    };

    let mut snippet = String::new();
    let mut lines_added: u32 = 0;
    let mut lines_removed: u32 = 0;
    let mut line_count: usize = 0;
    let mut truncated = false;

    for hunk_idx in 0..patch.num_hunks() {
        if truncated {
            break;
        }

        let Ok((_, hunk_line_count)) = patch.hunk(hunk_idx) else {
            continue;
        };

        snippet.push_str("@@ hunk @@\n");

        for line_idx in 0..hunk_line_count {
            if line_count >= MAX_DIFF_LINES {
                truncated = true;
                break;
            }

            let Ok(line) = patch.line_in_hunk(hunk_idx, line_idx) else {
                continue;
            };

            let origin = line.origin();
            let content = std::str::from_utf8(line.content()).unwrap_or("");
            snippet.push(origin);
            snippet.push_str(content);

            match origin {
                '+' => lines_added += 1,
                '-' => lines_removed += 1,
                _ => {}
            }

            line_count += 1;
        }
    }

    if truncated {
        snippet.push_str("\n[diff truncated]\n");
    }

    (snippet, lines_added, lines_removed)
}
