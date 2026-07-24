//! Notebook extraction helpers for `.ipynb` content sources.

use crate::models::lineage::{LineageAuthorityContext, LineageEdgeCandidate};
use crate::models::notebook::{
    ExtractedNotebook, NotebookCellRecord, NotebookDocument, NotebookMetadata, NotebookSummary,
};
use crate::services::parsing::{
    extract_python_lineage, extract_sql_lineage, resolve_cell_candidates,
};

const MAGIC_SQL_CELL: &str = "%%sql";
const MAGIC_SQL_LINE: &str = "%sql";
const MAGIC_SCALA_CELL: &str = "%%scala";
const MAGIC_SPARKR_CELL: &str = "%%sparkr";
const MAGIC_PYTHON_CELL: &str = "%%python";

/// Recognized code-cell magic tokens, longest cell-magic form first so a
/// `%%sql` cell-magic is never mis-detected as the `%sql` line-magic. There is
/// **no** `%%spark` magic (AR-14).
const CODE_CELL_MAGICS: [&str; 5] = [
    MAGIC_SQL_CELL,
    MAGIC_SCALA_CELL,
    MAGIC_SPARKR_CELL,
    MAGIC_PYTHON_CELL,
    MAGIC_SQL_LINE,
];

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
                    raw_parse_source: None,
                });
            }
            "code" => {
                code_cells += 1;
                let language = resolve_code_language(trimmed, &document.metadata);
                let raw_parse_source = lineage_parse_source(trimmed, &language);
                extracted_cells.push(NotebookCellRecord {
                    chunk_id,
                    chunk_index,
                    record_kind: "notebook_code_cell".to_string(),
                    language: language.clone(),
                    content: format!("Language: {language}. {trimmed}"),
                    raw_parse_source,
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

/// Return the leading code-cell magic token (`%%sql`, `%sql`, `%%scala`,
/// `%%sparkr`, `%%python`) if `trimmed` begins with one, else `None`.
///
/// The token must sit at the head of the first line and be terminated by
/// whitespace or end-of-line so `%sqlfoo` is not treated as `%sql`.
fn leading_magic_token(trimmed: &str) -> Option<&'static str> {
    let first_line = trimmed.lines().next()?.trim_end();
    CODE_CELL_MAGICS.into_iter().find(|magic| {
        first_line == *magic
            || first_line
                .strip_prefix(*magic)
                .is_some_and(|rest| rest.starts_with(char::is_whitespace))
    })
}

/// Strip a recognized leading `magic` token (and the remainder of its line) from
/// `trimmed`, returning the trimmed cell body.
fn strip_leading_magic(trimmed: &str, magic: &str) -> String {
    let remainder = trimmed.strip_prefix(magic).unwrap_or(trimmed);
    remainder.trim().to_string()
}

/// Compute the non-persisted raw parse source handed to the Spark-lineage
/// extractors for one code cell (095-F, Unit U4a).
///
/// Returns the author-written source with the `Language: {lang}. ` retrieval
/// wrapper **never** applied and the leading cell magic stripped, or `None` when
/// the cell is not routable for lineage: a `%sql` line-magic cell (excluded from
/// v1 — AR-11), a `%%scala`/`%%sparkr` cell, or any non-PySpark/Spark-SQL
/// language. A `None` result fails closed — the router emits no lineage for the
/// cell (AR-10).
fn lineage_parse_source(trimmed: &str, language: &str) -> Option<String> {
    let magic = leading_magic_token(trimmed);
    match (language, magic) {
        // `%%sql` cell-magic routes to the U3 SQL extractor; strip the magic. A
        // `%sql` line-magic cell (AR-11) and a kernel-default SQL cell with no
        // `%%sql` magic both fail closed via the wildcard — only `%%sql` cell
        // magic routes to U3.
        ("sql", Some(MAGIC_SQL_CELL)) => Some(strip_leading_magic(trimmed, MAGIC_SQL_CELL)),
        // A `%%python` cell magic routes to the U2b PySpark resolver.
        ("python", Some(MAGIC_PYTHON_CELL)) => {
            Some(strip_leading_magic(trimmed, MAGIC_PYTHON_CELL))
        }
        // A plain Python code cell carries no cell magic.
        ("python", None) => Some(trimmed.to_string()),
        // `%sql` line-magic (AR-11), `%%scala`, `%%sparkr`, and any other
        // language/magic combination are not routable in v1 — fail closed.
        _ => None,
    }
}

/// Route each notebook cell to its Spark-lineage extractor and collect the
/// directional edge candidates (095-F, Unit U4a).
///
/// Python cells route to the U2b single-cell dataflow resolver and `%%sql`
/// cell-magic cells route to the U3 Spark-SQL extractor, each binding dataset
/// identities through `authority_ctx` (fail-closed on any unresolved authority).
/// A cell whose `raw_parse_source` is `None` — unrecoverable, a `%sql`
/// line-magic cell (AR-11), or a non-routable language — contributes no lineage
/// (AR-10). This performs no persistence; the collected candidates are handed to
/// the U4 write path (`095.015-T`).
#[must_use]
pub fn route_notebook_lineage(
    cells: &[NotebookCellRecord],
    authority_ctx: &LineageAuthorityContext,
) -> Vec<LineageEdgeCandidate> {
    cells
        .iter()
        .flat_map(|cell| route_cell_lineage(cell, authority_ctx))
        .collect()
}

/// Route a single cell to the appropriate extractor, failing closed to no
/// candidates whenever the raw parse source is absent or extraction errors.
fn route_cell_lineage(
    cell: &NotebookCellRecord,
    authority_ctx: &LineageAuthorityContext,
) -> Vec<LineageEdgeCandidate> {
    let Some(parse_source) = cell.raw_parse_source.as_deref() else {
        return Vec::new();
    };

    match cell.language.as_str() {
        "sql" => extract_sql_lineage(parse_source, authority_ctx).unwrap_or_default(),
        "python" => extract_python_lineage(parse_source, authority_ctx)
            .map(|events| resolve_cell_candidates(&events))
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_notebook, route_notebook_lineage};
    use crate::models::lineage::LineageAuthorityContext;
    use crate::models::notebook::NotebookCellRecord;
    use std::collections::BTreeMap;

    /// A trusted-authority context that resolves the `main` catalog and the
    /// `s3://bucket` storage authority, so resolvable literals bind to edges.
    fn trusted_ctx() -> LineageAuthorityContext {
        let mut catalogs = BTreeMap::new();
        catalogs.insert("main".to_owned(), "metastore-prod".to_owned());
        LineageAuthorityContext::new(catalogs, vec!["s3://bucket".to_owned()])
    }

    #[test]
    fn routes_sql_cell_magic_and_pyspark_cell_with_byte_exact_parse_source() {
        // Cell 1 (`%%sql` cell-magic) → U3; cell 2 (PySpark) → U2b. Both derive
        // `main.sales.summary` from `main.sales.orders`.
        let json = r#"{
            "cells": [
                {"cell_type": "code", "source": "%%sql\nCREATE TABLE main.sales.summary AS SELECT * FROM main.sales.orders"},
                {"cell_type": "code", "source": "df = spark.table(\"main.sales.orders\")\ndf.write.saveAsTable(\"main.sales.summary\")"}
            ],
            "metadata": {"language_info": {"name": "python"}}
        }"#;

        let extracted = extract_notebook(json, "nb.ipynb").expect("notebook parses");
        assert_eq!(extracted.cells.len(), 2);

        // Byte-exact raw parse source: no `Language: {lang}. ` wrapper and the
        // leading `%%sql` cell-magic stripped; chunk_index preserved.
        assert_eq!(extracted.cells[0].language, "sql");
        assert_eq!(extracted.cells[0].chunk_index, 1);
        assert_eq!(
            extracted.cells[0].raw_parse_source.as_deref(),
            Some("CREATE TABLE main.sales.summary AS SELECT * FROM main.sales.orders")
        );
        // The persisted content still carries the retrieval wrapper + magic.
        assert!(
            extracted.cells[0]
                .content
                .starts_with("Language: sql. %%sql")
        );

        assert_eq!(extracted.cells[1].language, "python");
        assert_eq!(extracted.cells[1].chunk_index, 2);
        assert_eq!(
            extracted.cells[1].raw_parse_source.as_deref(),
            Some(
                "df = spark.table(\"main.sales.orders\")\ndf.write.saveAsTable(\"main.sales.summary\")"
            )
        );

        let candidates = route_notebook_lineage(&extracted.cells, &trusted_ctx());
        assert_eq!(
            candidates.len(),
            2,
            "both cells route to one candidate each"
        );
        for candidate in &candidates {
            assert_eq!(candidate.target.name, "main.sales.summary");
            assert_eq!(candidate.sources.len(), 1);
            assert_eq!(candidate.sources[0].name, "main.sales.orders");
        }
    }

    #[test]
    fn sql_line_magic_cell_is_excluded_from_routing() {
        // `%sql` LINE-magic is EXCLUDED from v1 (AR-11): raw_parse_source is None
        // and the cell never reaches the U3 SQL extractor.
        let json = r#"{
            "cells": [
                {"cell_type": "code", "source": "%sql\nCREATE TABLE main.sales.summary AS SELECT * FROM main.sales.orders"}
            ],
            "metadata": {"language_info": {"name": "python"}}
        }"#;

        let extracted = extract_notebook(json, "nb.ipynb").expect("notebook parses");
        assert_eq!(extracted.cells.len(), 1);
        assert_eq!(extracted.cells[0].language, "sql");
        assert_eq!(
            extracted.cells[0].raw_parse_source, None,
            "%sql line-magic cell is not routable"
        );

        let candidates = route_notebook_lineage(&extracted.cells, &trusted_ctx());
        assert!(
            candidates.is_empty(),
            "a %sql line-magic cell must not be routed to U3"
        );
    }

    #[test]
    fn unrecoverable_raw_parse_source_emits_no_lineage() {
        // Fail-closed (AR-10): a cell whose raw parse source could not be
        // recovered (None) emits no lineage even when its content would
        // otherwise resolve to an edge.
        let cell = NotebookCellRecord {
            chunk_id: "cell-0001".to_owned(),
            chunk_index: 1,
            record_kind: "notebook_code_cell".to_owned(),
            language: "sql".to_owned(),
            content:
                "Language: sql. CREATE TABLE main.sales.summary AS SELECT * FROM main.sales.orders"
                    .to_owned(),
            raw_parse_source: None,
        };

        let candidates = route_notebook_lineage(std::slice::from_ref(&cell), &trusted_ctx());
        assert!(
            candidates.is_empty(),
            "None raw_parse_source must fail closed"
        );
    }
}
