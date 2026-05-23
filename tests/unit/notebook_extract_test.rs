//! Unit tests for notebook extraction helpers (063.003-T, 063.002-T).

use std::fs;
use std::path::PathBuf;

use engram::services::notebook_extract::extract_notebook;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("notebooks")
        .join(name)
}

fn load_fixture(name: &str) -> String {
    fs::read_to_string(fixture_path(name)).expect("read notebook fixture")
}

/// S-NBX-01: Summary extraction preserves markdown and code cell ordinals.
#[test]
fn extract_notebook_builds_summary_and_stable_cell_ordinals() {
    let notebook = extract_notebook(
        load_fixture("python_markdown.ipynb").as_str(),
        "notebooks/python_markdown.ipynb",
    )
    .expect("fixture should parse");

    assert_eq!(notebook.summary.default_language, "python");
    assert_eq!(notebook.summary.total_cells, 2);
    assert_eq!(notebook.summary.indexed_cell_count, 2);
    assert_eq!(notebook.cells.len(), 2);

    assert_eq!(notebook.cells[0].chunk_id, "cell-0001");
    assert_eq!(notebook.cells[0].chunk_index, 1);
    assert_eq!(notebook.cells[0].record_kind, "notebook_markdown_cell");
    assert_eq!(notebook.cells[0].language, "markdown");
    assert!(notebook.cells[0].content.contains("Notebook intro."));

    assert_eq!(notebook.cells[1].chunk_id, "cell-0002");
    assert_eq!(notebook.cells[1].chunk_index, 2);
    assert_eq!(notebook.cells[1].record_kind, "notebook_code_cell");
    assert_eq!(notebook.cells[1].language, "python");
    assert!(
        notebook.cells[1]
            .content
            .contains("print('hello from code')")
    );
    assert!(
        !notebook.cells[1].content.contains("hello from output"),
        "outputs must not appear in extracted notebook cell content"
    );
}

/// S-NBX-02: Magic and metadata precedence resolve the expected code language.
#[test]
fn extract_notebook_applies_language_precedence() {
    let sql = extract_notebook(
        load_fixture("sql_magic.ipynb").as_str(),
        "notebooks/sql_magic.ipynb",
    )
    .expect("sql fixture should parse");
    let scala = extract_notebook(
        load_fixture("scala_magic.ipynb").as_str(),
        "notebooks/scala_magic.ipynb",
    )
    .expect("scala fixture should parse");
    let sparkr = extract_notebook(
        load_fixture("sparkr_magic.ipynb").as_str(),
        "notebooks/sparkr_magic.ipynb",
    )
    .expect("sparkr fixture should parse");
    let metadata = extract_notebook(
        load_fixture("metadata_fallback.ipynb").as_str(),
        "notebooks/metadata_fallback.ipynb",
    )
    .expect("metadata fixture should parse");

    assert_eq!(sql.cells[0].language, "sql");
    assert_eq!(scala.cells[0].language, "scala");
    assert_eq!(sparkr.cells[0].language, "sparkr");
    assert_eq!(
        metadata.cells[0].language, "python",
        "language_info.name must win over kernelspec.language"
    );

    let unknown = extract_notebook(
        r#"{
          "cells": [
            {
              "cell_type": "code",
              "metadata": {},
              "outputs": [],
              "source": ["value <- 1\n"]
            }
          ],
          "metadata": {},
          "nbformat": 4,
          "nbformat_minor": 5
        }"#,
        "notebooks/unknown.ipynb",
    )
    .expect("unknown notebook should still parse");

    assert_eq!(unknown.cells[0].language, "unknown");
}

/// S-NBX-03: Malformed notebook JSON returns `None` without panicking.
#[test]
fn extract_notebook_returns_none_for_invalid_json() {
    let notebook = extract_notebook("{ this is not valid json", "notebooks/bad.ipynb");
    assert!(
        notebook.is_none(),
        "malformed notebook must be skipped cleanly"
    );
}
