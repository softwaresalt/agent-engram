//! Jupyter notebook ingestion models.
//!
//! Provides raw notebook deserialization types plus the extracted summary and
//! per-cell content shapes used by notebook indexing.

use serde::{Deserialize, Serialize};

/// A deserialized Jupyter notebook document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
pub struct NotebookDocument {
    /// Notebook cells in source order.
    #[serde(default)]
    pub cells: Vec<NotebookCell>,

    /// Top-level notebook metadata.
    #[serde(default)]
    pub metadata: NotebookMetadata,
}

/// A single Jupyter notebook cell.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
pub struct NotebookCell {
    /// Notebook cell type such as `markdown` or `code`.
    #[serde(default)]
    pub cell_type: String,

    /// Cell-local metadata payload.
    #[serde(default)]
    pub metadata: serde_json::Value,

    /// Author-written cell source, stored either as a string or line array.
    #[serde(default)]
    pub source: NotebookSource,
}

/// The Jupyter notebook source representation.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum NotebookSource {
    /// Source stored as a line array.
    Lines(Vec<String>),
    /// Source stored as a single string.
    Text(String),
}

impl Default for NotebookSource {
    fn default() -> Self {
        Self::Lines(Vec::new())
    }
}

impl NotebookSource {
    /// Render the notebook source as a single text block.
    #[must_use]
    pub fn to_text(&self) -> String {
        match self {
            Self::Lines(lines) => lines.join(""),
            Self::Text(text) => text.clone(),
        }
    }
}

/// Top-level notebook metadata relevant to language resolution.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
pub struct NotebookMetadata {
    /// Kernel metadata.
    #[serde(default)]
    pub kernelspec: Option<NotebookKernelSpec>,

    /// Language metadata.
    #[serde(default)]
    pub language_info: Option<NotebookLanguageInfo>,
}

/// Notebook kernel metadata.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
pub struct NotebookKernelSpec {
    /// Kernel language such as `python`.
    #[serde(default)]
    pub language: Option<String>,
}

/// Notebook language metadata.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
pub struct NotebookLanguageInfo {
    /// Declared notebook language such as `python`.
    #[serde(default)]
    pub name: Option<String>,
}

/// Extracted notebook summary used to build the file-level content record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotebookSummary {
    /// Optional notebook title inferred from the first markdown heading.
    pub title: Option<String>,

    /// Default language inferred from notebook metadata.
    pub default_language: String,

    /// Total cells present in the notebook payload.
    pub total_cells: usize,

    /// Number of cells emitted into content records.
    pub indexed_cell_count: usize,

    /// Number of markdown cells emitted into content records.
    pub markdown_cells: usize,

    /// Number of code cells emitted into content records.
    pub code_cells: usize,
}

/// Extracted author-written notebook cell content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotebookCellRecord {
    /// Stable chunk identifier derived from the notebook cell ordinal.
    pub chunk_id: String,

    /// One-based cell ordinal within the notebook.
    pub chunk_index: u32,

    /// Retrieval record kind such as `notebook_markdown_cell`.
    pub record_kind: String,

    /// Resolved cell language.
    pub language: String,

    /// Searchable notebook cell text excluding outputs and execution state.
    pub content: String,
}

/// Extracted notebook content ready for content-record persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedNotebook {
    /// File-level notebook summary.
    pub summary: NotebookSummary,

    /// Author-written per-cell retrieval records.
    pub cells: Vec<NotebookCellRecord>,
}

/// Aggregated result from a notebook indexer run over one content source.
#[derive(Debug, Default, Clone)]
pub struct NotebookIndexResult {
    /// Number of notebook files newly ingested or updated due to content change.
    pub ingested: usize,

    /// Number of notebook files skipped because content hash was unchanged.
    pub unchanged: usize,

    /// Number of notebook files removed from the index during deletion sweep.
    pub removed: usize,

    /// Total notebook files scanned in the source path.
    pub total_files: usize,
}
