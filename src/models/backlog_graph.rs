//! Backlog graph data models for workspace requirement tracking.
//!
//! Provides [`BacklogNode`], [`BacklogEdge`], [`BacklogEdgeType`],
//! [`BacklogContentRecord`], and [`BacklogIndexResult`] used by the
//! backlog indexer and CozoDB persistence layer.
//!
//! These models are separate from [`crate::models::backlog`], which
//! contains SpecKit-specific types used by hydration/dehydration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single backlog artifact node (feature, task, subtask, etc.).
///
/// Keyed by `id` in the `backlog_node` CozoDB relation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklogNode {
    /// Artifact identifier from YAML frontmatter (e.g. `001-F`, `001.001-T`).
    pub id: String,

    /// Human-readable title from frontmatter.
    pub title: String,

    /// Artifact kind (e.g. `feature`, `task`, `subtask`, `deliberation`, `shipment`).
    pub kind: String,

    /// Workflow status (e.g. `queued`, `active`, `done`, `blocked`).
    pub status: String,

    /// Labels from frontmatter, stored as a comma-separated string in CozoDB.
    #[serde(default)]
    pub labels: Vec<String>,

    /// Workspace-relative path to the markdown file.
    pub file_path: String,

    /// SHA-256 hash of the file content at index time.
    pub content_hash: String,

    /// Registry source path this node belongs to.
    pub source_path: String,

    /// Timestamp of last indexing.
    pub ingested_at: DateTime<Utc>,
}

/// Relationship between two backlog nodes.
///
/// Stored in the `backlog_edge` CozoDB relation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklogEdge {
    /// The originating artifact identifier.
    pub from_id: String,

    /// The target artifact identifier.
    pub to_id: String,

    /// Relationship kind.
    pub edge_type: BacklogEdgeType,

    /// Registry source path this edge belongs to.
    pub source_path: String,
}

/// The kind of relationship between two backlog nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BacklogEdgeType {
    /// Parent artifact contains child artifact (hierarchy).
    ParentOf,
    /// Source artifact depends on target before it can start.
    DependsOn,
    /// Source artifact references target artifact.
    References,
}

impl BacklogEdgeType {
    /// Return the canonical snake_case string for this edge type.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParentOf => "parent_of",
            Self::DependsOn => "depends_on",
            Self::References => "references",
        }
    }
}

impl std::fmt::Display for BacklogEdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A content record for a backlog file stored in the dedicated
/// `backlog_content_record` CozoDB relation.
///
/// Separate from [`crate::models::ContentRecord`] (which writes to the
/// generic `content_record` relation) to prevent key collisions when
/// backlog paths overlap `docs/` or other source paths already indexed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklogContentRecord {
    /// Workspace-relative file path (primary key in the relation).
    pub file_path: String,

    /// Content type — always `"backlog"` for backlog content records.
    pub content_type: String,

    /// SHA-256 hash of the file content.
    pub content_hash: String,

    /// Full markdown text of the file body (after the frontmatter block).
    pub content: String,

    /// Registry source path this record belongs to.
    pub source_path: String,

    /// Timestamp of last ingestion.
    pub ingested_at: DateTime<Utc>,
}

/// Aggregated result from a backlog indexer run over a content source.
#[derive(Debug, Default)]
pub struct BacklogIndexResult {
    /// Nodes produced or updated in this run.
    pub nodes: Vec<BacklogNode>,

    /// Edges produced or updated in this run.
    pub edges: Vec<BacklogEdge>,

    /// Content records produced or updated in this run.
    pub records: Vec<BacklogContentRecord>,

    /// Number of files that were newly ingested or changed.
    pub ingested: usize,

    /// Number of files that were skipped because content was unchanged.
    pub unchanged: usize,

    /// Number of files that were removed from the index (deletion sweep).
    pub removed: usize,
}
