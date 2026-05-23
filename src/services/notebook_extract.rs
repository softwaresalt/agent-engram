//! Notebook extraction helpers for `.ipynb` content sources.

use crate::models::notebook::{
    ExtractedNotebook, NotebookCellRecord, NotebookDocument, NotebookMetadata, NotebookSummary,
};

const MAGIC_SQL_CELL: &str = "%%sql";
const MAGIC_SQL_LINE: &str = "%sql";
const MAGIC_SCALA_CELL: &str = "%%scala";
const MAGIC_SPARKR_CELL: &str = "%%sparkr";
const MAGIC_PYTHON_CELL: &str = "%%python";

/// Parse and extract notebook summary and per-cell retrieval records.
#[must_use]
pub fn extract_notebook(json_content: &str, _file_path: &str) -> Option<ExtractedNotebook> {
    let document = serde_json::from_str::<NotebookDocument>(json_content).ok()?;
    let mut title = None;
    let mut markdown_cells = 0_usize;
    let mut code_cells = 0_usize;
    let mut extracted_cells = Vec::new();

    for (index, cell) in document.cells.iter().enumerate() {
        let source_text = cell.source.to_text();
        let trimmed = source_text.trim();
        if trimmed.is_empty() {
            continue;
        }

        let chunk_index = u32::try_from(index + 1).ok()?;
        let chunk_id = format!("cell-{chunk_index:04}");

        match cell.cell_type.as_str() {
            "markdown" => {
                if title.is_none() {
                    title = notebook_title(trimmed);
                }
                markdown_cells += 1;
                extracted_cells.push(NotebookCellRecord {
                    chunk_id,
                    chunk_index,
                    record_kind: "notebook_markdown_cell".to_string(),
                    language: "markdown".to_string(),
                    content: trimmed.to_string(),
                });
            }
            "code" => {
                code_cells += 1;
                let language = resolve_code_language(trimmed, &document.metadata);
                extracted_cells.push(NotebookCellRecord {
                    chunk_id,
                    chunk_index,
                    record_kind: "notebook_code_cell".to_string(),
                    language: language.clone(),
                    content: format!("Language: {language}. {trimmed}"),
                });
            }
            _ => {}
        }
    }

    let default_language = default_notebook_language(&document.metadata);
    let indexed_cell_count = extracted_cells.len();

    Some(ExtractedNotebook {
        summary: NotebookSummary {
            title,
            default_language,
            total_cells: document.cells.len(),
            indexed_cell_count,
            markdown_cells,
            code_cells,
        },
        cells: extracted_cells,
    })
}

fn notebook_title(markdown_text: &str) -> Option<String> {
    markdown_text.lines().find_map(|line| {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            return None;
        }

        let title = trimmed.trim_start_matches('#').trim();
        (!title.is_empty()).then(|| title.to_owned())
    })
}

fn resolve_code_language(source_text: &str, metadata: &NotebookMetadata) -> String {
    magic_language(source_text)
        .map(ToOwned::to_owned)
        .or_else(|| metadata.language_info.as_ref()?.name.clone())
        .or_else(|| metadata.kernelspec.as_ref()?.language.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

fn default_notebook_language(metadata: &NotebookMetadata) -> String {
    metadata
        .language_info
        .as_ref()
        .and_then(|info| info.name.clone())
        .or_else(|| {
            metadata
                .kernelspec
                .as_ref()
                .and_then(|kernelspec| kernelspec.language.clone())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn magic_language(source_text: &str) -> Option<&'static str> {
    let first_non_empty = source_text
        .lines()
        .map(str::trim_start)
        .find(|line| !line.is_empty())?;

    if first_non_empty.starts_with(MAGIC_SQL_CELL) || first_non_empty.starts_with(MAGIC_SQL_LINE) {
        Some("sql")
    } else if first_non_empty.starts_with(MAGIC_SCALA_CELL) {
        Some("scala")
    } else if first_non_empty.starts_with(MAGIC_SPARKR_CELL) {
        Some("sparkr")
    } else if first_non_empty.starts_with(MAGIC_PYTHON_CELL) {
        Some("python")
    } else {
        None
    }
}
