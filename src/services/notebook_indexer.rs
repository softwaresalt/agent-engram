//! Notebook content indexer for `.ipynb` sources.

use std::collections::{BTreeMap, HashMap};
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use tracing::{debug, info, warn};

use crate::db::queries::CodeGraphQueries;
use crate::errors::EngramError;
use crate::models::content::ContentRecord;
use crate::models::lineage::{
    CURRENT_EXTRACTOR_VERSION, LineageAuthorityContext, LineageEdgeCandidate, LineageEndpoint,
    LineageEvidence,
};
use crate::models::notebook::{NotebookCellRecord, NotebookIndexResult};
use crate::models::registry::ContentSource;
use crate::services::ingestion::{compute_hash, content_record_identity_seed};
use crate::services::notebook_extract::{extract_notebook, route_notebook_lineage};
use crate::services::source_traversal::{collect_files_in_workspace, is_regular_file_in_workspace};

/// Notebook lineage extraction seam (095-F, Unit U1b / AR-04/G6).
///
/// U1b constructs the trusted-authority context from config and threads it to
/// this boundary; the live PySpark + Spark-SQL candidate collection is U4
/// (`095.006-T`). Isolating the seam here lets U1b verify **construction +
/// propagation** without depending on the parsers: a stub asserts it receives
/// the expected context, and the fail-closed contract holds — an **empty**
/// context can never yield an endpoint.
pub trait NotebookLineageExtractor {
    /// Extract directional lineage edge candidates for one notebook's cells,
    /// binding every dataset identity through `authority_ctx` or dropping it
    /// (fail-closed; 013-D, AR-01).
    fn extract(
        &self,
        notebook_path: &str,
        cells: &[NotebookCellRecord],
        authority_ctx: &LineageAuthorityContext,
    ) -> Vec<LineageEdgeCandidate>;
}

/// The production notebook lineage extractor.
///
/// Delegates to [`route_notebook_lineage`](crate::services::notebook_extract::route_notebook_lineage),
/// which routes each cell to the U2b (PySpark) / U3 (Spark-SQL) extractors and
/// binds identities through `authority_ctx` — an empty context resolves nothing,
/// so the flat aggregate stays fail-closed. The `index_notebook_source` write
/// path routes per cell directly so it can carry each cell's `chunk_index` into
/// evidence; this trait remains the flat authority-propagation seam (U1b).
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultNotebookLineageExtractor;

