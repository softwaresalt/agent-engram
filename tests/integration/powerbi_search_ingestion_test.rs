//! Integration tests for Power BI search ingestion (061.004-T).
//!
//! Validates the pure helper functions exported by [`powerbi_indexer`]:
//! hash-based change detection, deletion sweep, file collection, and
//! entity-summary extraction.  These tests run without a `CozoDB` instance,
//! mirroring the precedent set by `backlog_indexer_test.rs`.
//!
//! Tests: S-PIN-01 through S-PIN-08

use std::fs;
use tempfile::TempDir;

use engram::services::powerbi_indexer::{
    collect_powerbi_files, compute_deleted_paths, compute_file_hash, extract_entity_summaries,
};

// ── Hash helpers ──────────────────────────────────────────────────────────

/// S-PIN-01: `compute_file_hash` is deterministic for the same content.
#[test]
fn compute_file_hash_is_deterministic() {
    let content = b"{ \"tables\": [] }";
    let h1 = compute_file_hash(content);
    let h2 = compute_file_hash(content);
    assert_eq!(h1, h2, "same content must produce the same hash");
    assert_eq!(h1.len(), 64, "SHA-256 hex digest is 64 characters");
}

/// S-PIN-02: Different content produces a different hash.
#[test]
fn compute_file_hash_differs_for_changed_content() {
    let original = b"{ \"tables\": [] }";
    let changed = b"{ \"tables\": [{ \"name\": \"Sales\" }] }";
    let h1 = compute_file_hash(original);
    let h2 = compute_file_hash(changed);
    assert_ne!(h1, h2, "changed content must produce a different hash");
}

// ── Deletion sweep ────────────────────────────────────────────────────────

/// S-PIN-03: `compute_deleted_paths` detects a file that no longer exists.
#[test]
fn compute_deleted_paths_detects_removed_file() {
    let dir = TempDir::new().expect("tempdir");

    // Create one live file.
    let live_path = dir.path().join("live.json");
    fs::write(&live_path, r#"{"tables":[]}"#).expect("write live file");

    // Normalise paths to forward-slash workspace-relative form.
    let root = dir.path();
    let live_rel = live_path
        .strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let gone_rel = "gone.json".to_string();

    let known = vec![live_rel.clone(), gone_rel.clone()];
    let deleted = compute_deleted_paths(&known, root);

    assert_eq!(
        deleted.len(),
        1,
        "only gone.json should be returned as deleted"
    );
    assert!(
        deleted[0].contains("gone.json"),
        "deleted path should reference gone.json"
    );
}

/// S-PIN-04: When all known files still exist, no paths are returned as deleted.
#[test]
fn compute_deleted_paths_returns_empty_when_all_files_exist() {
    let dir = TempDir::new().expect("tempdir");
    let file = dir.path().join("model.bim");
    fs::write(&file, r#"{"model":{"tables":[]}}"#).expect("write file");

    let root = dir.path();
    let rel = file
        .strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");

    let deleted = compute_deleted_paths(&[rel], root);
    assert!(deleted.is_empty(), "no files should be reported deleted");
}

// ── File collection ───────────────────────────────────────────────────────

/// S-PIN-05: `collect_powerbi_files` discovers `.json` and `.bim` files.
#[test]
fn collect_powerbi_files_finds_json_and_bim() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("report.json"), "{}").expect("write json");
    fs::write(dir.path().join("model.bim"), "{}").expect("write bim");
    fs::write(dir.path().join("ignore.md"), "# doc").expect("write md");

    let files = collect_powerbi_files(dir.path());
    assert_eq!(files.len(), 2, "should find exactly 2 Power BI files");

    let names: Vec<_> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap())
        .collect();
    assert!(names.contains(&"report.json"), "report.json must be found");
    assert!(names.contains(&"model.bim"), "model.bim must be found");
    assert!(
        !names.contains(&"ignore.md"),
        "markdown files must be ignored"
    );
}

/// S-PIN-06: `collect_powerbi_files` is recursive.
#[test]
fn collect_powerbi_files_is_recursive() {
    let dir = TempDir::new().expect("tempdir");
    let sub = dir.path().join("subfolder");
    fs::create_dir_all(&sub).expect("create subfolder");
    fs::write(sub.join("deep.json"), "{}").expect("write nested json");
    fs::write(dir.path().join("top.json"), "{}").expect("write top-level json");

    let files = collect_powerbi_files(dir.path());
    assert_eq!(files.len(), 2, "should find files in subfolders too");
}

// ── Entity summary extraction ─────────────────────────────────────────────

/// S-PIN-07: Report JSON produces entity summaries for pages and visuals.
#[test]
fn extract_entity_summaries_from_report_json() {
    let report_json = r#"{
        "displayName": "Sales Dashboard",
        "reportSections": [
            {
                "displayName": "Overview",
                "ordinal": 1,
                "visualContainers": [
                    { "visualType": "barChart" },
                    { "visualType": "lineChart" }
                ]
            }
        ]
    }"#;

    let summaries = extract_entity_summaries(report_json, "reports/sales/report.json");
    assert!(
        !summaries.is_empty(),
        "report JSON should produce at least one entity summary"
    );

    // Expect one page summary and two visual summaries.
    let page_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_page")
        .count();
    let visual_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_visual")
        .count();

    assert_eq!(page_count, 1, "should have one page summary");
    assert_eq!(visual_count, 2, "should have two visual summaries");

    let page = summaries
        .iter()
        .find(|(k, _, _, _)| k == "powerbi_page")
        .unwrap();
    assert_eq!(page.1, "Overview", "page name should be Overview");
}

/// S-PIN-08: `model.bim` JSON produces entity summaries for tables and measures.
#[test]
fn extract_entity_summaries_from_model_bim() {
    let model_json = r#"{
        "model": {
            "tables": [
                {
                    "name": "Sales",
                    "columns": [
                        { "name": "Date", "dataType": "dateTime" }
                    ],
                    "measures": [
                        { "name": "Total Sales", "expression": "SUM(Sales[Amount])" }
                    ]
                }
            ]
        }
    }"#;

    let summaries = extract_entity_summaries(model_json, "semantic/model.bim");
    assert!(
        !summaries.is_empty(),
        "model.bim JSON should produce at least one entity summary"
    );

    let table_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_table")
        .count();
    let measure_count = summaries
        .iter()
        .filter(|(kind, _, _, _)| kind == "powerbi_measure")
        .count();

    assert_eq!(table_count, 1, "should have one table summary");
    assert_eq!(measure_count, 1, "should have one measure summary");

    let table = summaries
        .iter()
        .find(|(k, _, _, _)| k == "powerbi_table")
        .unwrap();
    assert_eq!(table.1, "Sales", "table name should be Sales");
}

/// S-PIN-09: Unknown JSON that is neither report nor model returns empty summaries.
#[test]
fn extract_entity_summaries_returns_empty_for_unknown_json() {
    let unknown = r#"{"foo": "bar", "baz": 42}"#;
    let summaries = extract_entity_summaries(unknown, "unknown/file.json");
    assert!(
        summaries.is_empty(),
        "unknown JSON should produce no entity summaries"
    );
}

/// S-PIN-10: Invalid JSON returns empty summaries without panicking.
#[test]
fn extract_entity_summaries_returns_empty_for_invalid_json() {
    let bad = "{ this is not json at all !!!";
    let summaries = extract_entity_summaries(bad, "bad/file.json");
    assert!(
        summaries.is_empty(),
        "invalid JSON should produce no entity summaries"
    );
}
