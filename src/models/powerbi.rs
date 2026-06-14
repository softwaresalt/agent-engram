//! Power BI project entity models.
//!
//! Provides intermediate entity types for JSON-backed PBIP workspaces.
//! These types are populated by the extraction layer
//! ([`crate::services::powerbi_extract`]) and consumed by the indexer
//! ([`crate::services::powerbi_indexer`]).

use serde::{Deserialize, Serialize};

/// A Power BI report extracted from a PBIP report folder.
///
/// A report corresponds to one `.Report/` folder within a PBIP project and
/// is the top-level container for pages and visuals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiReport {
    /// Stable synthetic ID derived from the workspace-relative report path.
    pub id: String,

    /// Report display name from the JSON descriptor.
    pub name: String,

    /// Workspace-relative path to the report JSON file.
    pub path: String,

    /// Pages contained in this report, in ordinal order.
    #[serde(default)]
    pub pages: Vec<PowerBiPage>,
}

/// A single page within a Power BI report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiPage {
    /// Stable synthetic ID derived from report path + page name + ordinal.
    pub id: String,

    /// Page display name.
    pub name: String,

    /// Ordinal position preserved from the Power BI JSON payload.
    pub ordinal: u32,

    /// Visual elements placed on this page.
    #[serde(default)]
    pub visuals: Vec<PowerBiVisual>,
}

/// A visual element on a Power BI report page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiVisual {
    /// Stable synthetic ID derived from page ID + visual type + index.
    pub id: String,

    /// Visual display name or title, or an index-based fallback.
    pub name: String,

    /// Visual type string (e.g. `"barChart"`, `"lineChart"`, `"card"`).
    pub visual_type: String,
}

/// A Power BI semantic model (dataset / `.SemanticModel` folder).
///
/// Corresponds to the `model.bim` file and contains the tabular data model
/// including tables, measures, expressions, columns, relationships, and data
/// sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiSemanticModel {
    /// Stable synthetic ID derived from the workspace-relative model path.
    pub id: String,

    /// Model display name.
    pub name: String,

    /// Workspace-relative path to the `model.bim` file.
    pub path: String,

    /// Tables defined in this model.
    #[serde(default)]
    pub tables: Vec<PowerBiTable>,

    /// Relationships between tables.
    #[serde(default)]
    pub relationships: Vec<PowerBiRelationship>,

    /// Top-level expressions or parameter queries defined in the model.
    #[serde(default)]
    pub expressions: Vec<PowerBiExpression>,

    /// Data sources referenced by this model.
    #[serde(default)]
    pub data_sources: Vec<PowerBiDataSource>,
}

/// A table within a Power BI semantic model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiTable {
    /// Stable synthetic ID derived from model ID + table name.
    pub id: String,

    /// Table name as declared in the model.
    pub name: String,

    /// Columns defined on this table.
    #[serde(default)]
    pub columns: Vec<PowerBiColumn>,

    /// Measures defined on this table.
    #[serde(default)]
    pub measures: Vec<PowerBiMeasure>,
}

/// A column within a Power BI table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiColumn {
    /// Stable synthetic ID derived from table ID + column name.
    pub id: String,

    /// Column name.
    pub name: String,

    /// Data type string (e.g. `"string"`, `"int64"`, `"dateTime"`).
    #[serde(default)]
    pub data_type: Option<String>,
}

/// A DAX measure within a Power BI table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiMeasure {
    /// Stable synthetic ID derived from table ID + measure name.
    pub id: String,

    /// Measure name.
    pub name: String,

    /// DAX expression (may be truncated for indexing purposes).
    #[serde(default)]
    pub expression: Option<String>,
}

/// A top-level expression or parameter query within a Power BI semantic model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiExpression {
    /// Stable synthetic ID derived from model ID + expression name.
    pub id: String,

    /// Expression name.
    pub name: String,

    /// Expression body text.
    #[serde(default)]
    pub expression: Option<String>,
}

/// A relationship between two tables in a Power BI semantic model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiRelationship {
    /// Stable synthetic ID derived from model ID + endpoint column names.
    pub id: String,

    /// Name of the source (many-side) table.
    pub from_table: String,

    /// Name of the source (many-side) column.
    pub from_column: String,

    /// Name of the target (one-side) table.
    pub to_table: String,

    /// Name of the target (one-side) column.
    pub to_column: String,
}

/// A data source referenced by a Power BI semantic model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerBiDataSource {
    /// Stable synthetic ID derived from model ID + sequential index.
    pub id: String,

    /// Data source name.
    pub name: String,

    /// Data source kind string (e.g. `"sql"`, `"sharepoint"`, `"excel"`).
    #[serde(default)]
    pub source_type: Option<String>,
}

/// Aggregated result from a Power BI indexer run over one content source.
#[derive(Debug, Default, Clone)]
pub struct PowerBiIndexResult {
    /// Number of files newly ingested or updated due to content change.
    pub ingested: usize,

    /// Number of files skipped because content hash was unchanged.
    pub unchanged: usize,

    /// Number of files removed from the index (deletion sweep).
    pub removed: usize,

    /// Total files scanned (ingested + unchanged + skipped).
    pub total_files: usize,
}
