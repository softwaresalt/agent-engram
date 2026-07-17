//! Notebook content indexer for `.ipynb` sources.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use tracing::{debug, info, warn};

use crate::db::queries::CodeGraphQueries;
use crate::errors::EngramError;
use crate::models::content::ContentRecord;
use crate::models::notebook::NotebookIndexResult;
use crate::models::registry::ContentSource;
use crate::services::ingestion::{compute_hash, content_record_identity_seed};
use crate::services::notebook_extract::extract_notebook;
use crate::services::source_traversal::{collect_files_in_workspace, is_regular_file_in_workspace};

/// Collect all notebook files under `dir` recursively.
#[must_use]
pub fn collect_notebook_files(dir: &Path) -> Vec<PathBuf> {
    collect_notebook_files_in_workspace(dir, dir)
}

/// Collect notebook files under `dir`, traversing only symlinked directories
/// whose canonical target remains under `workspace_root`.
#[must_use]
pub fn collect_notebook_files_in_workspace(dir: &Path, workspace_root: &Path) -> Vec<PathBuf> {
    collect_files_in_workspace(dir, workspace_root, is_notebook_file)
}

fn is_notebook_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ipynb"))
}

fn workspace_relative_path(rel_path: &str) -> Option<PathBuf> {
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

fn compute_deleted_paths(
    workspace_relative_paths: &[String],
    workspace_root: &Path,
) -> Vec<String> {
    let Ok(canonical_root) = workspace_root.canonicalize() else {
        return workspace_relative_paths.to_vec();
    };

    workspace_relative_paths
        .iter()
        .filter_map(|rel| {
            let Some(relative_path) = workspace_relative_path(rel) else {
                warn!(
                    path = %rel,
                    "skipping notebook deletion sweep path that escapes the workspace root"
                );
                return None;
            };

            let candidate = workspace_root.join(relative_path);
            let is_deleted = !is_regular_file_in_workspace(&candidate, &canonical_root);
            is_deleted.then(|| rel.clone())
        })
        .collect()
}

fn summary_record_content(
    rel_path: &str,
    extracted: &crate::models::notebook::ExtractedNotebook,
) -> String {
    let title = extracted
        .summary
        .title
        .as_deref()
        .unwrap_or("Untitled notebook");

    format!(
        "Notebook: {title}. Path: {rel_path}. Default language: {}. Total cells: {}. Indexed cells: {}. Markdown cells: {}. Code cells: {}.",
        extracted.summary.default_language,
        extracted.summary.total_cells,
        extracted.summary.indexed_cell_count,
        extracted.summary.markdown_cells,
        extracted.summary.code_cells
    )
}

/// Index all notebook files from a single content source.
pub async fn index_notebook_source(
    source: &ContentSource,
    workspace_root: &Path,
    queries: &CodeGraphQueries,
    max_file_size: u64,
) -> Result<NotebookIndexResult, EngramError> {
    let mut result = NotebookIndexResult::default();

    let source_dir = workspace_root.join(&source.path);
    if !source_dir.exists() {
        debug!(
            path = %source.path,
            "Notebook source directory does not exist — skipping"
        );
        return Ok(result);
    }

    let files = collect_notebook_files_in_workspace(&source_dir, workspace_root);
    result.total_files = files.len();

    let existing_hashes: HashMap<String, String> = queries
        .select_content_records(Some("notebook"))
        .await?
        .into_iter()
        .filter(|record| record.source_path == source.path)
        .map(|record| (record.file_path, record.content_hash))
        .collect();

    for file_path in &files {
        let Ok(metadata) = file_path.metadata() else {
            continue;
        };

        if metadata.len() > max_file_size {
            debug!(
                path = %file_path.display(),
                "Notebook file exceeds max_file_size — skipping"
            );
            continue;
        }

        let Ok(content_bytes) = std::fs::read(file_path) else {
            continue;
        };
        let Ok(content_text) = std::str::from_utf8(&content_bytes) else {
            continue;
        };

        let rel_path = file_path
            .strip_prefix(workspace_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");
        let content_hash = compute_hash(content_bytes.as_slice());

        if existing_hashes.get(&rel_path).map(String::as_str) == Some(content_hash.as_str()) {
            result.unchanged += 1;
            continue;
        }

        let Some(extracted) = extract_notebook(content_text, &rel_path) else {
            debug!(path = %rel_path, "malformed notebook JSON — skipping file");
            continue;
        };

        queries
            .delete_content_records_by_scope(&rel_path, "notebook", &source.path)
            .await?;

        let file_size = metadata.len();
        let now = Utc::now();

        let summary_seed = content_record_identity_seed(&rel_path, "notebook", &source.path, None);
        let summary_record = ContentRecord {
            id: format!("cr_{}", compute_hash(summary_seed.as_bytes())),
            content_type: "notebook".to_string(),
            file_path: rel_path.clone(),
            content_hash: content_hash.clone(),
            content: summary_record_content(&rel_path, &extracted),
            embedding: None,
            source_path: source.path.clone(),
            file_size_bytes: file_size,
            ingested_at: now,
            record_kind: "notebook_summary".to_string(),
            chunk_id: None,
            chunk_index: None,
            heading_path: Vec::new(),
            line_start: None,
            line_end: None,
            fallback_reason: None,
            lint_summary: None,
            suggestions: Vec::new(),
        };
        queries.upsert_content_record(&summary_record).await?;

        for cell in &extracted.cells {
            let identity_seed = content_record_identity_seed(
                &rel_path,
                "notebook",
                &source.path,
                Some(cell.chunk_id.as_str()),
            );
            let record = ContentRecord {
                id: format!("cr_{}", compute_hash(identity_seed.as_bytes())),
                content_type: "notebook".to_string(),
                file_path: rel_path.clone(),
                content_hash: content_hash.clone(),
                content: cell.content.clone(),
                embedding: None,
                source_path: source.path.clone(),
                file_size_bytes: file_size,
                ingested_at: now,
                record_kind: cell.record_kind.clone(),
                chunk_id: Some(cell.chunk_id.clone()),
                chunk_index: Some(cell.chunk_index),
                heading_path: Vec::new(),
                line_start: None,
                line_end: None,
                fallback_reason: None,
                lint_summary: None,
                suggestions: Vec::new(),
            };
            queries.upsert_content_record(&record).await?;
        }

        result.ingested += 1;
    }

    info!(
        ingested = result.ingested,
        unchanged = result.unchanged,
        total = result.total_files,
        source = %source.path,
        "Notebook indexing complete"
    );

    Ok(result)
}

/// Remove content records for notebook files that no longer exist on disk.
pub async fn sweep_deleted_notebook_files(
    source: &ContentSource,
    workspace_root: &Path,
    queries: &CodeGraphQueries,
) -> Result<usize, EngramError> {
    let records = queries.select_content_records(Some("notebook")).await?;
    let known_paths: Vec<String> = records
        .into_iter()
        .filter(|record| record.source_path == source.path)
        .map(|record| record.file_path)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let deleted = compute_deleted_paths(&known_paths, workspace_root);
    let mut removed = 0_usize;

    for path in &deleted {
        queries
            .delete_content_records_by_scope(path, "notebook", &source.path)
            .await?;
        removed += 1;
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::collect_notebook_files_in_workspace;
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    #[cfg(unix)]
    fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(src, dst)
    }

    #[cfg(windows)]
    fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(src, dst)
    }

    #[cfg(unix)]
    fn symlink_file(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(src, dst)
    }

    #[cfg(windows)]
    fn symlink_file(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_file(src, dst)
    }

    fn create_symlink_dir(src: &Path, dst: &Path) -> bool {
        match symlink_dir(src, dst) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("create directory symlink: {error}"),
        }
    }

    fn create_symlink_file(src: &Path, dst: &Path) -> bool {
        match symlink_file(src, dst) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("create file symlink: {error}"),
        }
    }

    #[test]
    fn collect_notebook_files_handles_symlink_cycles_and_workspace_bounds() {
        let workspace = TempDir::new().expect("tempdir");
        let external = TempDir::new().expect("tempdir");

        let notebooks_dir = workspace.path().join("notebooks");
        let nested_dir = notebooks_dir.join("nested");
        fs::create_dir_all(&nested_dir).expect("create nested notebook dir");
        fs::write(notebooks_dir.join("real.ipynb"), "{}").expect("write real notebook");
        fs::write(nested_dir.join("inner.ipynb"), "{}").expect("write nested notebook");

        let shared_dir = workspace.path().join("shared");
        fs::create_dir_all(&shared_dir).expect("create shared notebook dir");
        fs::write(shared_dir.join("shared.ipynb"), "{}").expect("write shared notebook");

        let escape_dir = external.path().join("escape");
        fs::create_dir_all(&escape_dir).expect("create external notebook dir");
        let external_file = escape_dir.join("outside.ipynb");
        fs::write(&external_file, "{}").expect("write external notebook");

        let linked_dir_created = create_symlink_dir(&escape_dir, &notebooks_dir.join("linked-dir"));
        let linked_file_created =
            create_symlink_file(&external_file, &notebooks_dir.join("linked-file.ipynb"));
        let linked_shared_created =
            create_symlink_dir(&shared_dir, &notebooks_dir.join("linked-shared"));
        if linked_shared_created {
            let _ = create_symlink_dir(&notebooks_dir, &shared_dir.join("cycle"));
        }

        if !linked_dir_created && !linked_file_created && !linked_shared_created {
            return;
        }

        let files = collect_notebook_files_in_workspace(&notebooks_dir, workspace.path());
        let rel_paths: Vec<String> = files
            .iter()
            .map(|path| {
                path.strip_prefix(workspace.path())
                    .expect("path under workspace")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();

        assert!(rel_paths.iter().any(|p| p == "notebooks/real.ipynb"));
        assert!(
            rel_paths
                .iter()
                .any(|p| p == "notebooks/nested/inner.ipynb")
        );
        assert!(
            !rel_paths.iter().any(|p| p.contains("outside.ipynb")),
            "workspace-escaping symlink targets must be skipped; got {rel_paths:?}"
        );
        if linked_shared_created {
            assert!(
                rel_paths
                    .iter()
                    .any(|p| p == "notebooks/linked-shared/shared.ipynb"),
                "in-workspace symlinked directories should be collected; got {rel_paths:?}"
            );
            assert_eq!(
                rel_paths
                    .iter()
                    .filter(|p| p.ends_with("shared.ipynb"))
                    .count(),
                1,
                "symlink cycles should not collect duplicate real files; got {rel_paths:?}"
            );
        }
    }

    #[test]
    fn compute_deleted_paths_reports_file_symlink_candidates_as_deleted() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let regular_path = workspace.path().join("regular.ipynb");
        let symlink_target = workspace.path().join("target.ipynb");
        let symlink_path = workspace.path().join("indexed.ipynb");
        fs::write(&regular_path, "{}").expect("write regular notebook");
        fs::write(&symlink_target, "{}").expect("write target notebook");
        if !create_symlink_file(&symlink_target, &symlink_path) {
            return;
        }

        let external = TempDir::new().expect("external tempdir");
        let external_dir = external.path().join("escape");
        fs::create_dir_all(&external_dir).expect("create external dir");
        fs::write(external_dir.join("outside.ipynb"), "{}").expect("write external notebook");
        if !create_symlink_dir(&external_dir, &workspace.path().join("linked-outside")) {
            return;
        }

        let deleted = super::compute_deleted_paths(
            &[
                "regular.ipynb".to_string(),
                "indexed.ipynb".to_string(),
                "linked-outside/outside.ipynb".to_string(),
                "absent.ipynb".to_string(),
            ],
            workspace.path(),
        );

        assert_eq!(
            deleted,
            vec![
                "indexed.ipynb".to_string(),
                "linked-outside/outside.ipynb".to_string(),
                "absent.ipynb".to_string(),
            ]
        );
    }
}
