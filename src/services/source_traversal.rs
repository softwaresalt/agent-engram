//! Shared source-tree traversal helpers.

use std::collections::HashSet;
use std::fs::FileType;
use std::path::{Component, Path, PathBuf};

use tracing::warn;

/// Outcome of a completeness-aware source-tree collection.
///
/// `complete` is `true` only when every directory reached under the traversal
/// root was successfully read. A single unreadable subtree (or a directory
/// whose canonical path could not be resolved) flips it to `false`, marking the
/// pass **non-authoritative**. Deletion reconciliation gates alias-stale
/// removal on `complete == true`, so a partial pass degrades to
/// physical-existence-only sweeping and can never wrongly delete a live record
/// (fail-closed; INV-2).
pub(crate) struct CollectedFiles {
    pub(crate) files: Vec<PathBuf>,
    pub(crate) complete: bool,
}

/// Collect files under `dir` whose paths satisfy `is_target_file`.
///
/// Directory symlinks are traversed only when their canonical target remains
/// under `workspace_root`; canonical directory visits are tracked so cycles and
/// aliases cannot recurse indefinitely or duplicate the same real tree.
///
/// Thin wrapper over [`collect_files_in_workspace_checked`] that discards the
/// authoritative-completeness flag; existing callers that only need the file
/// list stay unchanged.
#[must_use]
pub(crate) fn collect_files_in_workspace(
    dir: &Path,
    workspace_root: &Path,
    is_target_file: fn(&Path) -> bool,
) -> Vec<PathBuf> {
    collect_files_in_workspace_checked(dir, workspace_root, is_target_file).files
}

/// Completeness-aware variant of [`collect_files_in_workspace`].
///
/// Returns the collected files alongside a `complete` flag (see
/// [`CollectedFiles`]) that reports whether every directory was readable. The
/// deletion sweeps consume the flag to decide whether an alias-stale record may
/// be reconciled away this pass.
#[must_use]
pub(crate) fn collect_files_in_workspace_checked(
    dir: &Path,
    workspace_root: &Path,
    is_target_file: fn(&Path) -> bool,
) -> CollectedFiles {
    let mut files = Vec::new();
    let Ok(canonical_root) = workspace_root.canonicalize() else {
        // The workspace authority bound could not be established → the pass is
        // non-authoritative and yields no collected files (fail-closed).
        return CollectedFiles {
            files,
            complete: false,
        };
    };
    let mut visited = HashSet::new();
    let mut complete = true;
    collect_recursive(
        dir,
        &canonical_root,
        &mut visited,
        &mut files,
        &mut complete,
        is_target_file,
    );
    files.sort();
    files.dedup();
    CollectedFiles { files, complete }
}

/// Return true when `path` is a physical regular file whose canonical target
/// remains under `canonical_root`.
///
/// The final path component is inspected with `symlink_metadata`, so a file
/// symlink is not treated as live. Intermediate directory symlinks still work
/// when their resolved target remains inside the workspace.
#[must_use]
pub(crate) fn is_regular_file_in_workspace(path: &Path, canonical_root: &Path) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.file_type().is_file() {
        return false;
    }
    path.canonicalize()
        .is_ok_and(|canonical| canonical.starts_with(canonical_root))
}

fn collect_recursive(
    dir: &Path,
    canonical_root: &Path,
    visited: &mut HashSet<PathBuf>,
    files: &mut Vec<PathBuf>,
    complete: &mut bool,
    is_target_file: fn(&Path) -> bool,
) {
    let Ok(canonical_dir) = dir.canonicalize() else {
        // A directory we intended to descend into could not be resolved: the
        // pass is no longer authoritative (fail-closed).
        *complete = false;
        return;
    };
    if !canonical_dir.starts_with(canonical_root) || !visited.insert(canonical_dir) {
        // Out-of-bounds or already-visited (alias/cycle dedup): an intentional
        // skip, not a missed subtree — completeness is preserved.
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            warn!(
                dir = %dir.display(),
                %error,
                "skipping unreadable directory during source traversal"
            );
            // An unreadable subtree makes the pass non-authoritative.
            *complete = false;
            return;
        }
    };
    let mut entries: Vec<_> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let file_type = std::fs::symlink_metadata(&path).ok()?.file_type();
            Some((entry_rank(&file_type), path, file_type))
        })
        .collect();
    entries.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    for (_, path, file_type) in entries {
        if file_type.is_dir() {
            collect_recursive(
                &path,
                canonical_root,
                visited,
                files,
                complete,
                is_target_file,
            );
        } else if file_type.is_file() && is_target_file(&path) {
            files.push(path);
        } else if file_type.is_symlink() {
            collect_symlinked_directory(
                &path,
                canonical_root,
                visited,
                files,
                complete,
                is_target_file,
            );
        }
    }
}

