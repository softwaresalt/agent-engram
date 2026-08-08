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
    let Ok(canonical_dir) = dir.canonicalize() else {
        return CollectedFiles {
            files,
            complete: false,
        };
    };
    if !canonical_dir.starts_with(&canonical_root) {
        // The requested traversal root is outside the workspace authority
        // bound. Reject it before recursion so an out-of-workspace tree can
        // never certify an authoritative empty pass.
        return CollectedFiles {
            files,
            complete: false,
        };
    }
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
    let mut collected_entries: Vec<(u8, PathBuf, FileType)> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warn!(
                    dir = %dir.display(),
                    %error,
                    "skipping unreadable directory entry during source traversal (fail-closed)"
                );
                // A dropped entry may hide a still-present indexed file, so the
                // pass is no longer authoritative (fail-closed; INV-2). Never
                // silently omit an entry while leaving `complete == true`, or a
                // live record could be reconciled away as alias-stale.
                *complete = false;
                continue;
            }
        };
        let path = entry.path();
        let file_type = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata.file_type(),
            Err(error) => {
                warn!(
                    path = %path.display(),
                    %error,
                    "skipping traversal entry whose metadata could not be read (fail-closed)"
                );
                // Same fail-closed rule: an unreadable entry degrades the pass.
                *complete = false;
                continue;
            }
        };
        collected_entries.push((entry_rank(&file_type), path, file_type));
    }
    collected_entries
        .sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    for (_, path, file_type) in collected_entries {
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
    let canonical_target = match path.canonicalize() {
        Ok(target) => target,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // A genuinely broken symlink resolves to nothing — it is not a
            // missed readable subtree, so completeness is preserved.
            return;
        }
        Err(error) => {
            // 100-S Copilot review: a transient error (PermissionDenied, I/O)
            // may hide an in-workspace subtree whose files are still indexed.
            // Treating it as "broken" would leave `complete == true` and let
            // the reconciler delete those records as not-collected, so the pass
            // is no longer authoritative (fail-closed; INV-2).
            warn!(
                path = %path.display(),
                %error,
                "skipping symlinked directory whose target could not be resolved (fail-closed)"
            );
            *complete = false;
            return;
        }
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

/// Fail-closed physical-absence oracle for the shared deletion reconciler.
///
/// Returns `true` only when the stored path is *provably* gone: a `NotFound`
/// from `symlink_metadata`/`canonicalize`, a path that now resolves to a
/// non-file, or one whose real target escapes the workspace. A transient error
/// (`PermissionDenied`, other I/O) cannot prove absence — an unreadable parent
/// directory still contains a live file — so the record is retained (`false`).
/// The safety floor forbids deleting a live record we merely failed to stat.
///
/// This is intentionally distinct from the shared
/// [`is_regular_file_in_workspace`], which the out-of-scope backlog/pbip
/// `compute_deleted_paths` still call with their historical fail-open on
/// transient errors (tracked as a separate follow-up).
fn is_physically_absent(path: &Path, canonical_root: &Path) -> bool {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
        Err(error) => {
            warn!(
                path = %path.display(),
                %error,
                "retaining stored record: cannot stat path to prove absence (fail-closed)"
            );
            return false;
        }
    };
    if !metadata.file_type().is_file() {
        // The path exists but is no longer the regular file we stored (e.g. it
        // was replaced by a directory): the original file is genuinely gone.
        return true;
    }
    match path.canonicalize() {
        Ok(canonical) => !canonical.starts_with(canonical_root),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(error) => {
            warn!(
                path = %path.display(),
                %error,
                "retaining stored record: cannot canonicalize present path (fail-closed)"
            );
            false
        }
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
            let physically_absent = is_physically_absent(&candidate, &canonical_root);
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
    use std::path::{Path, PathBuf};

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
        fn is_ipynb(path: &std::path::Path) -> bool {
            path.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("ipynb"))
        }

        let workspace = TempDir::new().expect("workspace tempdir");
        let dir = workspace.path().join("nb");
        fs::create_dir_all(dir.join("sub")).expect("create nested dir");
        fs::write(dir.join("a.ipynb"), "{}").expect("write a");
        fs::write(dir.join("sub").join("b.ipynb"), "{}").expect("write b");

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

    /// 110-S U1: a traversal root outside the workspace cannot certify an
    /// authoritative empty pass. The guard must reject it before recursion so
    /// no out-of-workspace file is collected or treated as deletion evidence.
    #[test]
    fn checked_collector_rejects_out_of_workspace_root_as_non_authoritative() {
        fn is_ipynb(path: &std::path::Path) -> bool {
            path.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("ipynb"))
        }

        let workspace = TempDir::new().expect("workspace tempdir");
        let outside = TempDir::new().expect("outside tempdir");
        fs::write(outside.path().join("live.ipynb"), "{}").expect("write outside notebook");

        let collected =
            super::collect_files_in_workspace_checked(outside.path(), workspace.path(), is_ipynb);

        assert!(
            collected.files.is_empty(),
            "an out-of-workspace traversal root must yield no files"
        );
        assert!(
            !collected.complete,
            "an out-of-workspace traversal root must be non-authoritative"
        );
    }

    /// 100-S review P1-A (fail-closed): a subdirectory whose entries can be
    /// listed (read bit) but not stat'd (no execute/traverse bit) produces
    /// per-entry `symlink_metadata` failures. The collector must flag the pass
    /// non-authoritative (`complete == false`) rather than silently omit the
    /// entries — otherwise the reconciler would treat a still-present record as
    /// alias-stale and delete live content. (Pre-fix `entries.flatten()` +
    /// `.ok()?` dropped such entries while leaving `complete == true`.)
    #[cfg(unix)]
    #[test]
    fn per_entry_metadata_failure_marks_pass_non_authoritative() {
        use std::os::unix::fs::PermissionsExt;

        fn is_ipynb(path: &std::path::Path) -> bool {
            path.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("ipynb"))
        }

        let workspace = TempDir::new().expect("workspace tempdir");
        let dir = workspace.path().join("nb");
        let locked = dir.join("locked");
        fs::create_dir_all(&locked).expect("create locked subdir");
        fs::write(locked.join("a.ipynb"), "{}").expect("write a");
        // Read bit but no execute bit: read_dir(locked) can list names, but
        // symlink_metadata on each entry fails (no traverse permission).
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o444)).expect("chmod locked r--");

        let collected = super::collect_files_in_workspace_checked(&dir, workspace.path(), is_ipynb);

        // Restore perms so TempDir cleanup can remove the tree.
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755))
            .expect("restore locked perms");

        assert!(
            !collected.complete,
            "per-entry metadata (or listing) failures must mark the pass non-authoritative (fail-closed)"
        );
    }

    /// 100-S Copilot review (fail-closed physical-absence): a stored record
    /// whose file is still present but cannot be stat'd — because a parent
    /// directory lost its traverse bit — must be RETAINED, not swept. Pre-fix
    /// `is_regular_file_in_workspace` returned `false` on the `PermissionDenied`
    /// from `symlink_metadata`, so `physically_absent` became `true` and the
    /// live record was deleted unconditionally (bypassing the `complete` gate).
    /// `complete: false` isolates the physical-absence branch.
    #[cfg(unix)]
    #[test]
    fn reconciler_retains_present_record_under_unreadable_parent() {
        use std::os::unix::fs::PermissionsExt;

        let workspace = TempDir::new().expect("workspace tempdir");
        let locked = workspace.path().join("locked");
        fs::create_dir_all(&locked).expect("create locked dir");
        let keep = locked.join("keep.ipynb");
        fs::write(&keep, "{}").expect("write keep");
        // No traverse (execute) bit on the parent: symlink_metadata(keep) and
        // canonicalize(keep) both fail with PermissionDenied (not NotFound).
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("chmod locked 000");

        let collected = CollectedFiles {
            files: Vec::new(),
            complete: false,
        };
        let deleted = reconcile_deleted_paths(
            &stored(&["locked/keep.ipynb"]),
            &collected,
            workspace.path(),
        );

        // Restore perms so TempDir cleanup can remove the tree.
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755))
            .expect("restore locked perms");

        assert!(
            deleted.is_empty(),
            "a present-but-unreadable record must be retained (fail-closed); got {deleted:?}"
        );
    }

    /// 100-S Copilot review (fail-closed symlinked-directory resolution): a
    /// directory symlink whose target cannot be resolved because of a transient
    /// (non-`NotFound`) error must mark the pass non-authoritative rather than
    /// be treated as a harmless broken link. Otherwise its still-indexed files
    /// go uncollected while `complete == true`, and the reconciler deletes them.
    #[cfg(unix)]
    #[test]
    fn symlinked_dir_with_unresolvable_target_marks_pass_non_authoritative() {
        use std::os::unix::fs::PermissionsExt;

        fn is_ipynb(path: &std::path::Path) -> bool {
            path.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("ipynb"))
        }

        let workspace = TempDir::new().expect("workspace tempdir");
        // A real in-workspace subtree, reached only through an unreadable parent.
        let gated = workspace.path().join("gated");
        let target = gated.join("target");
        fs::create_dir_all(&target).expect("create target");
        fs::write(target.join("x.ipynb"), "{}").expect("write x");

        // The scan root holds a directory symlink into the gated subtree.
        let root = workspace.path().join("root");
        fs::create_dir_all(&root).expect("create root");
        std::os::unix::fs::symlink(&target, root.join("link")).expect("symlink");

        // Drop the traverse bit on `gated` so canonicalize(link) fails with
        // PermissionDenied (not NotFound) when resolving through it.
        fs::set_permissions(&gated, fs::Permissions::from_mode(0o000)).expect("chmod gated 000");

        let collected =
            super::collect_files_in_workspace_checked(&root, workspace.path(), is_ipynb);

        // Restore perms so TempDir cleanup can remove the tree.
        fs::set_permissions(&gated, fs::Permissions::from_mode(0o755))
            .expect("restore gated perms");

        assert!(
            !collected.complete,
            "an unresolvable symlinked directory (transient error) must mark the pass non-authoritative (fail-closed)"
        );
    }

    // ── Deletion-sweep safety (migrated from the retired pub `compute_deleted_paths`
    //    integration coverage; now exercised through the shared reconciler). ──

    /// S-PIN-11 (migrated): the reconciler skips stored paths that would escape
    /// the workspace root (absolute or parent-relative) and sweeps only the
    /// safe, physically-absent workspace-relative path. A `complete: false`
    /// collection isolates the physical-absence branch (no alias supersede).
    #[test]
    fn reconciler_ignores_escape_attempts() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let absolute = workspace
            .path()
            .join("outside.json")
            .to_string_lossy()
            .replace('\\', "/");
        let collected = CollectedFiles {
            files: Vec::new(),
            complete: false,
        };

        let deleted = reconcile_deleted_paths(
            &[
                absolute,
                "../outside.json".to_string(),
                "gone.json".to_string(),
            ],
            &collected,
            workspace.path(),
        );

        assert_eq!(
            deleted,
            vec!["gone.json".to_string()],
            "only safe workspace-relative paths participate in deletion sweeps"
        );
    }

    /// S-PIN-23 (migrated): a workspace-relative symlink whose target now
    /// resolves outside the workspace is swept as deleted rather than preserved
    /// by following the link.
    #[test]
    fn reconciler_treats_outside_symlink_target_as_deleted() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let external = TempDir::new().expect("external tempdir");
        let external_file = external.path().join("outside.tmdl");
        fs::write(&external_file, "table Outside").expect("write external tmdl");

        let link_path = workspace.path().join("linked.tmdl");
        if !create_symlink_file(&external_file, &link_path) {
            return;
        }

        let collected = CollectedFiles {
            files: Vec::new(),
            complete: false,
        };
        let deleted =
            reconcile_deleted_paths(&stored(&["linked.tmdl"]), &collected, workspace.path());

        assert_eq!(
            deleted,
            vec!["linked.tmdl".to_string()],
            "symlinks resolving outside the workspace must be swept as deleted"
        );
    }

    /// S-PIN-24 (migrated): final-component file symlinks and paths reached via
    /// an outside-pointing directory symlink are swept as deleted, while a
    /// regular in-workspace file is retained and an absent path is swept. A
    /// `complete: false` collection isolates the physical-absence branch so the
    /// live regular file is retained on presence alone.
    #[test]
    fn reconciler_reports_file_symlink_candidates_as_deleted() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let regular_path = workspace.path().join("regular.tmdl");
        let symlink_target = workspace.path().join("target.tmdl");
        let symlink_path = workspace.path().join("indexed.tmdl");
        fs::write(&regular_path, "table Regular").expect("write regular tmdl");
        fs::write(&symlink_target, "table Target").expect("write target tmdl");
        if !create_symlink_file(&symlink_target, &symlink_path) {
            return;
        }

        let external = TempDir::new().expect("external tempdir");
        let external_dir = external.path().join("escape");
        fs::create_dir_all(&external_dir).expect("create external dir");
        fs::write(external_dir.join("outside.tmdl"), "table Outside").expect("write external tmdl");
        if !create_symlink_dir(&external_dir, &workspace.path().join("linked-outside")) {
            return;
        }

        let collected = CollectedFiles {
            files: Vec::new(),
            complete: false,
        };
        let deleted = reconcile_deleted_paths(
            &stored(&[
                "regular.tmdl",
                "indexed.tmdl",
                "linked-outside/outside.tmdl",
                "absent.tmdl",
            ]),
            &collected,
            workspace.path(),
        );

        assert_eq!(
            deleted,
            vec![
                "indexed.tmdl".to_string(),
                "linked-outside/outside.tmdl".to_string(),
                "absent.tmdl".to_string(),
            ]
        );
    }

    #[cfg(unix)]
    fn symlink_file(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(src, dst)
    }

    #[cfg(windows)]
    fn symlink_file(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_file(src, dst)
    }

    #[cfg(unix)]
    fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(src, dst)
    }

    #[cfg(windows)]
    fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(src, dst)
    }

    fn create_symlink_file(src: &Path, dst: &Path) -> bool {
        symlink_file(src, dst).is_ok()
    }

    fn create_symlink_dir(src: &Path, dst: &Path) -> bool {
        symlink_dir(src, dst).is_ok()
    }
}
