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
//! # 062.001-T scope
//!
//! Task 062.001-T introduces only the dispatch boundary: the registry recognizes
//! `pbip` as a built-in content type and [`crate::services::ingestion`] routes
//! `pbip` sources to [`index_pbip_source`]. The real walk, extraction, and
//! persistence behavior arrives in 062.002-T through 062.007-T.

use std::path::Path;

use tracing::debug;

use crate::db::queries::CodeGraphQueries;
use crate::errors::EngramError;
use crate::models::pbip::PbipIndexResult;
use crate::models::registry::ContentSource;

/// Index all PBIP project-definition files from a single content source.
///
/// # Current behavior (062.001-T scope)
///
/// Returns an empty [`PbipIndexResult`] without touching the database. Verifies
/// that the source directory exists and emits a debug log line for traceability
/// so dispatch wiring is observable in unit and integration runs.
///
/// Real PBIP file collection, extraction, content-record emission, and graph
/// persistence land in 062.002-T through 062.007-T.
///
/// # Errors
///
/// Currently infallible. The signature mirrors the existing dedicated indexers
/// ([`crate::services::powerbi_indexer::index_powerbi_source`] and
/// [`crate::services::notebook_indexer::index_notebook_source`]) so the dispatch
/// branch in [`crate::services::ingestion`] can call this function symmetrically
/// once real indexing is wired up.
pub async fn index_pbip_source(
    source: &ContentSource,
    workspace_root: &Path,
    _queries: &CodeGraphQueries,
    _max_file_size: u64,
) -> Result<PbipIndexResult, EngramError> {
    let source_dir = workspace_root.join(&source.path);
    if !source_dir.is_dir() {
        debug!(
            path = %source.path,
            "PBIP source directory does not exist — skipping (062.001-T stub)"
        );
        return Ok(PbipIndexResult::default());
    }

    debug!(
        path = %source.path,
        "PBIP dispatch reached for source — collection and extraction land in 062.002-T+"
    );

    Ok(PbipIndexResult::default())
}

/// Sweep deleted PBIP project-definition files from the index.
///
/// # Current behavior (062.001-T scope)
///
/// Returns `0` without touching the database. The deletion sweep gains real
/// behavior in 062.002-T (Emit PBIP content records and graph edges) once the
/// indexer is persisting records.
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
