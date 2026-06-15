//! PBIP content indexer for the Power BI project-definition layout.
//!
//! Dedicated source boundary for `.pbip` workspaces that use the newer
//! project-definition contract (split `.pbip` / `.pbir` / `.pbism` descriptors,
//! per-report/page/visual JSON under `definition/`, and `definition/**/*.tmdl`).
//!
//! This module is intentionally kept distinct from [`crate::services::powerbi_indexer`]
//! so the legacy JSON/BIM-backed `powerbi` source path remains stable while
//! the dedicated `pbip` path is built out across 062-F.
//!
//! # Build-out status
//!
//! * 062.001-T introduced the dispatch boundary so [`crate::services::ingestion`]
//!   routes `pbip` sources here.
//! * 062.003-T added the pure extractors in
//!   [`crate::services::pbip_extract`] and
//!   [`crate::services::pbip_tmdl`].
//! * **062.004-T (this task)** introduces the [`collect_pbip_files`] walker and
//!   the [`compute_deleted_paths`] deletion-sweep helper. Content-record and
//!   graph-edge persistence still belongs to 062.002-T; this module's
//!   [`index_pbip_source`] currently uses the walker only to populate
//!   `total_files` on the returned [`PbipIndexResult`].

use std::fs::FileType;
use std::path::{Component, Path, PathBuf};

use tracing::{debug, warn};

use crate::db::queries::CodeGraphQueries;
use crate::errors::EngramError;
use crate::models::pbip::PbipIndexResult;
use crate::models::registry::ContentSource;

/// File extensions that belong to the PBIP project-definition layout.
///
/// Mirrors the spike's enumeration: workspace entry (`.pbip`), per-artifact
/// descriptors (`.pbir`, `.pbism`), project-definition JSON under
/// `<Artifact>/definition/`, and TMDL semantic-model files under
/// `<SemanticModel>/definition/`. Matching is case-insensitive.
const PBIP_EXTENSIONS: &[&str] = &["pbip", "pbir", "pbism", "json", "tmdl"];

// ── File collection ───────────────────────────────────────────────────────

/// Collect all PBIP project-definition files under `dir` recursively.
///
/// Returns a sorted list of absolute paths to files whose extension matches
/// the PBIP project-definition layout: `.pbip`, `.pbir`, `.pbism`, `.json`,
/// or `.tmdl`. Files with other extensions are ignored even when they sit
/// alongside PBIP files.
///
/// Symbolic links — whether to files or directories — are skipped without
/// following so a PBIP source cannot escape its workspace root or recurse
/// into an alias loop. This matches the
/// [`crate::services::notebook_indexer::collect_notebook_files`] containment
/// contract.
///
/// Returns an empty list if `dir` does not exist, resolves through a
/// symlink, or cannot be read. A `warn!` log entry is emitted in the
/// `read_dir` failure case so that field traces can distinguish "no files"
/// from "could not read directory".
#[must_use]
pub fn collect_pbip_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if physical_file_type(dir).is_some_and(|file_type| file_type.is_dir()) {
        collect_recursive(dir, &mut files);
    }
    files.sort();
    files
}

fn physical_file_type(path: &Path) -> Option<FileType> {
    std::fs::symlink_metadata(path)
        .ok()
        .map(|metadata| metadata.file_type())
}

fn collect_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            warn!(
                path = %dir.display(),
                error = %err,
                "skipping PBIP directory that could not be read"
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_type) = physical_file_type(&path) else {
            continue;
        };

        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            collect_recursive(&path, files);
        } else if file_type.is_file() && is_pbip_file(&path) {
            files.push(path);
        }
    }
}

fn is_pbip_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            PBIP_EXTENSIONS
                .iter()
                .any(|target| extension.eq_ignore_ascii_case(target))
        })
}

// ── Deletion sweep ────────────────────────────────────────────────────────

/// Return the subset of `workspace_relative_paths` whose files no longer
/// exist under `workspace_root`.
///
/// Each path is joined to `workspace_root` before the existence check so
/// that workspace-relative record paths (as stored in
/// `ContentRecord.file_path`) are handled correctly. Paths that attempt to
/// escape the workspace (absolute paths, `..` segments, Windows drive
/// prefixes) are rejected with a `warn!` log entry and excluded from the
/// returned set so a poisoned record cannot probe outside the workspace.
#[must_use]
pub fn compute_deleted_paths(
    workspace_relative_paths: &[String],
    workspace_root: &Path,
) -> Vec<String> {
    workspace_relative_paths
        .iter()
        .filter_map(|rel| {
            let Some(relative_path) = workspace_relative_path(rel) else {
                warn!(
                    path = %rel,
                    "skipping PBIP deletion sweep path that escapes the workspace root"
                );
                return None;
            };
            (!workspace_root.join(relative_path).exists()).then(|| rel.clone())
        })
        .collect()
}

fn workspace_relative_path(rel_path: &str) -> Option<PathBuf> {
    let path = Path::new(rel_path);
    // `has_root()` catches both Windows-style absolute paths (`C:\...`) and
    // forward-slash absolute paths (`/etc/passwd`) on Windows, where
    // `is_absolute()` requires a drive prefix and would otherwise let
    // `/etc/passwd` slip through.
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

// ── Async indexer ─────────────────────────────────────────────────────────

/// Index all PBIP project-definition files from a single content source.
///
/// # Current behavior (062.004-T scope)
///
/// Walks the source directory with [`collect_pbip_files`] and populates
/// `total_files` on the returned [`PbipIndexResult`]. Files are not yet
/// hashed or persisted; content-record and graph-edge persistence is the
/// job of 062.002-T.
///
/// # Errors
///
/// Currently infallible. The signature mirrors the existing dedicated indexers
/// ([`crate::services::powerbi_indexer::index_powerbi_source`] and
/// [`crate::services::notebook_indexer::index_notebook_source`]) so the
/// dispatch branch in [`crate::services::ingestion`] can call this function
/// symmetrically once real indexing is wired up.
pub async fn index_pbip_source(
    source: &ContentSource,
    workspace_root: &Path,
    _queries: &CodeGraphQueries,
    _max_file_size: u64,
) -> Result<PbipIndexResult, EngramError> {
    let mut result = PbipIndexResult::default();

    let source_dir = workspace_root.join(&source.path);
    if !physical_file_type(&source_dir).is_some_and(|file_type| file_type.is_dir()) {
        debug!(
            path = %source.path,
            "PBIP source directory does not exist or resolves through a symlink — skipping"
        );
        return Ok(result);
    }

    let files = collect_pbip_files(&source_dir);
    result.total_files = files.len();

    debug!(
        path = %source.path,
        total_files = result.total_files,
        "PBIP dispatch reached for source — persistence lands in 062.002-T"
    );

    Ok(result)
}

/// Sweep deleted PBIP project-definition files from the index.
///
/// # Current behavior (062.004-T scope)
///
/// Returns `0` without touching the database. The deletion sweep gains real
/// behavior in 062.002-T (Emit PBIP content records and graph edges) once
/// the indexer is persisting records. The supporting
/// [`compute_deleted_paths`] helper is exposed now so 062.002-T can wire it
/// to the persistence layer without reshaping the public surface.
///
/// # Errors
///
/// Currently infallible. Matches the signature of
/// [`crate::services::powerbi_indexer::sweep_deleted_powerbi_files`] for
/// consistent dispatch wiring.
pub async fn sweep_deleted_pbip_files(
    _source: &ContentSource,
    _workspace_root: &Path,
    _queries: &CodeGraphQueries,
) -> Result<usize, EngramError> {
    Ok(0)
}
