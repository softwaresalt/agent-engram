//! Power BI project (PBIP) ingestion models.
//!
//! Provides the result type emitted by the dedicated `pbip` indexer
//! ([`crate::services::pbip_indexer`]) that handles the newer
//! project-definition layout (`.pbip`, `.pbir`, `.pbism`, split report/page/visual
//! JSON, and `definition/**/*.tmdl`).
//!
//! Kept separate from [`crate::models::powerbi`] so the legacy JSON/BIM-backed
//! `powerbi` source contract and the newer project-definition `pbip` source
//! contract evolve independently.

use serde::{Deserialize, Serialize};

/// A PBIP workspace entry parsed from a `.pbip` file.
///
/// The `.pbip` file is the top-level entry point of a project-definition
/// workspace. It enumerates the report artifacts that make up the project, so
/// it resolves the workspace-to-report linkage. Populated by
/// [`crate::services::pbip_extract::parse_pbip_workspace`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PbipWorkspace {
    /// Stable synthetic ID derived from the workspace-relative `.pbip` path.
    pub id: String,

    /// Workspace-relative path to the `.pbip` entry file.
    pub path: String,

    /// Report folder paths declared under `artifacts[].report.path`, in order.
    pub report_paths: Vec<String>,
}

/// A report-to-semantic-model link parsed from a `.pbir` file.
///
/// The `.pbir` descriptor lives inside a `<Name>.Report/` folder and points at
/// the semantic model the report binds to (`datasetReference.byPath.path`).
/// Populated by [`crate::services::pbip_extract::parse_pbir_link`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PbipReportLink {
    /// Stable synthetic ID derived from the workspace-relative `.pbir` path.
    pub id: String,

    /// Workspace-relative path to the `.pbir` descriptor file.
    pub path: String,

    /// Workspace-relative report folder (the parent directory of the `.pbir`).
    pub report_path: String,

    /// Resolved workspace-relative path to the linked `.SemanticModel` folder.
    ///
    /// `None` when the report binds by connection string (`byConnection`)
    /// rather than by relative path (`byPath`), or when the relative path
    /// escapes the workspace root and cannot be resolved.
    pub semantic_model_path: Option<String>,
}

/// Aggregated result from a PBIP indexer run over one content source.
#[derive(Debug, Default, Clone)]
pub struct PbipIndexResult {
    /// Number of PBIP project-definition files newly ingested or updated due to content change.
    pub ingested: usize,

    /// Number of PBIP project-definition files skipped because content hash was unchanged.
    pub unchanged: usize,

    /// Number of PBIP project-definition files removed from the index during deletion sweep.
    pub removed: usize,

    /// Total PBIP project-definition files scanned in the source path.
    pub total_files: usize,
}
