//! Unit tests for PBIP TMDL extraction across a `definition/` directory (062.003-T).
//!
//! Verifies that the new pbip TMDL extractor walks a `definition/` directory,
//! invokes the `powerbi-tmdl-parser` crate via the existing
//! `extract_tmdl_semantic_model` adapter on each file, and merges the per-file
//! semantic-model fragments into one canonical `PowerBiSemanticModel`.
//!
//! Tests: S-PTX-01, S-PTX-02, S-PTX-03, S-PTX-04, S-PTX-05

use std::fs;

use engram::services::pbip_tmdl::extract_semantic_model_from_definition;
use tempfile::TempDir;

fn write(path: &std::path::Path, content: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent dir");
    fs::write(path, content).expect("write fixture file");
}

/// S-PTX-01: Returns `None` for a missing or empty `definition/` directory.
#[test]
fn extract_semantic_model_returns_none_for_missing_directory() {
    let temp = TempDir::new().expect("tempdir");
    let definition = temp.path().join("Sales.SemanticModel").join("definition");
    assert!(
        extract_semantic_model_from_definition(&definition).is_none(),
        "missing definition/ should yield None"
    );
}

/// S-PTX-02: Returns `None` when the `definition/` directory exists but holds no TMDL files.
#[test]
fn extract_semantic_model_returns_none_for_no_tmdl_files() {
    let temp = TempDir::new().expect("tempdir");
    let definition = temp.path().join("Sales.SemanticModel").join("definition");
    fs::create_dir_all(&definition).expect("create definition dir");
    fs::write(definition.join("readme.txt"), "not tmdl").expect("write non-tmdl file");

    assert!(
        extract_semantic_model_from_definition(&definition).is_none(),
        "definition/ without TMDL files should yield None"
    );
}

/// S-PTX-03: A multi-file fixture mirroring the PBIP `definition/` layout
/// merges into one canonical semantic model.
///
/// Confirms that `model.tmdl`'s explicit `model` declaration sets the model
/// name, and that tables and relationships from sibling files are aggregated
/// into a single returned `PowerBiSemanticModel`.
#[test]
fn extract_semantic_model_merges_multi_file_definition() {
    let temp = TempDir::new().expect("tempdir");
    let definition = temp.path().join("Sales.SemanticModel").join("definition");

    write(
        &definition.join("model.tmdl"),
        "model Sales Dataset\n\nref table Sales\nref relationship SalesToProducts\n",
    );
    write(
        &definition.join("tables").join("Sales.tmdl"),
        "table Sales\n  column Amount\n    dataType: double\n",
    );
    write(
        &definition.join("relationships.tmdl"),
        "relationship SalesToProducts\n  fromColumn: Sales.ProductID\n  toColumn: Products.ID\n",
    );
    write(
        &definition.join("expressions.tmdl"),
        "expression SynapseDatabase = \"ILSOS_EDW\" meta [IsParameterQuery=true]\n",
    );

    let model = extract_semantic_model_from_definition(&definition)
        .expect("definition/ with TMDL files should produce a semantic model");

    assert_eq!(model.name, "Sales Dataset");
    assert_eq!(model.tables.len(), 1);
    assert_eq!(model.tables[0].name, "Sales");
    assert_eq!(model.relationships.len(), 1);
    assert_eq!(model.relationships[0].from_table, "Sales");
    assert_eq!(model.relationships[0].to_column, "ID");
    assert_eq!(model.expressions.len(), 1);
    assert_eq!(model.expressions[0].name, "SynapseDatabase");
}

/// S-PTX-04: All entities anchor against one canonical model ID derived from
/// the `definition/` directory, regardless of which file they were declared in.
#[test]
fn extract_semantic_model_uses_canonical_definition_id() {
    let temp = TempDir::new().expect("tempdir");
    let definition = temp.path().join("Sales.SemanticModel").join("definition");

    write(&definition.join("model.tmdl"), "model Sales\n");
    write(
        &definition.join("tables").join("Sales.tmdl"),
        "table Sales\n  column Amount\n    dataType: double\n",
    );

    let model = extract_semantic_model_from_definition(&definition)
        .expect("definition/ should produce a model");

    assert!(
        !model.id.is_empty(),
        "merged model must carry a canonical synthetic ID"
    );
    assert!(
        !model.tables[0].id.is_empty(),
        "merged table must carry a synthetic ID"
    );
}

/// S-PTX-05: Duplicate declarations across files (e.g. `model.tmdl` ref + a
/// per-table file) are deduped by ID so the merged model contains each entity once.
#[test]
fn extract_semantic_model_dedupes_duplicate_entity_ids() {
    let temp = TempDir::new().expect("tempdir");
    let definition = temp.path().join("Sales.SemanticModel").join("definition");

    // model.tmdl ref + per-table file describe the same table.
    write(
        &definition.join("model.tmdl"),
        "model Sales\n\ntable Sales\n",
    );
    write(
        &definition.join("tables").join("Sales.tmdl"),
        "table Sales\n  column Amount\n    dataType: double\n",
    );

    let model = extract_semantic_model_from_definition(&definition)
        .expect("definition/ should produce a model");

    assert_eq!(
        model.tables.len(),
        1,
        "duplicate `Sales` table across files should be deduplicated by synthetic ID"
    );
    assert_eq!(model.tables[0].name, "Sales");
    assert_eq!(
        model.tables[0].columns.len(),
        1,
        "merged table should still expose its column from the table file"
    );
}
