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

/// The page order and active page parsed from a `pages.json` descriptor.
///
/// `pages.json` declares the report's page order and which page is active.
/// Populated by [`crate::services::pbip_extract::parse_page_order`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PbipPageOrder {
    /// Logical page names in display order (`pageOrder`).
    pub order: Vec<String>,

    /// The active page name (`activePageName`), if declared.
    pub active_page: Option<String>,
}

/// A single report page parsed from a `page.json` descriptor.
///
/// Carries the page identity (logical name and display name). Page ordering
/// comes from [`PbipPageOrder`], not from the individual `page.json`. Populated
/// by [`crate::services::pbip_extract::parse_page`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PbipPage {
    /// Stable synthetic ID derived from the workspace-relative `page.json` path.
    pub id: String,

    /// Logical page name (`name`).
    pub name: String,

    /// Human-readable page title (`displayName`), falling back to `name`.
    pub display_name: String,

    /// Workspace-relative path to the `page.json` descriptor.
    pub path: String,
}

/// The kind of semantic field a visual binds to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PbipBindingKind {
    /// The field binds to a DAX measure.
    Measure,
    /// The field binds to a table column.
    Column,
}

/// A semantic binding hint extracted from a visual's query projections.
///
/// Records which model entity (table) and property (measure or column) a
/// visual references, so downstream graph indexing can emit visual→measure or
/// visual→table edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PbipBinding {
    /// Whether the bound property is a measure or a column.
    pub kind: PbipBindingKind,

    /// The source entity (table) name from `Expression.SourceRef.Entity`.
    pub entity: String,

    /// The bound property name from `Property`.
    pub property: String,
}

/// A visual element parsed from a `visual.json` descriptor.
///
/// Carries the visual identity, type, and semantic binding hints. Populated by
/// [`crate::services::pbip_extract::parse_visual`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PbipVisual {
    /// Stable synthetic ID derived from the workspace-relative `visual.json` path.
    pub id: String,

    /// Visual name (`name`).
    pub name: String,

    /// Visual type string (`visual.visualType`), e.g. `"card"`, `"barChart"`.
    pub visual_type: String,

    /// Workspace-relative path to the `visual.json` descriptor.
    pub path: String,

    /// Semantic binding hints extracted from the visual's query projections.
    #[serde(default)]
    pub bindings: Vec<PbipBinding>,
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
