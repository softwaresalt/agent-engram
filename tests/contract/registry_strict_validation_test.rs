//! Contract tests for `validate_sources_strict` (029-F WS-4 strict validation).
//!
//! Verifies the contract of [`validate_sources_strict`]:
//! - missing non-optional source → returns `Err` with remediation hint
//! - known renamed path → returns `Err` with migration suggestion
//! - optional missing source → returns `Ok` (no error emitted)

use std::fs;

use engram::models::registry::{ContentSource, ContentSourceStatus, RegistryConfig};
use engram::services::registry::validate_sources_strict;

/// Build a minimal [`RegistryConfig`] with a single source.
fn config_with_source(path: &str, optional: bool) -> RegistryConfig {
    RegistryConfig {
        sources: vec![ContentSource {
            content_type: "code".to_owned(),
            language: Some("rust".to_owned()),
            path: path.to_owned(),
            pattern: None,
            optional,
            status: ContentSourceStatus::Unknown,
        }],
        ..RegistryConfig::default()
    }
}

/// Missing non-optional source must return an error — callers must know
/// the registry is incomplete and needs operator attention.
///
/// Red phase: panics at `todo!()` in `validate_sources_strict`.
#[test]
fn strict_missing_non_optional_source_returns_error() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let mut cfg = config_with_source("src/does_not_exist", false);
    let result = validate_sources_strict(&mut cfg, workspace.path());

    assert!(
        result.is_err(),
        "strict validation must return Err for missing non-optional source"
    );
}

/// The error message for a missing source must contain a remediation hint so
/// operators know what action to take.
///
/// Red phase: panics at `todo!()`.
#[test]
fn strict_missing_source_error_contains_remediation_hint() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let mut cfg = config_with_source("src/does_not_exist", false);
    let err = validate_sources_strict(&mut cfg, workspace.path()).expect_err("should be an error");

    let msg = err.to_string();
    assert!(
        msg.contains("remediat") || msg.contains("hint") || msg.contains("missing"),
        "error message should contain remediation guidance, got: {msg}"
    );
}

/// A path matching a known rename pattern (e.g. `docs/` → `documentation/`)
/// must produce an error with a migration suggestion, not just a generic
/// "path not found" error.
///
/// Red phase: panics at `todo!()`.
#[test]
fn strict_known_renamed_path_returns_migration_suggestion() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    // Create the renamed path so the detection logic can compare
    fs::create_dir_all(workspace.path().join("documentation")).expect("create documentation");

    // "docs" is a known rename source → "documentation"
    let mut cfg = config_with_source("docs", false);
    let result = validate_sources_strict(&mut cfg, workspace.path());

    assert!(
        result.is_err(),
        "renamed path should still be a strict error"
    );
    let msg = result.expect_err("already checked").to_string();
    assert!(
        msg.contains("documentation") || msg.contains("migrat") || msg.contains("rename"),
        "error for renamed path should suggest the new location, got: {msg}"
    );
}

/// An optional missing source must NOT cause an error — it is intentionally
/// absent (e.g. a docs directory that may not exist in all checkouts).
///
/// Red phase: panics at `todo!()`.
#[test]
fn strict_optional_missing_source_is_silently_skipped() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let mut cfg = config_with_source("src/does_not_exist", true);
    let result = validate_sources_strict(&mut cfg, workspace.path());

    assert!(
        result.is_ok(),
        "optional missing source must not be treated as an error, got: {result:?}"
    );
}

/// Unit: `ContentSource` with serde default for `optional` round-trips correctly.
#[test]
fn content_source_optional_field_defaults_to_false() {
    let yaml = r"
type: code
path: src
";
    let source: ContentSource = serde_yaml::from_str(yaml).expect("should deserialise");
    assert!(
        !source.optional,
        "optional should default to false when absent from YAML"
    );
}

/// Unit: `ContentSource` with explicit `optional: true` round-trips correctly.
#[test]
fn content_source_optional_field_round_trips_true() {
    let yaml = r"
type: docs
path: docs
optional: true
";
    let source: ContentSource = serde_yaml::from_str(yaml).expect("should deserialise");
    assert!(source.optional, "optional: true should deserialise as true");
}
