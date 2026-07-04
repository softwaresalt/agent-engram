//! Power BI graph data models.
//!
//! Provides [`PowerBiNode`], [`PowerBiEdge`], [`PowerBiEdgeType`],
//! [`PowerBiNodeKind`], and [`PowerBiGraphIndexResult`] used by the Power BI
//! indexer and CozoDB persistence layer.
//!
//! These models are separate from [`crate::models::powerbi`], which contains
//! intermediate entity types populated during extraction, and from both
//! code-symbol and backlog-specific models.  The `pbi_` prefix on edge type
//! strings prevents collisions with code and backlog edges in shared traversal
//! queries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The kind of a Power BI graph node.
///
/// Covers every entity type that the Power BI indexer can persist as a
/// first-class graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerBiNodeKind {
    /// A Power BI report (`.Report/` folder or `report.json`).
    Report,
    /// A page within a report.
    Page,
    /// A visual element on a report page.
    Visual,
    /// A semantic model / dataset (`.SemanticModel/` folder, `model.bim`).
    SemanticModel,
    /// A table within a semantic model.
    Table,
    /// A column within a table.
    Column,
    /// A DAX measure within a table.
    Measure,
    /// A top-level expression or parameter query.
    Expression,
    /// A relationship between two tables.
    Relationship,
    /// A data source referenced by the semantic model.
    DataSource,
    /// A partition binding a table to a physical load definition.
    Partition,
}

impl PowerBiNodeKind {
    /// Return the canonical `snake_case` string used in CozoDB and the API.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Report => "report",
            Self::Page => "page",
            Self::Visual => "visual",
            Self::SemanticModel => "semantic_model",
            Self::Table => "table",
            Self::Column => "column",
            Self::Measure => "measure",
            Self::Expression => "expression",
            Self::Relationship => "relationship",
            Self::DataSource => "data_source",
            Self::Partition => "partition",
        }
    }
}

impl std::fmt::Display for PowerBiNodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single Power BI entity node in the graph.
///
/// Keyed by `id` in the `powerbi_node` CozoDB relation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiNode {
    /// Stable synthetic ID derived from the workspace-relative path and entity name.
    pub id: String,

    /// Entity display name (report title, page name, table name, etc.).
    pub name: String,

    /// Entity kind (report, page, visual, table, etc.).
    pub kind: PowerBiNodeKind,

    /// Workspace-relative path to the source file for this entity.
    pub file_path: String,

    /// Registry source path this node belongs to.
    pub source_path: String,

    /// SHA-256 hash of the source file at index time, for change detection.
    pub content_hash: String,

    /// Timestamp of last indexing.
    pub ingested_at: DateTime<Utc>,
}

/// A directed relationship between two Power BI graph nodes.
///
/// Stored in the `powerbi_edge` CozoDB relation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiEdge {
    /// The originating node identifier.
    pub from_id: String,

    /// The target node identifier.
    pub to_id: String,

    /// Relationship kind.
    pub edge_type: PowerBiEdgeType,

    /// Registry source path this edge belongs to.
    pub source_path: String,
}

/// The kind of relationship between two Power BI graph nodes.
///
/// Values are stored in CozoDB with a `pbi_` prefix so they remain distinct
/// from code-symbol and backlog edge types in shared traversal queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerBiEdgeType {
    /// Parent entity contains child entity.
    ///
    /// Covers: report→page, page→visual, semantic_model→table,
    /// table→column, table→measure.
    Contains,
    /// A visual or measure uses a specific field (column or measure).
    UsesField,
    /// A report depends on a semantic model for its data.
    DependsOnModel,
    /// An entity (page, visual) belongs to a specific report.
    BelongsToReport,
    /// A relationship between two tables (source → target direction).
    RelatesToTable,
}

impl PowerBiEdgeType {
    /// Return the canonical string stored in the `powerbi_edge` CozoDB relation.
    ///
    /// Values use the `pbi_` namespace prefix to avoid collisions with code and
    /// backlog edge type names in shared graph traversal queries.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Contains => "pbi_contains",
            Self::UsesField => "pbi_uses_field",
            Self::DependsOnModel => "pbi_depends_on_model",
            Self::BelongsToReport => "pbi_belongs_to_report",
            Self::RelatesToTable => "pbi_relates_to_table",
        }
    }
}

impl std::fmt::Display for PowerBiEdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Aggregated result from a Power BI graph indexer run.
#[derive(Debug, Default)]
pub struct PowerBiGraphIndexResult {
    /// Number of graph nodes written (new or updated).
    pub nodes_written: usize,

    /// Number of graph edges written (new or updated).
    pub edges_written: usize,

    /// Number of graph nodes removed by the deletion sweep.
    pub nodes_removed: usize,
}