fn collect_symlinked_directory(
    path: &Path,
    canonical_root: &Path,
    visited: &mut HashSet<PathBuf>,
    files: &mut Vec<PathBuf>,
    complete: &mut bool,
    is_target_file: fn(&Path) -> bool,
) {
    let Ok(canonical_target) = path.canonicalize() else {
        // A broken symlink resolves to nothing — it is not a missed readable
        // subtree, so completeness is preserved.
        return;
    };
    if !canonical_target.starts_with(canonical_root) || !canonical_target.is_dir() {
        return;
    }
    collect_recursive(
        path,
        canonical_root,
        visited,
        files,
        complete,
        is_target_file,
    );
}

fn entry_rank(file_type: &FileType) -> u8 {
    if file_type.is_dir() {
        0
    } else if file_type.is_file() {
        1
    } else if file_type.is_symlink() {
        2
    } else {
        3
    }
}

/// Reject a stored record path that would escape `workspace_root`.
///
/// Returns the workspace-relative [`PathBuf`] for a well-formed relative path,
/// or `None` for absolute paths, rooted paths, `..` traversal, or a Windows
/// drive prefix — the same escape guard the per-collector deletion sweeps used
/// before they were consolidated onto this shared reconciler.
#[must_use]
pub(crate) fn workspace_relative_path(rel_path: &str) -> Option<PathBuf> {
    let path = Path::new(rel_path);
    if path.is_absolute()
        || path.has_root()
        || path.components().any(|component| {
            component == Component::ParentDir || matches!(component, Component::Prefix(_))
        })
    {
        return None;
    }
    Some(path.to_path_buf())
}