impl NotebookLineageExtractor for DefaultNotebookLineageExtractor {
    fn extract(
        &self,
        _notebook_path: &str,
        cells: &[NotebookCellRecord],
        authority_ctx: &LineageAuthorityContext,
    ) -> Vec<LineageEdgeCandidate> {
        route_notebook_lineage(cells, authority_ctx)
    }
}

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
///
/// `authority_ctx` is the trusted-authority context built from the ingestion
/// registry's `[lineage]` config (U1b); it is threaded to the notebook lineage
/// extraction seam so table/path identities can bind to a trusted metastore or
/// storage authority. An empty context keeps lineage fail-closed.
pub async fn index_notebook_source(
    source: &ContentSource,
    workspace_root: &Path,
    queries: &CodeGraphQueries,
    max_file_size: u64,
    authority_ctx: &LineageAuthorityContext,
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

        // ── 095-F U4 write-path: route cells → candidates, persist lineage ──
        persist_notebook_lineage(
            queries,
            &rel_path,
            &extracted.cells,
            &content_hash,
            authority_ctx,
        )
        .await?;

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

/// Route a notebook's cells to the Spark-lineage extractors and persist the
/// resulting lineage subgraph (095-F, Unit U4 write-path).
///
/// Flattens each per-cell [`LineageEdgeCandidate`] to one directional
/// `lineage_derives_from` edge per `(target, source)` pair, then upserts **only**
/// the `dataset_node`s incident to an emitted edge — node creation is
/// edge-driven, never endpoint-driven, so a standalone read/write with no
/// counterpart edge writes nothing (fail-closed; Review comment D1). One
/// `lineage_edge_evidence` row is written per `(edge, cell)` keyed on
/// `chunk_index`, so the same edge observed in two cells is preserved (E1).
///
/// On re-index the prior lineage scope is deleted first, GC'ing stale
/// per-notebook evidence and now-unevidenced edges while a `dataset_node` still
/// evidenced elsewhere survives (AR-28). As the **final** write — only after
/// every node/edge/evidence upsert has succeeded — `lineage_index_state` is
/// stamped unconditionally for every extracted notebook, including a
/// zero-lineage one, so an unchanged notebook hash-skips next run instead of
/// re-extracting (AR-03); a failure in any earlier write leaves the notebook
/// un-stamped so it re-extracts (partial-graph recovery; cycle-7 I1).
///
/// # Errors
///
/// Returns [`EngramError`] if any lineage scope-delete, node/edge/evidence
/// upsert, or the freshness stamp fails.
async fn persist_notebook_lineage(
    queries: &CodeGraphQueries,
    notebook_path: &str,
    cells: &[NotebookCellRecord],
    content_hash: &str,
    authority_ctx: &LineageAuthorityContext,
) -> Result<(), EngramError> {
    // Scope-replace prior lineage first (a no-op for a brand-new notebook).
    queries.delete_lineage_by_scope(notebook_path).await?;

    let mut nodes: BTreeMap<String, LineageEndpoint> = BTreeMap::new();
    let mut edges: Vec<(String, String)> = Vec::new();
    let mut evidence: Vec<LineageEvidence> = Vec::new();

    // Route per cell so each candidate carries its originating `chunk_index`.
    for cell in cells {
        for candidate in route_notebook_lineage(std::slice::from_ref(cell), authority_ctx) {
            for source in &candidate.sources {
                edges.push((candidate.target.id.clone(), source.id.clone()));
                evidence.push(LineageEvidence {
                    from_id: candidate.target.id.clone(),
                    to_id: source.id.clone(),
                    notebook_path: notebook_path.to_owned(),
                    chunk_index: cell.chunk_index,
                    content_hash: content_hash.to_owned(),
                });
                // Edge-driven node set: only endpoints of an emitted edge.
                nodes.insert(candidate.target.id.clone(), candidate.target.clone());
                nodes.insert(source.id.clone(), source.clone());
            }
        }
    }

    if !edges.is_empty() {
        let node_set: Vec<LineageEndpoint> = nodes.into_values().collect();
        queries.upsert_dataset_nodes(&node_set).await?;
        queries.upsert_lineage_edges(&edges).await?;
        queries.upsert_lineage_edge_evidence(&evidence).await?;
    }

    // Final write (I1): stamp freshness only after every graph write succeeded.
    queries
        .upsert_lineage_index_state(notebook_path, CURRENT_EXTRACTOR_VERSION)
        .await?;

    Ok(())
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

    // ── 095-F U1b: authority-context propagation seam (AR-04/G6) ──────────

    use super::{DefaultNotebookLineageExtractor, NotebookLineageExtractor};
    use crate::models::lineage::LineageAuthorityContext;
    use crate::models::notebook::NotebookCellRecord;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    /// A stub extractor (seam) that records the authority context it is handed,
    /// so the test can assert the LIVE context — not an accidental empty one —
    /// reached the extractor call boundary.
    #[derive(Default)]
    struct SpyExtractor {
        seen_is_empty: RefCell<Vec<bool>>,
        seen_cat_authority: RefCell<Vec<Option<String>>>,
    }

    impl NotebookLineageExtractor for SpyExtractor {
        fn extract(
            &self,
            _notebook_path: &str,
            _cells: &[NotebookCellRecord],
            authority_ctx: &LineageAuthorityContext,
        ) -> Vec<crate::models::lineage::LineageEdgeCandidate> {
            self.seen_is_empty
                .borrow_mut()
                .push(authority_ctx.is_empty());
            self.seen_cat_authority
                .borrow_mut()
                .push(authority_ctx.catalog_authority_id("cat").map(str::to_owned));
            // A fail-closed stub: with an empty context it can never emit an edge.
            Vec::new()
        }
    }

    fn non_empty_ctx() -> LineageAuthorityContext {
        let mut catalogs = BTreeMap::new();
        catalogs.insert("cat".to_owned(), "prod-metastore".to_owned());
        LineageAuthorityContext::new(catalogs, vec!["s3://bucket".to_owned()])
    }

    // U1b: a configured (non-empty) authority context is propagated intact to
    // the extractor seam.
    #[test]
    fn configured_authority_context_reaches_the_extractor_seam() {
        let spy = SpyExtractor::default();
        let cells: Vec<NotebookCellRecord> = Vec::new();
        let out = spy.extract("n.ipynb", &cells, &non_empty_ctx());
        assert!(out.is_empty());
        assert_eq!(spy.seen_is_empty.borrow().as_slice(), &[false]);
        assert_eq!(
            spy.seen_cat_authority.borrow()[0].as_deref(),
            Some("prod-metastore"),
            "the live catalog->authority mapping must reach the seam"
        );
    }

    // U1b fail-closed: with NO authority configured the seam receives an EMPTY
    // context and yields ZERO endpoints.
    #[test]
    fn absent_authority_config_propagates_empty_context_and_yields_nothing() {
        let spy = SpyExtractor::default();
        let cells: Vec<NotebookCellRecord> = Vec::new();
        let out = spy.extract("n.ipynb", &cells, &LineageAuthorityContext::empty());
        assert!(out.is_empty(), "empty context must yield zero endpoints");
        assert_eq!(spy.seen_is_empty.borrow().as_slice(), &[true]);
        assert_eq!(spy.seen_cat_authority.borrow()[0], None);
    }

    // U1b/U4: the production extractor now delegates to the real per-cell
    // router, but an empty cell set (or an empty authority context) still yields
    // zero edges — the seam stays fail-closed.
    #[test]
    fn default_extractor_yields_nothing_for_empty_cells() {
        let cells: Vec<NotebookCellRecord> = Vec::new();
        assert!(
            DefaultNotebookLineageExtractor
                .extract("n.ipynb", &cells, &non_empty_ctx())
                .is_empty()
        );
    }
}
