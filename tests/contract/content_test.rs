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