/// Normalize an absolute collected path to the workspace-relative, `/`-separated
/// form the indexers persist as `ContentRecord.file_path`
/// (`strip_prefix(workspace_root)` + `replace('\\', "/")`), so collected paths
/// compare exactly against stored record paths.
fn normalize_collected_rel_path(absolute: &Path, workspace_root: &Path) -> Option<String> {
    absolute
        .strip_prefix(workspace_root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

/// Fail-closed reconciliation of stored record paths against the files actually
/// collected during a full-index pass.
///
/// A stored path is returned for deletion when it is **either**:
/// * physically absent under `workspace_root` (a genuine on-disk deletion), or
/// * not present in `collected` **and** the collection was
///   authoritative-complete (`collected.complete == true`) — the alias-stale
///   case where a directory alias supersedes the stored path so only a sibling
///   path is collected for the same real file.
///
/// Fail-closed guarantees:
/// * INV-3: if `workspace_root` cannot be canonicalized, returns an EMPTY set
///   (never the fail-open mass-delete of the retired `compute_deleted_paths`).
/// * INV-2: on a non-authoritative pass (`complete == false`) only
///   physically-absent paths are deleted; alias-stale records are retained.
/// * The workspace-relative escape guard is preserved — a stored path that
///   escapes the workspace root is logged and never deleted.
#[must_use]
pub(crate) fn reconcile_deleted_paths(
    stored_rel_paths: &[String],
    collected: &CollectedFiles,
    workspace_root: &Path,
) -> Vec<String> {
    // INV-3: a failure to establish the workspace authority bound yields an
    // EMPTY deletion set — never the fail-open mass-delete of the retired
    // per-collector `compute_deleted_paths`.
    let Ok(canonical_root) = workspace_root.canonicalize() else {
        return Vec::new();
    };

    let collected_rel: HashSet<String> = collected
        .files
        .iter()
        .filter_map(|absolute| normalize_collected_rel_path(absolute, workspace_root))
        .collect();

    stored_rel_paths
        .iter()
        .filter_map(|stored| {
            let Some(relative_path) = workspace_relative_path(stored) else {
                warn!(
                    path = %stored,
                    "skipping deletion sweep path that escapes the workspace root"
                );
                return None;
            };
            let candidate = workspace_root.join(relative_path);
            let physically_absent = !is_regular_file_in_workspace(&candidate, &canonical_root);
            // INV-1/INV-2: an alias-stale record (still physically present) is
            // reconciled away only when the collection was
            // authoritative-complete; a partial pass retains it (fail-closed).
            let not_collected = !collected_rel.contains(stored);
            let stale = physically_absent || (collected.complete && not_collected);
            stale.then(|| stored.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::{CollectedFiles, reconcile_deleted_paths};

    fn stored(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|p| (*p).to_owned()).collect()
    }

    /// INV-1 (alias-stale): the stored path is still physically present (its real
    /// file exists via the aliased directory) but a *different* collected path
    /// supersedes it; on an authoritative-complete pass the stale record is
    /// deleted. (RF-1/RF-2 reconciler core.)
    #[test]
    fn alias_superseded_stored_path_is_deleted_when_complete() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let real_dir = workspace.path().join("z");
        fs::create_dir_all(&real_dir).expect("create real dir");
        fs::write(real_dir.join("shared.ipynb"), "{}").expect("write real notebook");

        // Traversal collected the aliased path `a/shared.ipynb`; the stored
        // record still points at `z/shared.ipynb` (physically present).
        let collected = CollectedFiles {
            files: vec![workspace.path().join("a").join("shared.ipynb")],
            complete: true,
        };

        let deleted =
            reconcile_deleted_paths(&stored(&["z/shared.ipynb"]), &collected, workspace.path());

        assert_eq!(
            deleted,
            vec!["z/shared.ipynb".to_owned()],
            "alias-superseded stored path must be reconciled away on a complete pass"
        );
    }

    /// A stored path that was collected this pass and is physically present is
    /// retained (never a false delete).
    #[test]
    fn live_collected_path_is_retained() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let real_dir = workspace.path().join("z");
        fs::create_dir_all(&real_dir).expect("create real dir");
        fs::write(real_dir.join("shared.ipynb"), "{}").expect("write real notebook");

        let collected = CollectedFiles {
            files: vec![workspace.path().join("z").join("shared.ipynb")],
            complete: true,
        };

        let deleted =
            reconcile_deleted_paths(&stored(&["z/shared.ipynb"]), &collected, workspace.path());

        assert!(
            deleted.is_empty(),
            "a live, collected record must never be deleted; got {deleted:?}"
        );
    }

    /// INV-2 (non-authoritative pass): when the collection is incomplete, only a
    /// physically-absent record is deleted; the alias-stale (but present)
    /// record is retained. (RF-3.)
    #[test]
    fn incomplete_pass_retains_alias_stale_and_deletes_only_absent() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let real_dir = workspace.path().join("z");
        fs::create_dir_all(&real_dir).expect("create real dir");
        fs::write(real_dir.join("shared.ipynb"), "{}").expect("write real notebook");

        // A subtree read failed → complete == false.
        let collected = CollectedFiles {
            files: vec![workspace.path().join("a").join("shared.ipynb")],
            complete: false,
        };

        let deleted = reconcile_deleted_paths(
            &stored(&["z/shared.ipynb", "gone/deleted.ipynb"]),
            &collected,
            workspace.path(),
        );

        assert_eq!(
            deleted,
            vec!["gone/deleted.ipynb".to_owned()],
            "non-authoritative pass must delete only physically-absent paths; got {deleted:?}"
        );
    }

    /// RF-5: a genuinely deleted file is removed via the physical-absence path
    /// regardless of completeness.
    #[test]
    fn genuinely_absent_path_is_deleted() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let collected = CollectedFiles {
            files: Vec::new(),
            complete: true,
        };

        let deleted =
            reconcile_deleted_paths(&stored(&["models/gone.tmdl"]), &collected, workspace.path());

        assert_eq!(deleted, vec!["models/gone.tmdl".to_owned()]);
    }

    /// INV-3 / RF-4: when the workspace root cannot be canonicalized the
    /// reconciler returns an EMPTY deletion set — never a fail-open mass delete.
    #[test]
    fn canonicalize_failure_yields_empty_deletion_set() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let missing_root = workspace.path().join("does-not-exist-root");

        let collected = CollectedFiles {
            files: Vec::new(),
            complete: true,
        };

        let deleted =
            reconcile_deleted_paths(&stored(&["z/shared.ipynb"]), &collected, &missing_root);

        assert!(
            deleted.is_empty(),
            "canonicalize failure must fail closed (empty set); got {deleted:?}"
        );
    }

    /// The completeness collector reports `complete == true` on a fully readable
    /// tree and the thin wrapper still yields the file list.
    #[test]
    fn checked_collector_reports_complete_on_readable_tree() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let dir = workspace.path().join("nb");
        fs::create_dir_all(dir.join("sub")).expect("create nested dir");
        fs::write(dir.join("a.ipynb"), "{}").expect("write a");
        fs::write(dir.join("sub").join("b.ipynb"), "{}").expect("write b");

        fn is_ipynb(path: &std::path::Path) -> bool {
            path.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("ipynb"))
        }

        let collected = super::collect_files_in_workspace_checked(&dir, workspace.path(), is_ipynb);
        assert!(collected.complete, "a fully readable tree must be complete");
        let names: Vec<PathBuf> = collected
            .files
            .iter()
            .map(|p| p.file_name().map(PathBuf::from).unwrap_or_default())
            .collect();
        assert!(names.contains(&PathBuf::from("a.ipynb")));
        assert!(names.contains(&PathBuf::from("b.ipynb")));

        let via_wrapper = super::collect_files_in_workspace(&dir, workspace.path(), is_ipynb);
        assert_eq!(via_wrapper.len(), collected.files.len());
    }
}
