//! Backlog models for SpecKit-aware rehydration.
//!
//! Provides [`BacklogFile`], [`BacklogArtifacts`], [`BacklogItem`],
//! [`ProjectManifest`], and [`BacklogRef`] — the structured JSON
//! representations of per-feature SpecKit artifacts stored in
//! `.engram/backlog-NNN.json` and `.engram/project.json`.

use serde::{Deserialize, Serialize};

/// Full text contents of all SpecKit artifacts for a feature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklogArtifacts {
    /// Full text of `spec.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,

    /// Full text of `plan.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,

    /// Full text of `tasks.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<String>,

    /// Full text of `SCENARIOS.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenarios: Option<String>,

    /// Full text of `research.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub research: Option<String>,

    /// Full text of `ANALYSIS.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis: Option<String>,

    /// Full text of `data-model.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_model: Option<String>,

    /// Full text of `quickstart.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quickstart: Option<String>,
}

/// A sub-item within a backlog file (typically a task).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklogItem {
    /// Item identifier (e.g. `"T001"`).
    pub id: String,

    /// Item short name.
    pub name: String,

    /// Item description.
    pub description: String,
}

/// A per-feature JSON file linking SpecKit artifacts.
///
/// Serialized to `.engram/backlog-NNN.json` where `NNN` matches the
/// feature directory number under `specs/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklogFile {
    /// Feature number (e.g. `"001"`).
    pub id: String,

    /// Feature short name (e.g. `"core-mcp-daemon"`).
    pub name: String,

    /// Human-readable feature title.
    pub title: String,

    /// Associated git branch name.
    pub git_branch: String,

    /// Relative path to the feature spec directory.
    pub spec_path: String,

    /// Feature description from spec.
    pub description: String,

    /// Feature status (`"draft"`, `"in-progress"`, `"complete"`).
    pub status: String,

    /// Spec status (`"draft"`, `"approved"`, `"implemented"`).
    pub spec_status: String,

    /// Full text contents of all SpecKit artifacts.
    pub artifacts: BacklogArtifacts,

    /// Sub-items (tasks) extracted from artifacts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<BacklogItem>,
}

/// Reference to a single backlog file within the project manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklogRef {
    /// Feature number.
    pub id: String,

    /// Relative path to backlog JSON file.
    pub path: String,
}

/// Project-level metadata linking to all backlog files.
///
/// Serialized to `.engram/project.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectManifest {
    /// Project name.
    pub name: String,

    /// Project description.
    pub description: String,

    /// Git remote URL (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_url: Option<String>,

    /// Default git branch (e.g. `"main"`).
    pub default_branch: String,

    /// References to each backlog JSON file.
    #[serde(default)]
    pub backlogs: Vec<BacklogRef>,
}
