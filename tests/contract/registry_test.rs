//! Contract tests for registry loading and validation (T016).
//!
//! Validates scenarios: S001, S004, S005, S006, S007, S009, S014.

use std::fs;
use tempfile::TempDir;

use engram::services::registry::{load_registry, parse_registry_yaml, validate_sources};

/// S001: Valid registry with 3 sources loads and validates successfully.
#[test]
fn valid_three_source_registry_loads() {
    let dir = TempDir::new().unwrap();
    let ws = dir.path();
    fs::create_dir_all(ws.join("src")).unwrap();
    fs::create_dir_all(ws.join("tests")).unwrap();
    fs::create_dir_all(ws.join("docs")).unwrap();
    let engram = ws.join(".engram");
    fs::create_dir_all(&engram).unwrap();
    let yaml = "sources:\n  - type: code\n    language: rust\n    path: src\n  - type: tests\n    language: rust\n    path: tests\n  - type: docs\n    language: markdown\n    path: docs\n";
    fs::write(engram.join("registry.yaml"), yaml).unwrap();

    let config = load_registry(&engram.join("registry.yaml"))
        .expect("load should succeed")
        .expect("config should be Some");

    assert_eq!(config.sources.len(), 3);
    assert_eq!(config.sources[0].content_type, "code");
    assert_eq!(config.sources[1].content_type, "tests");
    assert_eq!(config.sources[2].content_type, "docs");
}

/// S004: Registry entry with missing path gets status Missing.
#[test]
fn missing_path_gets_missing_status() {
    let dir = TempDir::new().unwrap();
    let ws = dir.path();
    let yaml = "sources:\n  - type: code\n    path: nonexistent\n";
    let mut config = parse_registry_yaml(yaml).unwrap();

    let active = validate_sources(&mut config, ws).unwrap();
    assert_eq!(active, 0);
    assert_eq!(
        config.sources[0].status,
        engram::models::registry::ContentSourceStatus::Missing
    );
}

/// S005: Empty sources list falls back gracefully.
#[test]
fn empty_sources_accepted() {
    let yaml = "sources: []\n";
    let config = parse_registry_yaml(yaml).unwrap();
    assert!(config.sources.is_empty());
}

/// S006: No registry file returns None (legacy fallback).
#[test]
fn no_registry_file_returns_none() {
    let dir = TempDir::new().unwrap();
    let registry_path = dir.path().join("registry.yaml");
    let result = load_registry(&registry_path).unwrap();
    assert!(result.is_none());
}

/// S007: Duplicate paths are rejected.
#[test]
fn duplicate_paths_rejected() {
    let dir = TempDir::new().unwrap();
    let ws = dir.path();
    fs::create_dir_all(ws.join("src")).unwrap();
    let yaml = "sources:\n  - type: code\n    path: src\n  - type: tests\n    path: src\n";
    let mut config = parse_registry_yaml(yaml).unwrap();

    let active = validate_sources(&mut config, ws).unwrap();
    // First is Active, second is Error (duplicate)
    assert_eq!(active, 1);
    assert_eq!(
        config.sources[0].status,
        engram::models::registry::ContentSourceStatus::Active
    );
    assert_eq!(
        config.sources[1].status,
        engram::models::registry::ContentSourceStatus::Error
    );
}

/// S009: Path traversal outside workspace root is rejected.
#[test]
fn path_traversal_rejected() {
    let dir = TempDir::new().unwrap();
    let ws = dir.path();
    let yaml = "sources:\n  - type: code\n    path: ../../etc\n";
    let mut config = parse_registry_yaml(yaml).unwrap();

    let active = validate_sources(&mut config, ws).unwrap();
    assert_eq!(active, 0);
    // Path with '..' that doesn't exist should be Error (traversal detected)
    // or Missing (path simply doesn't exist). Both are non-Active.
    assert_ne!(
        config.sources[0].status,
        engram::models::registry::ContentSourceStatus::Active,
        "Path traversal must not be Active"
    );
}

/// S014: Built-in content type accepted without warning.
#[test]
fn built_in_type_accepted() {
    let yaml = "sources:\n  - type: code\n    language: rust\n    path: src\n";
    let config = parse_registry_yaml(yaml).unwrap();
    assert_eq!(config.sources[0].content_type, "code");
}

/// Validation with active source returns count 1.
#[test]
fn active_source_counted() {
    let dir = TempDir::new().unwrap();
    let ws = dir.path();
    fs::create_dir_all(ws.join("src")).unwrap();
    let yaml = "sources:\n  - type: code\n    path: src\n";
    let mut config = parse_registry_yaml(yaml).unwrap();

    let active = validate_sources(&mut config, ws).unwrap();
    assert_eq!(active, 1);
    assert_eq!(
        config.sources[0].status,
        engram::models::registry::ContentSourceStatus::Active
    );
}
