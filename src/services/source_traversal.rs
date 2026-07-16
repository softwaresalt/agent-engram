//! Shared source-tree traversal helpers.

use std::collections::HashSet;
use std::fs::FileType;
use std::path::{Path, PathBuf};

/// Collect files under `dir` whose paths satisfy `is_target_file`.
///
/// Directory symlinks are traversed only when their canonical target remains
/// under `workspace_root`; canonical directory visits are tracked so cycles and
/// aliases cannot recurse indefinitely or duplicate the same real tree.
#[must_use]
pub(crate) fn collect_files_in_workspace(
    dir: &Path,
    workspace_root: &Path,
    is_target_file: fn(&Path) -> bool,
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(canonical_root) = workspace_root.canonicalize() else {
        return files;
    };
    let mut visited = HashSet::new();
    collect_recursive(
        dir,
        &canonical_root,
        &mut visited,
        &mut files,
        is_target_file,
    );
    files.sort();
    files.dedup();
    files
}

fn collect_recursive(
    dir: &Path,
    canonical_root: &Path,
    visited: &mut HashSet<PathBuf>,
    files: &mut Vec<PathBuf>,
    is_target_file: fn(&Path) -> bool,
) {
    let Ok(canonical_dir) = dir.canonicalize() else {
        return;
    };
    if !canonical_dir.starts_with(canonical_root) || !visited.insert(canonical_dir) {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
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
            collect_recursive(&path, canonical_root, visited, files, is_target_file);
        } else if file_type.is_file() && is_target_file(&path) {
            files.push(path);
        } else if file_type.is_symlink() {
            collect_symlinked_directory(&path, canonical_root, visited, files, is_target_file);
        }
    }
}

fn collect_symlinked_directory(
    path: &Path,
    canonical_root: &Path,
    visited: &mut HashSet<PathBuf>,
    files: &mut Vec<PathBuf>,
    is_target_file: fn(&Path) -> bool,
) {
    let Ok(canonical_target) = path.canonicalize() else {
        return;
    };
    if !canonical_target.starts_with(canonical_root) || !canonical_target.is_dir() {
        return;
    }
    collect_recursive(path, canonical_root, visited, files, is_target_file);
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
