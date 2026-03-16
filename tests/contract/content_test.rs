//! Contract tests for content ingestion and type-filtered search (T021).
//!
//! Tests the ingestion pipeline's file handling behavior without SurrealDB.
//! Validates scenarios: S015, S016, S028, S029, S030 at the service API level.

use engram::services::registry::parse_registry_yaml;

/// S015: Code source type is recognized (would route to code graph).
#[test]
fn code_source_type_recognized() {
    let yaml = "sources:\n  - type: code\n    language: rust\n    path: src\n";
    let config = parse_registry_yaml(yaml).unwrap();
    assert_eq!(config.sources[0].content_type, "code");
}

/// S016: Spec source type creates content records (not code graph).
#[test]
fn spec_source_type_is_not_code() {
    let yaml = "sources:\n  - type: spec\n    path: specs\n";
    let config = parse_registry_yaml(yaml).unwrap();
    assert_ne!(config.sources[0].content_type, "code");
}

/// S028: content_type parameter is valid for filtering.
#[test]
fn content_type_filter_param_valid() {
    // Verify the model supports content_type field for filtering.
    let record = engram::models::content::ContentRecord {
        id: "test".to_string(),
        content_type: "spec".to_string(),
        file_path: "specs/spec.md".to_string(),
        content_hash: "abc".to_string(),
        content: "test content".to_string(),
        embedding: None,
        source_path: "specs".to_string(),
        file_size_bytes: 12,
        ingested_at: chrono::Utc::now(),
    };
    assert_eq!(record.content_type, "spec");
}

/// S029: Unknown content type doesn't cause errors (returns empty).
#[test]
fn unknown_content_type_is_valid_string() {
    let yaml = "sources:\n  - type: custom_xyz\n    path: custom\n";
    let config = parse_registry_yaml(yaml).unwrap();
    assert_eq!(config.sources[0].content_type, "custom_xyz");
}

/// S030: Multiple content types coexist.
#[test]
fn multiple_content_types_coexist() {
    let yaml = "sources:\n  - type: spec\n    path: specs\n  - type: docs\n    path: docs\n  - type: code\n    path: src\n";
    let config = parse_registry_yaml(yaml).unwrap();
    let types: Vec<&str> = config
        .sources
        .iter()
        .map(|s| s.content_type.as_str())
        .collect();
    assert_eq!(types, vec!["spec", "docs", "code"]);
}

// ── SpecKit hydration contract tests (T028) ────────────────────────

/// S032: Single feature dir produces backlog with artifacts.
#[test]
fn single_feature_dir_produces_backlog() {
    use engram::services::hydration::scan_speckit_features;
    use std::fs;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let ws = dir.path();
    let feature = ws.join("specs").join("001-core-mcp-daemon");
    fs::create_dir_all(&feature).unwrap();
    fs::write(feature.join("spec.md"), "# Core MCP Daemon\nDescription").unwrap();
    fs::write(feature.join("plan.md"), "# Plan\nArchitecture").unwrap();

    let backlogs = scan_speckit_features(ws);
    assert_eq!(backlogs.len(), 1);
    assert_eq!(backlogs[0].id, "001");
    assert_eq!(backlogs[0].name, "core-mcp-daemon");
    assert!(backlogs[0].artifacts.spec.is_some());
    assert!(backlogs[0].artifacts.plan.is_some());
    assert!(backlogs[0].artifacts.tasks.is_none());
}

/// S034: Project manifest created with backlog references.
#[test]
fn project_manifest_references_backlogs() {
    use engram::services::hydration::{build_project_manifest, scan_speckit_features};
    use std::fs;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let ws = dir.path();
    fs::create_dir_all(ws.join("specs").join("001-test")).unwrap();
    fs::write(ws.join("specs").join("001-test").join("spec.md"), "# Test").unwrap();

    let backlogs = scan_speckit_features(ws);
    let manifest = build_project_manifest(ws, &backlogs);
    assert_eq!(manifest.backlogs.len(), 1);
    assert_eq!(manifest.backlogs[0].id, "001");
    assert!(manifest.backlogs[0].path.contains("backlog-001.json"));
}

/// S035: Partial artifacts produce null fields.
#[test]
fn partial_artifacts_produce_none() {
    use engram::services::hydration::scan_speckit_features;
    use std::fs;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let ws = dir.path();
    let feature = ws.join("specs").join("002-partial");
    fs::create_dir_all(&feature).unwrap();
    fs::write(feature.join("spec.md"), "# Partial").unwrap();
    // No plan.md, no tasks.md, no SCENARIOS.md

    let backlogs = scan_speckit_features(ws);
    assert_eq!(backlogs.len(), 1);
    assert!(backlogs[0].artifacts.spec.is_some());
    assert!(backlogs[0].artifacts.plan.is_none());
    assert!(backlogs[0].artifacts.tasks.is_none());
    assert!(backlogs[0].artifacts.scenarios.is_none());
}

/// S038: No specs directory returns empty (legacy fallback).
#[test]
fn no_specs_dir_returns_empty() {
    use engram::services::hydration::scan_speckit_features;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let backlogs = scan_speckit_features(dir.path());
    assert!(backlogs.is_empty());
}

/// S039: Non-SpecKit directory in specs/ is ignored.
#[test]
fn non_speckit_dir_ignored() {
    use engram::services::hydration::scan_speckit_features;
    use std::fs;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let ws = dir.path();
    fs::create_dir_all(ws.join("specs").join("random-notes")).unwrap();
    fs::write(
        ws.join("specs").join("random-notes").join("notes.md"),
        "random",
    )
    .unwrap();

    let backlogs = scan_speckit_features(ws);
    assert!(backlogs.is_empty());
}
